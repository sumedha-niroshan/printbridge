#![cfg(windows)]

use anyhow::Result;
use std::ffi::OsString;
use std::time::Duration;
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

    // Run the async runtime
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Start server in background
        tokio::spawn(async {
            if let Err(e) = crate::run().await {
                tracing::error!("Server error: {}", e);
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

    Ok(())
}
