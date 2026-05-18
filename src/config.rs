use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub cert: CertConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CertConfig {
    pub path: String,
    pub common_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
}

impl Config {
    pub fn load(config_path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(config_path)
            .context("Failed to read config.toml")?;
        toml::from_str(&contents).context("Failed to parse config.toml")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: 8282,
                host: "127.0.0.1".to_string(),
                allowed_origins: vec![
                    "http://localhost".to_string(),
                    "http://localhost:3000".to_string(),
                    "http://localhost:8080".to_string(),
                ],
            },
            cert: CertConfig {
                path: "certs/printbridge.pfx".to_string(),
                common_name: "PrintBridge Local".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        }
    }
}
