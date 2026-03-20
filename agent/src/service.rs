//! Windows Service integration.
//!
//! Allows the agent to run as a native Windows Service via the Service Control
//! Manager (SCM). When started by the SCM, the service dispatcher takes over
//! and calls back into the normal agent entry point. When started from the
//! command line directly, this module is not used.
//!
//! ## Shutdown flow
//!
//! When the SCM sends a Stop or Shutdown signal, the event handler calls
//! `std::process::exit(0)`. This terminates the process without running the
//! async shutdown sequence (buffer drain, sink flush) in `async_main`.
//!
//! This is acceptable because:
//! 1. The SQLite local buffer is crash-safe (WAL mode). Events written to
//!    SQLite but not yet uploaded will be drained on the next startup.
//! 2. Events already in the HTTP batch buffer (up to 10 events, flushed
//!    every 500ms) may be lost. Under normal load this is at most a few
//!    seconds of data.
//! 3. The capture loop blocks on a raw socket recv() with no cancellation
//!    mechanism. A clean shutdown would require closing the socket from
//!    another thread, which is a larger refactor across all three platform
//!    capture backends. This is tracked for a future improvement.
//! 4. The existing CLI Ctrl+C path has the same behavior -- the process
//!    exits without draining. The service stop path is consistent.
//!
//! The `windows-service` crate maps SCM stop requests to this callback.

use std::ffi::OsString;
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

/// Windows Service name registered with the SCM.
/// Must match the name used in the install script's New-Service call.
pub(crate) const SERVICE_NAME: &str = "AIRanger";

/// Start the Windows Service dispatcher.
///
/// This function does not return while the service is running. It calls the
/// SCM, which in turn invokes `service_main`. Returns an error if the binary
/// was not started by the SCM (e.g. double-click or command line), in which
/// case the caller should fall through to the normal CLI entry point.
pub(crate) fn run_as_service() -> Result<(), windows_service::Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        eprintln!("[ai-ranger] Service error: {e}");
    }
}

fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                // Exit the process immediately. See the module-level doc comment
                // for why this does not attempt an async buffer drain. The SQLite
                // buffer is crash-safe; at most a few seconds of in-flight events
                // in the HTTP batch buffer are lost (same as Ctrl+C on CLI).
                std::process::exit(0);
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // Report "running" to the SCM.
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: windows_service::service::ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    // Run the normal agent logic. This blocks until the agent exits.
    // Pass false to skip CLI arg parsing since the SCM does not pass user flags.
    // The agent loads its enrollment config from disk (written during install).
    crate::main_inner(false);

    // Report "stopped" to the SCM.
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: windows_service::service::ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    Ok(())
}
