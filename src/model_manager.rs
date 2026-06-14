use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use anyhow::{bail, Result};
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::state::AppState;

pub const MODEL_REPO: &str = "jedisct1/MiMo-7B-RL-GGUF";
pub const MODEL_FILE: &str = "MiMo-7B-RL-Q8_0.gguf";
const HF_BASE: &str = "https://huggingface.co";

pub fn models_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("perfmax")
        .join("models")
}

pub fn default_model_path() -> PathBuf {
    models_dir().join(MODEL_FILE)
}

pub fn model_exists() -> bool {
    default_model_path().exists()
}

/// Download MiMo-7B-RL-Q8_0.gguf from HuggingFace with live progress in AppState.
pub async fn download_model(state: Arc<RwLock<AppState>>) -> Result<PathBuf> {
    let dest = default_model_path();
    if dest.exists() {
        return Ok(dest);
    }

    std::fs::create_dir_all(models_dir())?;

    let url = format!(
        "{}/{}/resolve/main/{}",
        HF_BASE, MODEL_REPO, MODEL_FILE
    );

    info!("Downloading {} → {}", url, dest.display());

    {
        let mut s = state.write().unwrap();
        s.model_download = Some(crate::state::DownloadState {
            file: MODEL_FILE.into(),
            bytes_done: 0,
            bytes_total: 0,
            done: false,
            error: None,
        });
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;

    let resp = client.get(&url).send().await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0);

    {
        let mut s = state.write().unwrap();
        if let Some(dl) = &mut s.model_download {
            dl.bytes_total = total;
        }
    }

    let tmp = dest.with_extension("gguf.part");
    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut done: u64 = 0;

    use futures_util::StreamExt;
    let mut byte_stream = resp.bytes_stream();
    while let Some(chunk) = byte_stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;

        let mut s = state.write().unwrap();
        if let Some(dl) = &mut s.model_download {
            dl.bytes_done = done;
        }
    }
    file.flush().await?;
    drop(file);

    tokio::fs::rename(&tmp, &dest).await?;

    {
        let mut s = state.write().unwrap();
        if let Some(dl) = &mut s.model_download {
            dl.done = true;
        }
    }

    info!("Model downloaded → {}", dest.display());
    Ok(dest)
}
