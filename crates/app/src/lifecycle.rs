//! Process startup and shutdown.
//!
//! Startup: resolve the requested documents, hand control to the platform layer
//! which brings up the window and pumps messages until the user is done.
//! Shutdown: propagate the exit code and record it.

use anyhow::Result;

use crate::cli::Cli;
use nc_platform::{StartupOptions, run as platform_run};

/// Display name used in window titles and logs.
pub const APP_NAME: &str = "NoteCraft";

/// Run the application to completion. Blocks for the lifetime of the UI on a
/// host with a native window; returns immediately otherwise.
pub fn run(cli: Cli) -> Result<i32> {
    tracing::info!(
        app = APP_NAME,
        version = env!("CARGO_PKG_VERSION"),
        files = cli.files.len(),
        "startup"
    );

    let options = StartupOptions {
        open_paths: cli.files.clone(),
        title: window_title(&cli),
    };

    let outcome = platform_run(options)?;

    tracing::info!(exit_code = outcome.exit_code, "shutdown");
    Ok(outcome.exit_code)
}

/// Window title, following the usual convention: the open document's name
/// followed by the application name, or "Untitled" when nothing is open.
fn window_title(cli: &Cli) -> String {
    match cli.files.first() {
        Some(path) => format!("{} - {APP_NAME}", path.display()),
        None => format!("Untitled - {APP_NAME}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn untitled_when_no_file() {
        assert_eq!(window_title(&Cli::default()), "Untitled - NoteCraft");
    }

    #[test]
    fn uses_first_open_path() {
        let cli = Cli {
            files: vec![PathBuf::from("notes.txt"), PathBuf::from("other.txt")],
            ..Cli::default()
        };
        assert_eq!(window_title(&cli), "notes.txt - NoteCraft");
    }
}
