mod cert;
mod config;
mod printer;
mod protocol;
mod websocket;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use tokio_rustls::rustls::{self, ServerConfig as TlsServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[cfg(windows)]
mod service;

#[tokio::main]
async fn main() -> Result<()> {
    // If running as a Windows service, hand off to service handler
    #[cfg(windows)]
    if std::env::args().any(|a| a == "--service") {
        return service::run_as_service();
    }

    run().await
}

pub async fn run() -> Result<()> {
    // Determine data directory
    let data_dir = data_dir();
    std::fs::create_dir_all(&data_dir).context("Failed to create data dir")?;

    // Load or create config
    let config_path = data_dir.join("config.toml");
    let config = if config_path.exists() {
        config::Config::load(&config_path)?
    } else {
        let default = config::Config::default();
        let toml = include_str!("../config.toml");
        std::fs::write(&config_path, toml).ok();
        default
    };

    // Setup logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.logging.level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("PrintBridge v{} starting", env!("CARGO_PKG_VERSION"));
    info!("Data directory: {:?}", data_dir);

    // TLS certificate
    let cert_paths = cert::ensure_cert(&data_dir)?;

    // Build rustls config
    let tls_config = build_tls_config(&cert_paths)?;

    let config = Arc::new(config);

    // Run WebSocket server
    websocket::run_server(config, Arc::new(tls_config)).await
}

fn build_tls_config(cert_paths: &cert::CertPaths) -> Result<TlsServerConfig> {
    let cert_pem = std::fs::read(&cert_paths.cert_pem)
        .context("Failed to read cert PEM")?;
    let key_pem = std::fs::read(&cert_paths.key_pem)
        .context("Failed to read key PEM")?;

    let cert_chain: Vec<rustls::pki_types::CertificateDer<'static>> =
        certs(&mut cert_pem.as_ref())
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse certificate PEM")?;

    let mut keys: Vec<rustls::pki_types::PrivateKeyDer<'static>> =
        pkcs8_private_keys(&mut key_pem.as_ref())
            .map(|k| k.map(rustls::pki_types::PrivateKeyDer::Pkcs8))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse private key")?;

    if keys.is_empty() {
        anyhow::bail!("No private keys found in key PEM file");
    }

    let tls_config = TlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))
        .context("Failed to build TLS config")?;

    Ok(tls_config)
}

fn data_dir() -> PathBuf {
    // Check for --data-dir argument
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--data-dir" {
            if let Some(dir) = args.get(i + 1) {
                return PathBuf::from(dir);
            }
        }
    }

    // Default: use system app data directory
    if let Some(proj) = ProjectDirs::from("com", "printbridge", "PrintBridge") {
        proj.data_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}
