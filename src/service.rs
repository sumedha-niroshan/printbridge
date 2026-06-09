#![cfg(windows)]

use anyhow::{Context, Result};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls::rustls::{self, ServerConfig as TlsServerConfig};
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const SERVICE_NAME: &str = "PXL";

define_windows_service!(ffi_service_main, service_main);

pub fn run_as_service() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .map_err(|e| anyhow::anyhow!("Service dispatcher failed: {}", e))
}

fn service_main(_args: Vec<OsString>) {
    // Setup logging for service
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stdout)
        .init();

    if let Err(e) = run_service() {
        tracing::error!("Service error: {}", e);
    }
}

fn run_service() -> Result<()> {
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        move |control| -> ServiceControlHandlerResult {
            match control {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    shutdown_tx.send(()).ok();
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        },
    )?;

    // Report: running
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("PXL Print Service starting");

    // Load configuration
    let data_dir = get_service_data_dir();
    std::fs::create_dir_all(&data_dir).context("Failed to create data dir")?;

    let config_path = data_dir.join("config.toml");
    let config = if config_path.exists() {
        crate::config::Config::load(&config_path)?
    } else {
        let default = crate::config::Config::default();
        let toml = include_str!("../config.toml");
        std::fs::write(&config_path, toml).ok();
        default
    };

    // TLS certificate
    let cert_paths = crate::cert::ensure_cert(&data_dir)?;
    let tls_config = Arc::new(build_tls_config(&cert_paths)?);
    let config = Arc::new(config);

    // Run the async runtime
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Start server in background
        tokio::spawn({
            let config = config.clone();
            let tls_config = tls_config.clone();
            async move {
                match crate::websocket::run_server(config, tls_config).await {
                    Ok(_) => info!("WebSocket server stopped"),
                    Err(e) => tracing::error!("WebSocket server error: {}", e),
                }
            }
        });

        // Wait for stop signal
        tokio::task::spawn_blocking(move || {
            shutdown_rx.recv().ok();
        })
        .await
        .ok();
    });

    // Report: stopped
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("PXL Print Service stopped");
    Ok(())
}

fn get_service_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("APPDATA") {
        PathBuf::from(path).join("PXL")
    } else {
        PathBuf::from("C:\\ProgramData\\PXL")
    }
}

fn build_tls_config(cert_paths: &crate::cert::CertPaths) -> Result<TlsServerConfig> {
    let cert_pem = std::fs::read(&cert_paths.cert_pem).context("Failed to read cert PEM")?;
    let key_pem = std::fs::read(&cert_paths.key_pem).context("Failed to read key PEM")?;

    let cert_chain: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut cert_pem.as_ref())
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
