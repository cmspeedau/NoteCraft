//! Where NoteCraft keeps its per-user files.
//!
//! Two files live here: the session ([`crate::ui::session`], which documents are
//! open) and the settings ([`crate::ui::settings`], which is how the shell looks and
//! behaves). Both are small hand-written text files rather than something a
//! serialization crate produces: they are meant to be readable in a text editor when
//! a user reports something odd, and a format defined by `key value` lines tolerates a
//! key its reader does not know instead of failing on it.
//!
//! This module owns the *directory* because both files need it, and a configuration
//! directory involves the one piece of platform behaviour worth getting in one place:
//! where it is, and what an override means.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The directory NoteCraft keeps its files in, or `None` when there is nowhere to
/// put them.
///
/// `NOTECRAFT_CONFIG_DIR` overrides it, which is what tests and a portable install
/// use; otherwise the platform's usual per-user configuration directory, and nothing
/// when none can be found — which is not an error, just a run that keeps no files.
pub(crate) fn directory() -> Option<PathBuf> {
    if let Some(directory) = non_empty(std::env::var_os("NOTECRAFT_CONFIG_DIR")) {
        return Some(PathBuf::from(directory));
    }
    if cfg!(windows) {
        if let Some(appdata) = non_empty(std::env::var_os("APPDATA")) {
            return Some(PathBuf::from(appdata).join("NoteCraft"));
        }
    }
    if let Some(xdg) = non_empty(std::env::var_os("XDG_CONFIG_HOME")) {
        return Some(PathBuf::from(xdg).join("notecraft"));
    }
    non_empty(std::env::var_os("HOME"))
        .map(|home| PathBuf::from(home).join(".config").join("notecraft"))
}

/// One of NoteCraft's files, by name, in [`directory`].
///
/// `None` means there is nowhere for it to be: reads find nothing, writes do nothing,
/// and the application runs without persistence rather than failing.
pub(crate) fn file(name: &str) -> Option<PathBuf> {
    directory().map(|directory| directory.join(name))
}

/// Creates the directory if it is missing, which is what a write needs and a read
/// does not.
pub(crate) fn ensure_directory(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// The value, or `None` when it is unset or empty.
fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

/// An empty override is no override.
///
/// A launcher that exports `NOTECRAFT_CONFIG_DIR=` without a value would otherwise
/// put the files at the filesystem root, so an empty variable falls through to the
/// platform directory like an unset one. [`non_empty`] is what makes that true, and
/// the case is named here because nothing else in the file explains it.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_value_is_not_a_directory() {
        assert_eq!(non_empty(Some(OsString::from(""))), None);
        assert_eq!(
            non_empty(Some(OsString::from("/tmp/somewhere"))),
            Some(OsString::from("/tmp/somewhere"))
        );
    }
}
