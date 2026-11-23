//! Logix Viewer: shows the pages Logix Engine writes (waveforms, diagrams)
//! in windows of their own, one running copy per kind of page and user.
//!
//! `logix-viewer --name "Logix Waveforms" --socket <name>`
//!
//! Engines connect over the local socket (a Unix socket file, or a named
//! pipe name on Windows) and send requests, one JSON object per line
//! (`protocol.rs`). Each page shows in its own window, known by the page
//! file's path; a page shown again is reloaded in place. A window belongs
//! to the connection that opened it last, and closes when that connection
//! drops. The app quits when its last window closes.
//!
//! Pages get `window.logixHost` (`host.js`): the Save dialog, writing the
//! files the engine declared for the page (its view), showing a source
//! place in a note, and opening https links.

mod identity;
mod protocol;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use interprocess::local_socket::{ListenerOptions, Stream, prelude::*};
use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder, WindowId};
use wry::http::{Response, header::CONTENT_TYPE};
use wry::{WebView, WebViewBuilder};

use protocol::{Owners, Reply, Request};

/// The page contract's functions, defined before every page's own scripts.
const HOST_JS: &str = include_str!("host.js");

/// The custom scheme pages load from.
const SCHEME: &str = "logixpage";

/// What reaches the event loop from other threads.
enum UserEvent {
    /// A request from connection `conn`; the answer goes back on `reply`.
    Request {
        conn: u64,
        request: Request,
        reply: mpsc::Sender<Reply>,
    },
    /// Connection `conn` dropped: its engine exited.
    Dropped(u64),
    /// The page in window `number` called `window.logixHost`.
    Host {
        number: u64,
        body: String,
    },
    Menu(muda::MenuEvent),
}

/// One window and what its page may do.
struct PageWindow {
    window: Window,
    webview: WebView,
    page: PathBuf,
    declared: BTreeMap<String, PathBuf>,
    loads: u64,
}

