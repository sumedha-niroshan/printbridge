#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cert;
mod config;
mod gui;
mod printer;
mod protocol;
mod websocket;

#[cfg(windows)]
mod service;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{self, ServerConfig as TlsServerConfig};
use tracing::info;
use tracing_subscriber::EnvFilter;

// Global thread-safe logs buffer shared between tracing and GUI
pub static LOGS: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

// Global flags and context to wake up the GUI when another instance is launched
pub static WAKE_GUI: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static EGUI_CTX: Lazy<Mutex<Option<eframe::egui::Context>>> = Lazy::new(|| Mutex::new(None));

// Custom writer to stream tracing logs into the GUI and stdout
#[derive(Clone)]
pub struct GuiLogWriter {
    logs: Arc<Mutex<Vec<String>>>,
}

impl std::io::Write for GuiLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf).trim().to_string();
        if !s.is_empty() {
            if let Ok(mut logs) = self.logs.lock() {
                if logs.len() > 500 {
                    logs.remove(0);
                }
                logs.push(s);
            }
        }
        std::io::stdout().write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

pub struct GuiLogWriterMaker {
    logs: Arc<Mutex<Vec<String>>>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for GuiLogWriterMaker {
    type Writer = GuiLogWriter;

    fn make_writer(&self) -> Self::Writer {
        GuiLogWriter {
            logs: self.logs.clone(),
        }
    }
}

fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    // Check for command line flags
    #[cfg(windows)]
    let args: Vec<String> = std::env::args().collect();

    // Default: Run GUI (like QZ Tray — double-click opens the control panel)
    // With --service flag: Run as Windows Service (for SCM-managed background mode)
    #[cfg(windows)]
    {
        let run_service = args.contains(&"--service".to_string());
        if run_service {
            return service::run_as_service();
        }
    }

    // Run GUI mode (default)

    // Enforce single instance and handle wake-up of existing instance
    let wake_port = 28283;
    let wake_addr = format!("127.0.0.1:{}", wake_port);

    let _wake_listener = match std::net::TcpListener::bind(&wake_addr) {
        Ok(listener) => {
            // We are the first instance. Clone the listener for the background thread
            // so we can keep the original handle alive in this scope.
            let thread_listener = listener.try_clone().expect("failed to clone wake listener");
            std::thread::spawn(move || {
                for _stream in thread_listener.incoming().flatten() {
                    WAKE_GUI.store(true, std::sync::atomic::Ordering::SeqCst);
                    // Also set the SHOW_GUI flag so the window gets
                    // un-minimized through the same path as tray clicks
                    #[cfg(windows)]
                    gui::SHOW_GUI.store(true, std::sync::atomic::Ordering::SeqCst);
                    if let Ok(guard) = EGUI_CTX.lock() {
                        if let Some(ref ctx) = *guard {
                            ctx.request_repaint();
                        }
                    }
                }
            });
            listener
        }
        Err(_) => {
            // Another instance is already running. Signal it to wake up, then exit.
            if let Ok(mut stream) = std::net::TcpStream::connect(&wake_addr) {
                use std::io::Write;
                let _ = stream.write_all(b"wake\n");
            }
            return Ok(());
        }
    };

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

    // Setup logging with GUI writer and disable ANSI colors so logs in UI are clean
    let writer_maker = GuiLogWriterMaker { logs: LOGS.clone() };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer_maker)
        .with_ansi(false)
        .init();

    info!(
        "PXL Print Client v{} starting (GUI Mode)",
        env!("CARGO_PKG_VERSION")
    );
    info!("Data directory: {:?}", data_dir);

    // TLS certificate
    let cert_paths = cert::ensure_cert(&data_dir)?;

    // Build rustls config
    let tls_config = Arc::new(build_tls_config(&cert_paths)?);

    // Always start the Tokio runtime in background and launch native eframe GUI
    let rt = Arc::new(tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?);

    // Load PXL icon for window
    let icon_data = {
        let icon_bytes = include_bytes!("../icons/PXL Icon.png");
        let icon_image = image::load_from_memory_with_format(icon_bytes, image::ImageFormat::Png)
            .expect("Failed to decode PXL icon");
        let rgba = icon_image.to_rgba8();
        let (w, h) = rgba.dimensions();
        eframe::egui::IconData {
            rgba: rgba.into_raw(),
            width: w,
            height: h,
        }
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([520.0, 420.0])
            .with_min_inner_size([400.0, 320.0])
            .with_resizable(true)
            .with_icon(std::sync::Arc::new(icon_data)),
        ..Default::default()
    };

    eframe::run_native(
        "PXL Print Client",
        options,
        Box::new(move |cc| {
            Box::new(gui::PxlApp::new(
                cc,
                config_path,
                config,
                LOGS.clone(),
                rt,
                tls_config,
            ))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI: {:?}", e))
}

fn build_tls_config(cert_paths: &cert::CertPaths) -> Result<TlsServerConfig> {
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

    // Default: use system app data directory (matching service mode)
    if let Ok(path) = std::env::var("APPDATA") {
        PathBuf::from(path).join("PXL")
    } else {
        PathBuf::from("C:\\ProgramData\\PXL")
    }
}
