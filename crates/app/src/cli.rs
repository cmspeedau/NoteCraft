//! Command-line handling for the `notepad` binary.
//!
//! The reference app opens any paths given on the command line and otherwise
//! starts with a single untitled document. We keep the parser dependency-free
//! and hand-rolled so its behavior is fully unit-testable and does not drift
//! with a third-party crate.

use std::ffi::OsString;
use std::path::PathBuf;

/// Parsed command line.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Cli {
    /// Files to open, in the order given. Empty means "start untitled".
    pub files: Vec<PathBuf>,
    /// `--help` / `-h` / `/?` were requested.
    pub show_help: bool,
    /// `--version` / `-v` was requested.
    pub show_version: bool,
}

/// Why the command line could not be parsed.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CliError {
    /// An argument began with `-` but is not a known option.
    #[error("unrecognized option: {0}")]
    UnknownOption(String),
}

/// Parse an argument iterator, skipping the program name.
///
/// `args` must include the program name as its first element, mirroring
/// `std::env::args_os`.
pub fn parse<I>(args: I) -> Result<Cli, CliError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut cli = Cli::default();
    let mut iter = args.into_iter();
    let _program = iter.next();

    let mut options_ended = false;
    for arg in iter {
        if options_ended {
            cli.files.push(PathBuf::from(arg));
            continue;
        }

        let value = arg.to_string_lossy().into_owned();
        match value.as_str() {
            "--" => options_ended = true,
            "-h" | "--help" | "-?" | "/?" | "/help" => cli.show_help = true,
            "-v" | "-V" | "--version" => cli.show_version = true,
            other if other.starts_with('-') => {
                return Err(CliError::UnknownOption(other.to_string()));
            }
            _ => cli.files.push(PathBuf::from(arg)),
        }
    }

    Ok(cli)
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
    fn no_arguments_starts_untitled() {
        let cli = parse(args(&[])).unwrap();
        assert_eq!(cli, Cli::default());
    }

    #[test]
    fn positional_paths_are_collected_in_order() {
        let cli = parse(args(&["a.txt", "b.txt"])).unwrap();
        assert_eq!(
            cli.files,
            vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")]
        );
    }

    #[test]
    fn help_flags_are_recognized() {
        for flag in ["-h", "--help", "-?", "/?", "/help"] {
            assert!(parse(args(&[flag])).unwrap().show_help, "flag {flag}");
        }
    }

    #[test]
    fn version_flags_are_recognized() {
        for flag in ["-v", "-V", "--version"] {
            assert!(parse(args(&[flag])).unwrap().show_version, "flag {flag}");
        }
    }

    #[test]
    fn double_dash_stops_option_parsing() {
        let cli = parse(args(&["--", "-weird-name.txt"])).unwrap();
        assert_eq!(cli.files, vec![PathBuf::from("-weird-name.txt")]);
    }

    #[test]
    fn unknown_option_is_rejected() {
        let err = parse(args(&["--nope"])).unwrap_err();
        assert_eq!(err, CliError::UnknownOption("--nope".into()));
    }
}