/// Pages by window number, for the custom scheme's handler.
type Pages = Arc<Mutex<BTreeMap<u64, PathBuf>>>;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };
    // Which step in from the screen's corner the app's first window opens
    // at, so two apps' first windows don't hide each other.
    let slot: f64 = arg("--slot").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let (Some(name), Some(socket)) = (arg("--name"), arg("--socket")) else {
        eprintln!("usage: logix-viewer --name <app name> --socket <socket name>");
        std::process::exit(2);
    };

    let listener = match listen(&socket) {
        Ok(Some(l)) => l,
        Ok(None) => {
            eprintln!("{name} is already running on {socket}.");
            return;
        }
        Err(e) => {
            eprintln!("{name} can't listen on {socket}: {e}");
            std::process::exit(1);
        }
    };
    eprintln!("{name} is listening on {socket}.");
    // Its name and icon outside macOS (identity.rs), before GTK starts.
    let icon = identity::set_up(&name);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    {
        let proxy = proxy.clone();
        let app = name.clone();
        std::thread::spawn(move || accept(listener, proxy, app));
    }
    let menu = app_menu(&name);
    let state_file = state_file(&name);
    let mut size = load_size(state_file.as_deref());
    let mut size_saved = std::time::Instant::now();
    {
        let proxy = proxy.clone();
        muda::MenuEvent::set_event_handler(Some(move |e| {
            let _ = proxy.send_event(UserEvent::Menu(e));
        }));
    }

    let pages: Pages = Arc::default();
    let mut windows: BTreeMap<u64, PageWindow> = BTreeMap::new();
    let mut by_id: BTreeMap<WindowId, u64> = BTreeMap::new();
    let mut owners = Owners::default();
    let mut next_number = 0u64;
    let mut focused: Option<u64> = None;

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _ = &menu;
        match event {
            Event::UserEvent(UserEvent::Request {
                conn,
                request,
                reply,
            }) => {
                let answer = match request {
                    Request::Hello { id, contract, .. } => protocol::hello(id, contract, &name),
                    Request::Open {
                        id,
                        page,
                        title,
                        declared,
                        // openSource only shows the place (host.js), so the
                        // folder isn't needed.
                        source_folder: _,
                    } => {
                        if !page.is_file() {
                            Reply::problem(
                                id,
                                format!("The page {} doesn't exist.", page.display()),
                            )
                        } else if owners.open(conn, page.clone()) {
                            next_number += 1;
                            let number = next_number;
                            let title = if title.is_empty() {
                                name.clone()
                            } else {
                                title
                            };
                            let newest = windows.values().next_back().map(|w| &w.window);
                            let placed = place(target, newest, size, slot);
                            match new_window(
                                target,
                                &proxy,
                                &pages,
                                number,
                                &page,
                                &title,
                                placed,
                                icon.as_deref(),
                            ) {
                                Ok((window, webview)) => {
                                    by_id.insert(window.id(), number);
                                    window.set_focus();
                                    windows.insert(
                                        number,
                                        PageWindow {
                                            window,
                                            webview,
                                            page,
                                            declared,
                                            loads: 0,
                                        },
                                    );
                                    Reply::ok(id, "opened")
                                }
                                Err(e) => {
                                    owners.forget(&page);
                                    Reply::problem(id, format!("{name} can't open a window: {e}"))
                                }
                            }
                        } else {
                            match windows.values_mut().find(|w| w.page == page) {
                                Some(w) => {
                                    w.declared = declared;
                                    if !title.is_empty() {
                                        w.window.set_title(&title);
                                    }
                                    reload(w, &by_id);
                                    w.window.set_minimized(false);
                                    w.window.set_focus();
                                    Reply::ok(id, "reloaded")
                                }
                                None => Reply::problem(id, "The window was closed just now."),
                            }
                        }
                    }
                    Request::Close { id, page } => {
                        let closing = owners.close(conn, page.as_ref());
                        let n = closing.len();
                        close_pages(&closing, &mut windows, &mut by_id, &pages);
                        Reply::ok(id, format!("closed {n}"))
                    }
                };
                let _ = reply.send(answer);
                if windows.is_empty() && owners.is_empty() && had_windows(next_number) {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UserEvent::Dropped(conn)) => {
                let closing = owners.close(conn, None);
                close_pages(&closing, &mut windows, &mut by_id, &pages);
                if windows.is_empty() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UserEvent::Host { number, body }) => {
                if let Some(w) = windows.get(&number)
                    && let Err(e) = host_call(w, &body)
                {
                    eprintln!("{name}: {e}");
                }
            }
            Event::UserEvent(UserEvent::Menu(e)) => {
                if e.id().as_ref() == "reload"
                    && let Some(w) = focused.and_then(|n| windows.get_mut(&n))
                {
                    reload(w, &by_id);
                }
            }
            Event::WindowEvent {
                window_id, event, ..
            } => match event {
                WindowEvent::Focused(true) => focused = by_id.get(&window_id).copied(),
                WindowEvent::Resized(physical) => {
                    // The size a person gives a window is the next window's.
                    if let Some(w) = by_id.get(&window_id).and_then(|n| windows.get(n)) {
                        let logical: LogicalSize<f64> =
                            physical.to_logical(w.window.scale_factor());
                        size = Some(logical);
                        if size_saved.elapsed().as_millis() > 300 {
                            save_size(state_file.as_deref(), logical);
                            size_saved = std::time::Instant::now();
                        }
                    }
                }
                WindowEvent::CloseRequested => {
                    if let Some(size) = size {
                        save_size(state_file.as_deref(), size);
                    }
                    if let Some(number) = by_id.get(&window_id).copied() {
                        if let Some(w) = windows.get(&number) {
                            owners.forget(&w.page.clone());
                        }
                        let page = windows.get(&number).map(|w| w.page.clone());
                        if let Some(page) = page {
                            close_pages(&[page], &mut windows, &mut by_id, &pages);
                        }
                    }
                    if windows.is_empty() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    });
}

/// Where the app remembers its window size: `~/.logix/viewer/<name>.json`
/// (each Open Logix program keeps its files in its own folder there).
fn state_file(name: &str) -> Option<PathBuf> {
    let slug = name.to_lowercase().replace(' ', "-");
    Some(
        dirs::home_dir()?
            .join(".logix")
            .join("viewer")
            .join(format!("{slug}.json")),
    )
}

/// The size a person last gave one of the app's windows, if any.
fn load_size(file: Option<&Path>) -> Option<LogicalSize<f64>> {
    let saved = file
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    match saved {
        Some(v) => match (v["width"].as_f64(), v["height"].as_f64()) {
            (Some(w), Some(h)) if w >= 300.0 && h >= 200.0 => Some(LogicalSize::new(w, h)),
            _ => None,
        },
        None => None,
    }
}

fn save_size(file: Option<&Path>, size: LogicalSize<f64>) {
    let Some(file) = file else { return };
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text = serde_json::json!({ "width": size.width.round(), "height": size.height.round() });
    let _ = write_atomically(file, format!("{text}\n").as_bytes());
}

/// Where a new window goes and how big it is. Its size: the one a person
/// last gave the app's windows, at most 80% of the screen, else half the
/// screen's width and 60% of its height. Its place: one step down and to
/// the right of the app's newest window, as Terminal's; the app's first
/// window, or one that would run off the screen, starts near the top left
/// corner, `slot` steps in.
fn place(
    target: &EventLoopWindowTarget<UserEvent>,
    newest: Option<&Window>,
    remembered: Option<LogicalSize<f64>>,
    slot: f64,
) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    let Some(m) = target.primary_monitor() else {
        return (
            LogicalPosition::new(80.0, 80.0),
            remembered.unwrap_or(LogicalSize::new(900.0, 600.0)),
        );
    };
    let scale = m.scale_factor();
    let screen: LogicalSize<f64> = m.size().to_logical(scale);
    let origin: LogicalPosition<f64> = m.position().to_logical(scale);
    let size = match remembered {
        Some(s) => LogicalSize::new(
            s.width.min(screen.width * 0.8),
            s.height.min(screen.height * 0.8),
        ),
        None => LogicalSize::new(screen.width * 0.5, screen.height * 0.6),
    };
    let step = 28.0;
    let start = LogicalPosition::new(
        origin.x + screen.width * 0.08 + slot * step * 2.0,
        origin.y + screen.height * 0.08 + slot * step * 2.0,
    );
    let next = newest
        // The window's content corner: the point with_position places, so
        // the step isn't eaten by the title bar's height.
        .and_then(|w| {
            w.inner_position()
                .ok()
                .map(|p| p.to_logical::<f64>(w.scale_factor()))
        })
        .map(|p| LogicalPosition::new(p.x + step, p.y + step));
    let at = match next {
        Some(p)
            if p.x + size.width <= origin.x + screen.width
                && p.y + size.height <= origin.y + screen.height =>
        {
            p
        }
        _ => start,
    };
    (at, size)
}

