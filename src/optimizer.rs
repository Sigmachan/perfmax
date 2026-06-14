use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::info;

use crate::state::AppState;

pub async fn run(state: Arc<RwLock<AppState>>, dry_run: bool) {
    loop {
        tokio::time::sleep(Duration::from_millis(300)).await;

        let commands: Vec<String> = {
            let mut s = state.write().unwrap();
            std::mem::take(&mut s.pending_commands)
        };

        for cmd in commands {
            execute(&cmd, dry_run, &state);
        }
    }
}

fn execute(cmd: &str, dry_run: bool, state: &Arc<RwLock<AppState>>) {
    if dry_run {
        let entry = format!("[DRY] {cmd}");
        info!("{entry}");
        if let Ok(mut s) = state.write() {
            s.push_history(entry);
        }
        return;
    }

    // Parse into shell tokens and prefix with sudo
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    // Commands with tee redirection need sh -c
    let output = if cmd.contains('>') {
        std::process::Command::new("sudo")
            .args(["sh", "-c", cmd])
            .output()
    } else {
        std::process::Command::new("sudo")
            .args(&parts)
            .output()
    };

    let entry = match output {
        Ok(out) if out.status.success() => {
            format!("[OK] {cmd}")
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            format!("[FAIL] {cmd} — {}", stderr.trim())
        }
        Err(e) => {
            format!("[ERR] {cmd} — {e}")
        }
    };

    info!("{entry}");
    if let Ok(mut s) = state.write() {
        s.push_history(entry);
    }
}
