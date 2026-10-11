//! Structured logging setup.
//!
//! Logs go to stderr and never leave the machine (see `architecture.md` §11).
//! Verbosity is controlled by the `NOTEPAD_LOG` environment variable using the
//! standard `EnvFilter` directive syntax, defaulting to `info`.

use tracing_subscriber::EnvFilter;

/// Environment variable that overrides the default log filter.
pub const LOG_ENV: &str = "NOTEPAD_LOG";

/// Install the global tracing subscriber. Idempotent: a second call is ignored
/// because the global subscriber can only be set once per process.
pub fn init() {
    let filter = EnvFilter::try_from_env(LOG_ENV).unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