/// True once any window was opened: the app doesn't quit before its first.
fn had_windows(next_number: u64) -> bool {
    next_number > 0
}

/// Listens on the app's socket. None when another copy already answers
/// there; a socket file left by a copy that crashed is replaced.
fn listen(socket: &str) -> std::io::Result<Option<interprocess::local_socket::Listener>> {
    let bind = || {
        ListenerOptions::new()
            .name(socket_name(socket)?)
            .create_sync()
    };
    match bind() {
        Ok(l) => Ok(Some(l)),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            if Stream::connect(socket_name(socket)?).is_ok() {
                return Ok(None);
            }
            #[cfg(unix)]
            std::fs::remove_file(socket)?;
            bind().map(Some)
        }
        Err(e) => Err(e),
    }
}

/// The socket's name: a file path on macOS and Linux, a named pipe's name
/// on Windows.
pub fn socket_name(socket: &str) -> std::io::Result<interprocess::local_socket::Name<'_>> {
    #[cfg(windows)]
    {
        socket.to_ns_name::<interprocess::local_socket::GenericNamespaced>()
    }
    #[cfg(not(windows))]
    {
        socket.to_fs_name::<interprocess::local_socket::GenericFilePath>()
    }
}

/// Accepts connections, each served on a thread of its own.
fn accept(
    listener: interprocess::local_socket::Listener,
    proxy: EventLoopProxy<UserEvent>,
    app: String,
) {
    let mut next = 0u64;
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        next += 1;
        let conn = next;
        let proxy = proxy.clone();
        let app = app.clone();
        std::thread::spawn(move || {
            serve(stream, conn, &proxy, &app);
            let _ = proxy.send_event(UserEvent::Dropped(conn));
        });
    }
}

