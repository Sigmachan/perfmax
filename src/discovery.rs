use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

use crate::config::Config;
use crate::state::AppState;

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

/// Probe all configured providers every 15s, update AppState::provider_status.
pub async fn run(state: Arc<RwLock<AppState>>, config: Config) {
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("discovery client");

    loop {
        let providers = config.ai.providers.clone();
        let mut status: HashMap<String, (bool, Vec<String>)> = HashMap::new();

        for p in &providers {
            let url = format!("{}/models", p.endpoint);
            let mut req = client.get(&url);
            if let Some(key) = &p.api_key {
                req = req.bearer_auth(key);
            }

            match req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    let models: Vec<String> = resp
                        .json::<ModelsResponse>()
                        .await
                        .map(|m| m.data.into_iter().map(|e| e.id).collect())
                        .unwrap_or_default();
                    debug!("Provider '{}' online, {} models", p.name, models.len());
                    status.insert(p.name.clone(), (true, models));
                }
                _ => {
                    debug!("Provider '{}' offline", p.name);
                    status.insert(p.name.clone(), (false, vec![]));
                }
            }
        }

        if let Ok(mut s) = state.write() {
            s.provider_status = status;
        }

        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}
