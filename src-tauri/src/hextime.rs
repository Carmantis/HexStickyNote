//! HexTime sidecar
//!
//! HexTime (time tracking) is a Python/FastAPI app. It is bundled as a
//! PyInstaller binary (see sidecar/hextime/) and started on demand: the server
//! listens on a free loopback port and the HUD shows the UI it serves. The
//! process watches its stdin, so it also exits if HexStickyNote crashes.

use reqwest::Client;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

const BINARY_NAME: &str = "hextime-server";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

/// HexTime keeps language and theme in localStorage, which is tied to the
/// origin including the port; a stable port keeps those settings across runs.
const PREFERRED_PORT: u16 = 47613;

#[derive(Debug, Error)]
pub enum HexTimeError {
    #[error("HexTime is not bundled with this build (run sidecar/hextime/build.sh)")]
    NotBundled,
    #[error("Failed to start HexTime: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("HexTime stopped during startup ({0})")]
    Exited(String),
    #[error("HexTime did not start within {} seconds", STARTUP_TIMEOUT.as_secs())]
    Timeout,
}

struct Server {
    /// None when the server was already running (started by the MCP server)
    child: Option<Child>,
    url: String,
}

/// Managed state: the running HexTime server, if any
pub struct HexTime {
    server: Mutex<Option<Server>>,
    /// Port to use and to look for an already running server on; None means a
    /// private server on a random port (tests must never touch the user's data)
    preferred_port: Option<u16>,
    /// Database for a private server; None uses HexTime's own (the user's data)
    database_url: Option<String>,
}

impl Default for HexTime {
    fn default() -> Self {
        Self { server: Mutex::new(None), preferred_port: Some(PREFERRED_PORT), database_url: None }
    }
}

/// Where the sidecar binary is: HEXTIME_SERVER override, next to the app
/// executable (bundled via externalBin), or src-tauri/binaries in dev builds
pub fn sidecar_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("HEXTIME_SERVER") {
        return Some(PathBuf::from(path));
    }

    let exe_suffix = std::env::consts::EXE_SUFFIX;
    let bundled = std::env::current_exe()
        .ok()?
        .parent()?
        .join(format!("{}{}", BINARY_NAME, exe_suffix));
    if bundled.is_file() {
        return Some(bundled);
    }

    if cfg!(debug_assertions) {
        let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(format!("{}-{}{}", BINARY_NAME, env!("TARGET_TRIPLE"), exe_suffix));
        if dev.is_file() {
            return Some(dev);
        }
    }

    None
}

/// Whether a HexTime server answers at `url`
async fn is_hextime(client: &Client, url: &str) -> bool {
    client
        .get(format!("{}/api/v1/config", url))
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// The preferred port if it is free, otherwise any free port
fn free_port(preferred: Option<u16>) -> std::io::Result<u16> {
    if let Some(port) = preferred {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
        log::warn!("Port {} is busy; HexTime UI settings will not persist this run", port);
    }
    Ok(TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port())
}

fn spawn(path: &Path, port: u16, database_url: Option<&str>) -> std::io::Result<Child> {
    let mut command = Command::new(path);
    if let Some(url) = database_url {
        command.env("DATABASE_URL", url);
    }
    command
        .args(["--port", &port.to_string(), "--watch-stdin"])
        // Kept open for the server's lifetime; closing it stops the server
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        // HexTime reads an optional .env from the working directory
        .current_dir(std::env::temp_dir());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command.spawn()
}

impl HexTime {
    /// A server of its own on a random port with a throwaway database (for tests)
    pub fn isolated() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let file = std::env::temp_dir().join(format!("hextime-test-{}-{}.db", std::process::id(), n));
        let _ = std::fs::remove_file(&file);
        Self {
            server: Mutex::new(None),
            preferred_port: None,
            database_url: Some(format!("sqlite+pysqlite:///{}", file.display())),
        }
    }

    /// Start HexTime if it is not running and return the URL of its UI
    pub async fn start(&self, client: &Client) -> Result<String, HexTimeError> {
        let mut server = self.server.lock().await;

        if let Some(running) = server.as_mut() {
            let alive = match running.child.as_mut() {
                Some(child) => child.try_wait()?.is_none(),
                None => is_hextime(client, &running.url).await,
            };
            if alive {
                return Ok(running.url.clone());
            }
            log::warn!("HexTime server had exited; starting it again");
            *server = None;
        }

        // The Claude Desktop MCP server may already run HexTime on the preferred port
        if let Some(port) = self.preferred_port {
            let preferred = format!("http://127.0.0.1:{}", port);
            if is_hextime(client, &preferred).await {
                log::info!("Using the HexTime server already running at {}", preferred);
                *server = Some(Server { child: None, url: preferred.clone() });
                return Ok(preferred);
            }
        }

        let path = sidecar_path().ok_or(HexTimeError::NotBundled)?;
        let port = free_port(self.preferred_port)?;
        let url = format!("http://127.0.0.1:{}", port);

        log::info!("Starting HexTime: {:?} on port {}", path, port);
        let mut child = spawn(&path, port, self.database_url.as_deref())?;

        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                return Err(HexTimeError::Exited(status.to_string()));
            }

            if is_hextime(client, &url).await {
                break;
            }

            if started.elapsed() > STARTUP_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                return Err(HexTimeError::Timeout);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        log::info!("HexTime ready at {} after {:?}", url, started.elapsed());
        *server = Some(Server { child: Some(child), url: url.clone() });
        Ok(url)
    }

    /// Stop the server (called when the app exits)
    pub fn shutdown(&self) {
        let Ok(mut server) = self.server.try_lock() else {
            // A start is in progress; the server exits by itself once our stdin closes
            return;
        };
        let Some(mut child) = server.take().and_then(|running| running.child) else {
            // Not ours to stop
            return;
        };

        // Closing stdin lets HexTime exit on its own
        drop(child.stdin.take());

        let deadline = Instant::now() + SHUTDOWN_GRACE;
        while Instant::now() < deadline {
            if matches!(child.try_wait(), Ok(Some(_))) {
                log::info!("HexTime stopped");
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        log::warn!("HexTime did not stop in time; killing it");
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the sidecar: run sidecar/hextime/build.sh, then `cargo test -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn starts_reuses_and_stops_the_sidecar() {
        let hextime = HexTime::isolated();
        let client = Client::new();

        let url = hextime.start(&client).await.expect("sidecar starts");
        let config = client.get(format!("{}/api/v1/config", url)).send().await.unwrap();
        assert!(config.status().is_success());

        // A second start reuses the running server
        assert_eq!(hextime.start(&client).await.unwrap(), url);

        hextime.shutdown();
        let after = client
            .get(format!("{}/api/v1/config", url))
            .timeout(Duration::from_secs(1))
            .send()
            .await;
        assert!(after.is_err(), "server still answers after shutdown");
    }
}
