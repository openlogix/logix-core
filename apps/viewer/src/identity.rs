//! Each app's name and icon outside macOS, where the app bundle gives
//! them (`bundle.py`). The program is shared by every kind of page, so it
//! learns its app from `apps.json` beside it, by the name it was started
//! with, and its icon is `<kind>.png` (Linux) or `<kind>.ico` (Windows)
//! there.
//!
//! On Linux, a dock or task list shows an app's icon from its desktop
//! entry, which it finds by the window's app ID (Wayland) or window class
//! (X11): GTK takes both from the program's name, so the app sets that to
//! its ID and writes a desktop entry by that name, hidden from menus. The
//! window's own icon covers the rest. Without them, GNOME showed a generic
//! gear (the user's Linux test, 2026-11-23).
//!
//! On Windows, the taskbar groups windows by their app's ID (the
//! AppUserModelID), which is the program's path unless the program sets
//! one. Every app is the same `logix-viewer.exe`, so the waveforms and the
//! diagram shared one taskbar button, with the icon of whichever opened
//! first (the user's Windows test, 2026-11-24): each app sets its own ID.

// Parts serve only some systems; the tests run them all everywhere.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// An app in `apps.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct App {
    pub kind: String,
    pub name: String,
    pub id: String,
}

#[derive(Deserialize)]
struct AppsFile {
    apps: Vec<App>,
}

/// The app with this name in `apps.json`'s text.
pub fn find(apps_json: &str, name: &str) -> Option<App> {
    let file: AppsFile = serde_json::from_str(apps_json).ok()?;
    file.apps.into_iter().find(|a| a.name == name)
}

/// The app this program was started as, and its folder, from `apps.json`
/// beside the program.
pub fn this_app(name: &str) -> Option<(App, PathBuf)> {
    let folder = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let text = std::fs::read_to_string(folder.join("apps.json")).ok()?;
    Some((find(&text, name)?, folder))
}

/// The app's icon file beside the program, as `bundle.py` names it.
pub fn icon_file(folder: &Path, app: &App) -> PathBuf {
    let ext = if cfg!(windows) { "ico" } else { "png" };
    folder.join(format!("{}.{ext}", app.kind))
}

/// A desktop entry for the app: its name and icon, matched to its windows
/// by `StartupWMClass` and the file's name (the app's ID), and hidden from
/// menus, since engines start the app, not people.
pub fn desktop_entry(app: &App, program: &Path, icon: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={}\nIcon={}\nExec={} --name {}\nNoDisplay=true\nStartupWMClass={}\n",
        app.name,
        icon.display(),
        exec_quote(&program.to_string_lossy()),
        exec_quote(&app.name),
        app.id
    )
}

/// A desktop entry's quoting for one Exec argument.
fn exec_quote(arg: &str) -> String {
    let mut out = String::from("\"");
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Where the user's desktop entries go: `$XDG_DATA_HOME/applications`, or
/// `~/.local/share/applications`.
pub fn applications_folder() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| Some(dirs::home_dir()?.join(".local").join("share")))?;
    Some(data.join("applications"))
}

/// Linux: sets the program's name to the app's ID, before GTK starts, and
/// writes its desktop entry when it isn't there or differs. Returns the
/// icon file for the windows.
#[cfg(target_os = "linux")]
pub fn set_up(name: &str) -> Option<PathBuf> {
    let (app, folder) = this_app(name)?;
    gtk::glib::set_prgname(Some(app.id.as_str()));
    let icon = icon_file(&folder, &app);
    let program = folder.join("logix-viewer");
    if let Some(dir) = applications_folder() {
        let file = dir.join(format!("{}.desktop", app.id));
        let text = desktop_entry(&app, &program, &icon);
        if std::fs::read_to_string(&file).ok().as_deref() != Some(text.as_str()) {
            let written = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, &text));
            if let Err(e) = written {
                eprintln!("{name}: can't write {}: {e}", file.display());
            }
        }
    }
    icon.is_file().then_some(icon)
}

/// Windows: sets the process's app ID to the app's, before any window
/// opens, so each app has its own taskbar button. Returns the icon file
/// for the windows.
#[cfg(windows)]
pub fn set_up(name: &str) -> Option<PathBuf> {
    let (app, folder) = this_app(name)?;
    let id: Vec<u16> = app.id.encode_utf16().chain([0]).collect();
    // SAFETY: `id` is a NUL-terminated UTF-16 string that outlives the call,
    // which copies it.
    let result = unsafe {
        windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(id.as_ptr())
    };
    if result < 0 {
        eprintln!("{name}: can't set its app ID {}: error {result:#x}", app.id);
    }
    let icon = icon_file(&folder, &app);
    icon.is_file().then_some(icon)
}

/// macOS: the app bundle has the icon.
#[cfg(target_os = "macos")]
pub fn set_up(_name: &str) -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPS: &str = include_str!("../apps.json");

    #[test]
    fn every_app_is_found_by_its_name() {
        let w = find(APPS, "Logix Waveforms").unwrap();
        assert_eq!(w.kind, "waveforms");
        assert_eq!(w.id, "org.openlogix.logix-viewer.waveforms");
        assert_eq!(find(APPS, "Logix Diagrams").unwrap().kind, "diagram");
        assert_eq!(find(APPS, "Logix Viewer").unwrap().kind, "viewer");
        assert!(find(APPS, "Something Else").is_none());
    }

    #[test]
    fn the_desktop_entry_names_the_app_its_icon_and_its_windows() {
        let app = find(APPS, "Logix Waveforms").unwrap();
        let text = desktop_entry(
            &app,
            Path::new("/home/a b/logix-viewer/logix-viewer"),
            Path::new("/home/a b/logix-viewer/waveforms.png"),
        );
        assert_eq!(
            text,
            "[Desktop Entry]\nType=Application\nName=Logix Waveforms\nIcon=/home/a b/logix-viewer/waveforms.png\nExec=\"/home/a b/logix-viewer/logix-viewer\" --name \"Logix Waveforms\"\nNoDisplay=true\nStartupWMClass=org.openlogix.logix-viewer.waveforms\n"
        );
    }

    #[test]
    fn exec_arguments_escape_what_desktop_entries_reserve() {
        assert_eq!(exec_quote(r#"a"b$c`d\e"#), r#""a\"b\$c\`d\\e""#);
    }
}
