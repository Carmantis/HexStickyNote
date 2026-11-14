//! Saving files from the HexTime iframe
//!
//! HexTime's PDF export normally ends in `<a download>`, which never downloads
//! here: Tauri's navigation handler accepts every navigation with
//! `webkit_policy_decision_use`, overriding WebKit's own handling of downloads.
//!
//! HexTime also supports a `window.pywebview.api.save_file` bridge (its desktop
//! window's). The script below provides that bridge inside the iframe and hands
//! the file to the parent page (HexTimeView.svelte), which calls `save_pdf`.

use directories::UserDirs;
use std::path::{Path, PathBuf};
use tauri::webview::WebviewWindowBuilder;
use tauri::{App, AppHandle};
use tauri::webview::NewWindowResponse;
use tauri::Url;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::ShellExt;

const HEXTIME_BRIDGE: &str = r#"
(function () {
  if (window === window.top) return;
  let nextId = 0;
  const pending = new Map();
  window.addEventListener('message', (event) => {
    const reply = event.data;
    if (event.source !== window.parent || reply?.type !== 'hextime-save-file-result') return;
    const request = pending.get(reply.id);
    if (!request) return;
    pending.delete(reply.id);
    if (reply.error) request.reject(new Error(reply.error));
    else request.resolve(true);
  });
  window.pywebview = {
    api: {
      save_file(filename, data) {
        return new Promise((resolve, reject) => {
          const id = ++nextId;
          pending.set(id, { resolve, reject });
          window.parent.postMessage({ type: 'hextime-save-file', id, filename, data }, '*');
        });
      },
    },
  };
})();
"#;

/// Create the main window from its tauri.conf.json entry ("create": false),
/// because a script for all frames can only be set while building a window.
pub fn create_main_window(app: &App) -> tauri::Result<()> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
        .expect("tauri.conf.json defines the main window");

    let handle = app.handle().clone();
    let popup_handle = app.handle().clone();
    WebviewWindowBuilder::from_config(app, &config)?
        .initialization_script_for_all_frames(HEXTIME_BRIDGE)
        // A link in a note, an assistant answer or HexTime must not replace
        // the app with a web page: web links open in the browser instead.
        .on_navigation(move |url| follow_link(&handle, url))
        .on_new_window(move |url, _features| {
            follow_link(&popup_handle, &url);
            NewWindowResponse::Deny
        })
        .build()?;

    Ok(())
}

/// What to do when the main window is asked to load `url`
#[derive(Debug, PartialEq)]
enum LinkTarget {
    /// The app's own pages, or the HexTime server shown in the Time tab
    App,
    /// A web page: open it in the user's browser
    Browser,
    /// Anything else (file:, other schemes): ignore it
    Blocked,
}

fn link_target(url: &Url) -> LinkTarget {
    match (url.scheme(), url.host_str()) {
        ("tauri", _) | ("about", _) => LinkTarget::App,
        // tauri.localhost: the app on Windows; localhost: the dev server;
        // 127.0.0.1: HexTime (the sidecar binds only there)
        ("http" | "https", Some("tauri.localhost" | "localhost" | "127.0.0.1")) => LinkTarget::App,
        ("http" | "https", Some(_)) => LinkTarget::Browser,
        _ => LinkTarget::Blocked,
    }
}

/// Navigation handler: true lets the webview load `url`
fn follow_link(app: &AppHandle, url: &Url) -> bool {
    match link_target(url) {
        LinkTarget::App => true,
        LinkTarget::Browser => {
            #[allow(deprecated)] // tauri-plugin-opener would be a new dependency
            if let Err(e) = app.shell().open(url.as_str(), None) {
                log::warn!("Could not open {} in the browser: {}", url, e);
            }
            false
        }
        LinkTarget::Blocked => {
            log::warn!("Blocked navigation to {}", url);
            false
        }
    }
}

/// The file name part of `requested`, if it names a PDF
fn safe_pdf_name(requested: &str) -> Result<String, String> {
    let name = requested
        .rsplit(['/', '\\'])
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty() && !name.starts_with('.') && !name.contains('\0'))
        .ok_or_else(|| format!("Invalid file name: {}", requested))?;

    if !name.to_lowercase().ends_with(".pdf") {
        return Err(format!("Only PDF files can be saved: {}", name));
    }
    Ok(name.to_string())
}

/// Write to a free path in `dir`, adding " (1)" and so on rather than overwriting
fn write_unique(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, "pdf"));
    let mut counter = 0;
    loop {
        let candidate = if counter == 0 {
            name.to_string()
        } else {
            format!("{} ({}).{}", stem, counter, extension)
        };
        let path = dir.join(candidate);
        // create_new fails if the file exists, so nothing is ever overwritten
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                std::io::Write::write_all(&mut file, bytes)?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => counter += 1,
            Err(e) => return Err(e),
        }
    }
}

/// Save a PDF to the user's Downloads folder and say where it went
pub fn save_pdf(app: &AppHandle, filename: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let name = safe_pdf_name(filename)?;
    if !bytes.starts_with(b"%PDF-") {
        return Err("The file is not a PDF".to_string());
    }

    let dirs = UserDirs::new().ok_or("Failed to determine the home directory")?;
    let dir = dirs.download_dir().unwrap_or_else(|| dirs.home_dir());
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let path = write_unique(dir, &name, bytes).map_err(|e| format!("Failed to save {}: {}", name, e))?;
    log::info!("Saved {:?}", path);

    let notification = app
        .notification()
        .builder()
        .title("PDF saved")
        .body(path.display().to_string())
        .show();
    if let Err(e) = notification {
        log::warn!("Failed to show the notification: {}", e);
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_app_in_the_window_and_web_pages_out() {
        let target = |url: &str| link_target(&Url::parse(url).unwrap());
        assert_eq!(target("tauri://localhost/"), LinkTarget::App);
        assert_eq!(target("http://tauri.localhost/orb"), LinkTarget::App);
        assert_eq!(target("http://localhost:5173/"), LinkTarget::App);
        assert_eq!(target("http://127.0.0.1:47613/#/reports"), LinkTarget::App);
        assert_eq!(target("about:blank"), LinkTarget::App);
        assert_eq!(target("https://example.com/login"), LinkTarget::Browser);
        assert_eq!(target("http://127.0.0.1.evil.example/"), LinkTarget::Browser);
        assert_eq!(target("file:///etc/passwd"), LinkTarget::Blocked);
        assert_eq!(target("mailto:someone@example.com"), LinkTarget::Blocked);
    }

    #[test]
    fn keeps_only_the_name_of_a_pdf() {
        assert_eq!(safe_pdf_name("hextime-2026.pdf").unwrap(), "hextime-2026.pdf");
        assert_eq!(safe_pdf_name("../../.ssh/report.PDF").unwrap(), "report.PDF");
        assert_eq!(safe_pdf_name(r"C:\temp\report.pdf").unwrap(), "report.pdf");
        for bad in ["", "..", "dir/", ".pdf", "evil.sh", "report.pdf.exe"] {
            assert!(safe_pdf_name(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn never_overwrites_an_existing_file() {
        let dir = std::env::temp_dir().join(format!("hex-downloads-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let first = write_unique(&dir, "r.pdf", b"one").unwrap();
        let second = write_unique(&dir, "r.pdf", b"two").unwrap();

        assert_eq!(first.file_name().unwrap(), "r.pdf");
        assert_eq!(second.file_name().unwrap(), "r (1).pdf");
        assert_eq!(std::fs::read(&first).unwrap(), b"one");
        assert_eq!(std::fs::read(&second).unwrap(), b"two");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
