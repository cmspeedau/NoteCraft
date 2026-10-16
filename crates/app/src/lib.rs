//! NoteCraft application crate.
//!
//! Owns command-line handling, logging setup, and the startup/shutdown
//! lifecycle. The window itself is created by `nc-platform`.

pub mod cli;
pub mod lifecycle;
pub mod logging;

use std::ffi::OsString;
use std::process::ExitCode;

/// Text shown for `--help` and on a usage error.
pub const USAGE: &str = "\
NoteCraft - a fast, modern text and Markdown editor

Usage:
    notepad [options] [file ...]

Options:
    -h, --help       Show this help and exit
    -v, --version    Show version and exit
    --               Treat every following argument as a file path

With no file arguments, NoteCraft starts with a single untitled document.
";

/// Entry point used by `main`. Reads the process arguments.
pub fn run_from_env() -> ExitCode {
    run(std::env::args_os())
}

/// Testable entry point: parse `args`, dispatch, and map the result to an
/// [`ExitCode`].
pub fn run<I>(args: I) -> ExitCode
where
    I: IntoIterator<Item = OsString>,
{
    let cli = match cli::parse(args) {
        Ok(cli) => cli,
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!();
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    if cli.show_help {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    if cli.show_version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    logging::init();

    match lifecycle::run(cli) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(err) => {
            tracing::error!(error = %err, "fatal");
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<OsString> {
        std::iter::once("notepad")
            .chain(items.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn help_exits_success() {
        assert_eq!(run(args(&["--help"])), ExitCode::SUCCESS);
    }

    #[test]
    fn unknown_option_exits_with_code_2() {
        assert_eq!(run(args(&["--bogus"])), ExitCode::from(2));
    }
}
