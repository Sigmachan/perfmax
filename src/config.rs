use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ai: AiConfig,
    pub optimizer: OptimizerConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub endpoint: String,
    pub model: String,
    pub interval_secs: u64,
    pub max_tokens: u32,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AiConfig {
                endpoint: "http://127.0.0.1:8081/v1".to_string(),
                model: "mimo-7b".to_string(),
                interval_secs: 30,
                max_tokens: 512,
            },
            optimizer: OptimizerConfig {
                enabled: true,
                ryzenadj_path: "/usr/local/bin/ryzenadj".to_string(),
                dry_run: false,
            },
            metrics: MetricsConfig {
                interval_ms: 1000,
            },
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
