use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Provider {
    pub name: String,
    pub endpoint: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

impl Provider {
    pub fn local(name: &str, endpoint: &str, model: &str) -> Self {
        Self {
            name: name.into(),
            endpoint: endpoint.into(),
            model: model.into(),
            api_key: None,
        }
    }

    pub fn cloud(name: &str, endpoint: &str, model: &str, key: &str) -> Self {
        Self {
            name: name.into(),
            endpoint: endpoint.into(),
            model: model.into(),
            api_key: Some(key.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Name of the currently active provider (must match a Provider::name)
    pub active: String,
    pub providers: Vec<Provider>,
    pub interval_secs: u64,
    pub max_tokens: u32,
}

impl AiConfig {
    pub fn active_provider(&self) -> Option<&Provider> {
        self.providers.iter().find(|p| p.name == self.active)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub enabled: bool,
    pub ryzenadj_path: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ai: AiConfig,
    pub optimizer: OptimizerConfig,
    pub metrics: MetricsConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            active: "aeon (local)".into(),
            providers: vec![
                Provider::local("aeon (local)", "http://127.0.0.1:8080/v1", "aeon"),
                Provider::local("MiMo-7B (local)", "http://127.0.0.1:8081/v1", "mimo-7b"),
                Provider::local("Ollama (local)", "http://127.0.0.1:11434/v1", "llama3"),
                Provider::local("OpenClaw (local)", "http://127.0.0.1:18789/v1", "aeon"),
            ],
            interval_secs: 30,
            max_tokens: 512,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            optimizer: OptimizerConfig {
                enabled: true,
                ryzenadj_path: "/usr/local/bin/ryzenadj".into(),
                dry_run: true,
            },
            metrics: MetricsConfig { interval_ms: 1000 },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&text)?)
        } else {
            let cfg = Self::default();
            cfg.save()?;
            Ok(cfg)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("perfmax")
        .join("config.toml")
}