/// Reads one connection's requests until it drops, answering each.
fn serve(stream: Stream, conn: u64, proxy: &EventLoopProxy<UserEvent>, app: &str) {
    let mut send = &stream;
    for line in BufReader::new(&stream).lines() {
        let Ok(line) = line else { return };
        if line.trim().is_empty() {
            continue;
        }
        let answer = match protocol::parse(&line) {
            Err(refused) => refused,
            Ok(Request::Hello { id, contract, .. }) => protocol::hello(id, contract, app),
            Ok(request) => {
                let (tx, rx) = mpsc::channel();
                if proxy
                    .send_event(UserEvent::Request {
                        conn,
                        request,
                        reply: tx,
                    })
                    .is_err()
                {
                    return;
                }
                match rx.recv() {
                    Ok(r) => r,
                    Err(_) => return,
                }
            }
        };
        let Ok(text) = serde_json::to_string(&answer) else {
            return;
        };
        if writeln!(send, "{text}").is_err() || send.flush().is_err() {
            return;
        }
    }
}

/// The page's address in window `number`; `loads` makes each reload fresh.
fn page_url(number: u64, loads: u64) -> String {
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/{number}?load={loads}")
    } else {
        format!("{SCHEME}://localhost/{number}?load={loads}")
    }
}

#[allow(clippy::too_many_arguments)]
fn new_window(
    target: &EventLoopWindowTarget<UserEvent>,
    proxy: &EventLoopProxy<UserEvent>,
    pages: &Pages,
    number: u64,
    page: &Path,
    title: &str,
    (at, size): (LogicalPosition<f64>, LogicalSize<f64>),
    icon: Option<&Path>,
) -> Result<(Window, WebView), String> {
    pages.lock().unwrap().insert(number, page.to_path_buf());
    let builder = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size)
        .with_position(at);
    #[cfg(windows)]
    let builder = {
        use tao::platform::windows::IconExtWindows;
        builder.with_window_icon(icon.and_then(|f| tao::window::Icon::from_path(f, None).ok()))
    };
    let window = builder.build(target).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    if let Some(f) = icon {
        use gtk::prelude::GtkWindowExt;
        use tao::platform::unix::WindowExtUnix;
        let _ = window.gtk_window().set_icon_from_file(f);
    }
    #[cfg(target_os = "macos")]
    let _ = icon;
    let served = pages.clone();
    let ipc = proxy.clone();
    let builder = WebViewBuilder::new()
        .with_initialization_script(HOST_JS)
        .with_custom_protocol(SCHEME.into(), move |_id, request| {
            let number: Option<u64> = request.uri().path().trim_start_matches('/').parse().ok();
            let path = number.and_then(|n| served.lock().unwrap().get(&n).cloned());
            match path.map(std::fs::read) {
                Some(Ok(bytes)) => Response::builder()
                    .header(CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Cow::Owned(bytes))
                    .unwrap(),
                _ => Response::builder()
                    .status(404)
                    .body(Cow::Borrowed(&b"The page is gone."[..]))
                    .unwrap(),
            }
        })
        .with_ipc_handler(move |request| {
            let _ = ipc.send_event(UserEvent::Host {
                number,
                body: request.body().clone(),
            });
        })
        .with_url(page_url(number, 0));
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().ok_or("no GTK container")?;
        builder.build_gtk(vbox).map_err(|e| e.to_string())?
    };
    Ok((window, webview))
}

fn reload(w: &mut PageWindow, by_id: &BTreeMap<WindowId, u64>) {
    w.loads += 1;
    if let Some(number) = by_id.get(&w.window.id()) {
        let _ = w.webview.load_url(&page_url(*number, w.loads));
    }
}

