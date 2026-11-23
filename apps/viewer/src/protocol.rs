//! What engines send Logix Viewer and what it answers: one JSON object per
//! line over the app's local socket, each request with an `id` its answer
//! repeats. Plain code, no windows, so it's tested on its own.
//!
//! Requests:
//! - `hello`: `{"id":1,"type":"hello","contract":1,"client":"logix-engine 2026.11.22 (process 4412)"}`,
//!   first on every connection. Refused when the page contract the engine
//!   needs is newer than this app's.
//! - `open`: `{"id":2,"type":"open","page":"/abs/_build/main_tb.html","title":"…","declared":{"view":"/abs/main_tb.view"},"source_folder":"/abs"}`.
//!   Opens the page in a window of its own, or reloads the window that
//!   shows that page file already, and brings it to the front. The window
//!   then belongs to this connection.
//! - `close`: `{"id":3,"type":"close","page":"/abs/_build/main_tb.html"}`,
//!   or without `page` for all of this connection's windows.
//!
//! When a connection drops (its engine exits, even when killed), its
//! windows close.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The page contract this app implements: what `window.logixHost` offers.
/// Raised only when pages need something an older app doesn't have.
pub const CONTRACT: u32 = 1;

/// One request from an engine.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Hello {
        id: u64,
        contract: u32,
        #[serde(default)]
        client: String,
    },
    Open {
        id: u64,
        page: PathBuf,
        #[serde(default)]
        title: String,
        /// Files the page may write, by id, such as `view`.
        #[serde(default)]
        declared: BTreeMap<String, PathBuf>,
        /// Where the page's source files are. Accepted, not used: Logix
        /// Viewer has no editor and offers no `openSource`.
        #[serde(default)]
        source_folder: Option<PathBuf>,
    },
    Close {
        id: u64,
        #[serde(default)]
        page: Option<PathBuf>,
    },
}

#[cfg(test)]
impl Request {
    pub fn id(&self) -> u64 {
        match self {
            Request::Hello { id, .. } | Request::Open { id, .. } | Request::Close { id, .. } => *id,
        }
    }
}

/// The answer to one request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reply {
    pub id: u64,
    pub ok: bool,
    /// What happened: "opened", "reloaded", "closed 2".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<String>,
    /// Why not, in plain words for the person at the screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<u32>,
}

impl Reply {
    pub fn ok(id: u64, done: impl Into<String>) -> Reply {
        Reply {
            id,
            ok: true,
            done: Some(done.into()),
            problem: None,
            contract: None,
        }
    }
    pub fn problem(id: u64, problem: impl Into<String>) -> Reply {
        Reply {
            id,
            ok: false,
            done: None,
            problem: Some(problem.into()),
            contract: None,
        }
    }
}

/// Reads one request line. A line that isn't a request is refused with
/// the reason, answered with id 0 when it has none.
pub fn parse(line: &str) -> Result<Request, Reply> {
    serde_json::from_str(line).map_err(|e| {
        let id = serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .and_then(|v| v["id"].as_u64())
            .unwrap_or(0);
        Reply::problem(id, format!("Not a request Logix Viewer understands: {e}."))
    })
}

/// The answer to `hello`: refused when the engine's pages need a newer
/// contract than this app has.
pub fn hello(id: u64, contract: u32, app: &str) -> Reply {
    if contract > CONTRACT {
        Reply::problem(
            id,
            format!(
                "{app} is older than this page needs: close its windows so a newer one starts."
            ),
        )
    } else {
        Reply {
            contract: Some(CONTRACT),
            ..Reply::ok(id, "hello")
        }
    }
}

/// Which connection owns which window, by page file. Windows are known by
/// their page's path; the window itself lives with the event loop.
#[derive(Debug, Default)]
pub struct Owners {
    by_page: BTreeMap<PathBuf, u64>,
}

impl Owners {
    /// Records that `conn` shows `page` now. True if the page had no window
    /// yet (open one), false if one shows it already (reload it).
    pub fn open(&mut self, conn: u64, page: PathBuf) -> bool {
        self.by_page.insert(page, conn).is_none()
    }

    /// The pages `close` closes for `conn`: one page if it's that
    /// connection's, or all of them. They're forgotten.
    pub fn close(&mut self, conn: u64, page: Option<&PathBuf>) -> Vec<PathBuf> {
        let mine: Vec<PathBuf> = self
            .by_page
            .iter()
            .filter(|(p, c)| **c == conn && page.is_none_or(|want| *p == want))
            .map(|(p, _)| p.clone())
            .collect();
        for p in &mine {
            self.by_page.remove(p);
        }
        mine
    }

    /// A window the person closed.
    pub fn forget(&mut self, page: &PathBuf) {
        self.by_page.remove(page);
    }

    pub fn is_empty(&self) -> bool {
        self.by_page.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_requests_and_refuses_others() {
        let r = parse(r#"{"id":2,"type":"open","page":"/p/_build/main_tb.html","declared":{"view":"/p/main_tb.view"}}"#).unwrap();
        assert_eq!(r.id(), 2);
        let Request::Open { page, declared, .. } = r else {
            panic!()
        };
        assert_eq!(page, PathBuf::from("/p/_build/main_tb.html"));
        assert_eq!(declared["view"], PathBuf::from("/p/main_tb.view"));
        assert!(matches!(
            parse(r#"{"id":3,"type":"close"}"#).unwrap(),
            Request::Close { page: None, .. }
        ));

        let refused = parse(r#"{"id":7,"type":"delete_everything"}"#).unwrap_err();
        assert_eq!(refused.id, 7);
        assert!(!refused.ok);
        assert_eq!(parse("not json").unwrap_err().id, 0);
    }

    #[test]
    fn hello_refuses_a_newer_contract() {
        assert!(hello(1, CONTRACT, "Logix Waveforms").ok);
        let r = hello(1, CONTRACT + 1, "Logix Waveforms");
        assert!(!r.ok);
        assert!(
            r.problem
                .unwrap()
                .starts_with("Logix Waveforms is older than this page needs")
        );
    }

    #[test]
    fn windows_belong_to_the_connection_that_opened_them_last() {
        let mut o = Owners::default();
        let main = PathBuf::from("/a/_build/main_tb.html");
        let uart = PathBuf::from("/a/_build/uart_tb.html");
        assert!(o.open(1, main.clone()));
        assert!(o.open(2, uart.clone()));
        // Engine 2 shows the same page: one window, now engine 2's.
        assert!(!o.open(2, main.clone()));
        assert!(o.close(1, None).is_empty());
        assert_eq!(o.close(2, Some(&uart)), vec![uart]);
        assert_eq!(o.close(2, None), vec![main]);
        assert!(o.is_empty());
    }
}
