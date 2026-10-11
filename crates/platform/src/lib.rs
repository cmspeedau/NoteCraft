//! Platform services for NoteCraft.
//!
//! This crate is the *only* place in the workspace that talks to Windows APIs
//! directly. The rest of the application depends on the small surface exposed
//! here. On non-Windows hosts a stub backend keeps the workspace buildable and
//! testable headlessly (CI runs on Linux), which is why the two backends share
//! one interface.

use std::path::PathBuf;

#[cfg(not(windows))]
mod stub_backend;
#[cfg(windows)]
mod windows_backend;

#[cfg(not(windows))]
use stub_backend as backend;
#[cfg(windows)]
use windows_backend as backend;

/// Errors raised by the platform layer while bringing up the UI.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// `GetModuleHandleW` failed while resolving the process module.
    #[error("failed to resolve the process module (Win32 error {0})")]
    ModuleHandle(u32),
    /// `RegisterClassW` failed.
    #[error("failed to register the main window class (Win32 error {0})")]
    ClassRegistration(u32),
    /// `CreateWindowExW` failed.
    #[error("failed to create the main window (Win32 error {0})")]
    WindowCreation(u32),
}

/// Everything the platform layer needs to bring up the main window.
#[derive(Debug, Clone, Default)]
pub struct StartupOptions {
    /// Files requested on the command line to open, in order.
    pub open_paths: Vec<PathBuf>,
    /// Title for the main window.
    pub title: String,
}

/// The observable result of running the application's message loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOutcome {
    /// Process exit code the shell should observe.
    pub exit_code: i32,
}

/// Create the main window and run the message loop until the user closes it.
///
/// This call blocks for the lifetime of the UI. It returns once the message
/// loop terminates, carrying the exit code to propagate to the shell.
pub fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    backend::run(options)
}

/// Whether this build can actually present a native window on the host.
///
/// Used by the app layer to decide whether starting the UI will block; on a
/// headless CI host it returns `false`.
pub const fn has_native_window() -> bool {
    cfg!(windows)
}
