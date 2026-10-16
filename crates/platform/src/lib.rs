//! NoteCraft shell.
//!
//! This crate owns the window and everything drawn in it: the tab strip, the
//! menu bar, the formatting toolbar, the editor surface, the status bar, and
//! the settings page. The command surface (labels, shortcuts, availability) is
//! not restated here — it is read from the `nc-commands` registry.
//!
//! The UI is built with `egui` and hosted by `eframe`, so it runs on any
//! platform eframe supports.

use std::path::PathBuf;

mod ui;

/// Errors raised by the shell while bringing up the window.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// The window could not be created or run.
    #[error("failed to run the window: {0}")]
    Window(String),
}

/// Everything the shell needs to bring up the main window.
#[derive(Debug, Clone, Default)]
pub struct StartupOptions {
    /// Files requested on the command line to open, in order.
    pub open_paths: Vec<PathBuf>,
    /// Title for the main window.
    pub title: String,
}

/// The observable result of running the application's event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOutcome {
    /// Process exit code the shell should observe.
    pub exit_code: i32,
}

/// Create the main window and run the event loop until the user closes it.
///
/// This call blocks for the lifetime of the UI. It returns once the event loop
/// terminates, carrying the exit code to propagate to the shell.
pub fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    ui::run(options)
}

/// Whether this build can present a window. It always can.
pub const fn has_native_window() -> bool {
    true
}
