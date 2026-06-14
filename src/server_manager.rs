use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use reqwest::Client;
use tracing::{info, warn};

use crate::model_manager::default_model_path;
use crate::state::AppState;

pub const SERVER_PORT: u16 = 8081;

/// Find llama-server binary in common locations.
pub fn find_llama_server() -> Option<PathBuf> {
    let candidates = [
        "llama-server",
        "llama.cpp/llama-server",
        "/usr/bin/llama-server",
        "/usr/local/bin/llama-server",
        "/opt/llama.cpp/llama-server",
    ];
    for c in candidates {
        if let Ok(path) = which::which(c) {
            return Some(path);
        }
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub struct ServerManager {
    child: Arc<Mutex<Option<Child>>>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self { child: Arc::new(Mutex::new(None)) }
    }

    /// Spawn llama-server with the default model on SERVER_PORT.
    pub fn start(&self) -> anyhow::Result<()> {
        let bin = find_llama_server()
            .ok_or_else(|| anyhow::anyhow!(
                "llama-server not found — install with: sudo pacman -S llama.cpp"
            ))?;

        let model = default_model_path();
        if !model.exists() {
            anyhow::bail!("Model not found at {}", model.display());
        }

        let child = Command::new(&bin)
            .args([
                "--model", model.to_str().unwrap(),
                "--port", &SERVER_PORT.to_string(),
                "--n-gpu-layers", "999",
                "--ctx-size", "8192",
                "--host", "127.0.0.1",
            ])
            .spawn()?;

        info!("llama-server started (pid {})", child.id());
        *self.child.lock().unwrap() = Some(child);
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
        }
    }

    pub fn is_running(&self) -> bool {
        let mut guard = self.child.lock().unwrap();
        if let Some(child) = guard.as_mut() {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }
}

impl Drop for ServerManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Background task: keep llama-server alive, update AppState::local_server_online.
pub async fn run(state: Arc<RwLock<AppState>>, mgr: Arc<ServerManager>) {
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("server health client");

    let health_url = format!("http://127.0.0.1:{}/health", SERVER_PORT);

    loop {
        let online = client.get(&health_url).send().await
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        if let Ok(mut s) = state.write() {
            s.local_server_online = online;
        }

        // Auto-restart if crashed and model exists
        if !online && !mgr.is_running() && default_model_path().exists() {
            if find_llama_server().is_some() {
                warn!("llama-server not running — restarting");
                if let Err(e) = mgr.start() {
                    warn!("restart failed: {e}");
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