fn close_pages(
    closing: &[PathBuf],
    windows: &mut BTreeMap<u64, PageWindow>,
    by_id: &mut BTreeMap<WindowId, u64>,
    pages: &Pages,
) {
    let numbers: Vec<u64> = windows
        .iter()
        .filter(|(_, w)| closing.contains(&w.page))
        .map(|(n, _)| *n)
        .collect();
    for n in numbers {
        if let Some(w) = windows.remove(&n) {
            by_id.remove(&w.window.id());
            pages.lock().unwrap().remove(&n);
        }
    }
}

/// One call to `window.logixHost` from the page in `w`.
fn host_call(w: &PageWindow, body: &str) -> Result<(), String> {
    let msg: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let text = |k: &str| msg[k].as_str().unwrap_or_default().to_string();
    match msg["cmd"].as_str() {
        Some("writeDeclared") => {
            let id = text("id");
            let path = w.declared.get(&id).ok_or_else(|| {
                format!("the page may not write {id:?}: the engine declared no such file")
            })?;
            write_atomically(path, text("text").as_bytes())
        }
        Some("saveAs") => {
            let bytes = if text("mime") == "image/png" {
                base64::engine::general_purpose::STANDARD
                    .decode(text("data"))
                    .map_err(|e| e.to_string())?
            } else {
                text("data").into_bytes()
            };
            match rfd::FileDialog::new()
                .set_file_name(text("name"))
                .set_parent(&w.window)
                .save_file()
            {
                Some(path) => std::fs::write(&path, bytes)
                    .map_err(|e| format!("can't save {}: {e}", path.display())),
                None => Ok(()),
            }
        }
        Some("openUrl") => {
            let url = text("url");
            if url.starts_with("https://") {
                open_with_system(url.as_ref())
            } else {
                Err(format!("only https:// links open: {url}"))
            }
        }
        other => Err(format!("unknown host call {other:?}")),
    }
}

/// Writes a file under a temporary name, then renames it into place, so a
/// reader never sees half of it.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("logix-viewer-tmp");
    std::fs::write(&tmp, bytes)
        .and_then(|_| std::fs::rename(&tmp, path))
        .map_err(|e| format!("can't save {}: {e}", path.display()))
}

/// Opens a link with the system's default app.
fn open_with_system(what: &std::ffi::OsStr) -> Result<(), String> {
    let mut cmd = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/c", "start", ""]);
        c
    } else {
        std::process::Command::new("xdg-open")
    };
    cmd.arg(what).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// The menu bar on macOS: the app's name with Quit, Edit (so copying works
/// in pages), View with Reload, and Window. Other systems get none.
fn app_menu(name: &str) -> muda::Menu {
    use muda::{Menu, MenuItem, PredefinedMenuItem as P, Submenu};
    let menu = Menu::new();
    let reload = MenuItem::with_id("reload", "Reload", true, "CmdOrCtrl+R".parse().ok());
    let app = Submenu::with_items(
        name,
        true,
        &[
            &P::about(None, None),
            &P::separator(),
            &P::hide(None),
            &P::hide_others(None),
            &P::show_all(None),
            &P::separator(),
            &P::quit(None),
        ],
    );
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[
            &P::undo(None),
            &P::redo(None),
            &P::separator(),
            &P::cut(None),
            &P::copy(None),
            &P::paste(None),
            &P::select_all(None),
        ],
    );
    let view = Submenu::with_items("View", true, &[&reload]);
    let window = Submenu::with_items(
        "Window",
        true,
        &[&P::minimize(None), &P::close_window(None)],
    );
    if let (Ok(app), Ok(edit), Ok(view), Ok(window)) = (app, edit, view, window) {
        let _ = menu.append_items(&[&app, &edit, &view, &window]);
        // macOS lists the open windows in the Window menu and in the Dock
        // icon's menu only when the app names its Window menu.
        #[cfg(target_os = "macos")]
        {
            menu.init_for_nsapp();
            window.set_as_windows_menu_for_nsapp();
        }
    }
    menu
}
