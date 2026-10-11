//! Headless platform backend.
//!
//! Used on hosts without a native window system (Linux CI, developer machines
//! that only build the headless crates). It does not block: the "window" is a
//! no-op and the process returns immediately so tests and tooling can run.

use crate::{PlatformError, RunOutcome, StartupOptions};

pub(crate) fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    tracing::warn!(
        target: "nc_platform",
        "headless backend: no native window is available on this host; \
         the UI will not be presented"
    );
    tracing::info!(
        target: "nc_platform",
        title = %options.title,
        files = ?options.open_paths,
        "startup acknowledged (headless)"
    );
    Ok(RunOutcome { exit_code: 0 })
}
