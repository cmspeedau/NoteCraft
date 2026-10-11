//! The questions the reference asks before something is destroyed.
//!
//! Nothing here writes, reads, or asks anything. Each function answers one
//! question about a [`TextFile`]: *would the reference put a prompt on screen
//! here, and about what?* Answering is the caller's business — "Don't Save"
//! means closing without saving, "Save" means the ordinary save path, and
//! cancel means doing nothing.
//!
//! What is modelled is the *state* that raises each question. The wording of
//! the prompts is a `TO_CAPTURE` item (`docs/reference-matrix.md` §5), so the
//! caller puts the captured words in; the questions themselves are the ones
//! `ROADMAP.md` M4 names — unsaved changes, overwrite confirmation, explicit
//! encoding behaviour, and outside changes if the reference detects them.

use std::path::{Path, PathBuf};

use crate::encoding::{self, Encoding};
use crate::textfile::{ExternalChange, TextFile};

/// A question the reference asks before a change takes effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// The document has unsaved changes that are about to be discarded. The
    /// reference's three answers are Save, Don't Save, and Cancel.
    UnsavedChanges,
    /// Save As picked a file that is already there.
    Overwrite {
        /// The file that would be replaced.
        path: PathBuf,
    },
    /// The text holds characters the encoding it would be written in cannot
    /// represent, and the reference asks before they are replaced. The three
    /// answers are to keep the document open (cancel), to pick another
    /// encoding, or to write it anyway as `?`.
    Unrepresentable {
        /// The encoding the file would be written in.
        encoding: Encoding,
        /// The characters with no representation, in the order they appear.
        characters: Vec<char>,
    },
    /// The file changed on disk since NoteCraft read or wrote it.
    ///
    /// Whether the reference asks at all is a `TO_CAPTURE` item; this is the
    /// state the question would be asked about.
    OutsideChange {
        /// The file that changed.
        path: PathBuf,
    },
}

/// Whether closing, or replacing the document, would discard unsaved changes.
pub fn before_discarding(file: &TextFile) -> Option<Question> {
    file.is_dirty().then_some(Question::UnsavedChanges)
}

/// Whether saving would ask about characters the encoding cannot hold.
///
/// A document with no path is not asked about: the reference opens Save As
/// first, and the answer there decides the encoding.
pub fn before_saving(file: &TextFile) -> Option<Question> {
    if !file.has_path() {
        return None;
    }
    let characters = encoding::unrepresentable(&file.document().save_text(), file.encoding());
    (!characters.is_empty()).then_some(Question::Unrepresentable {
        encoding: file.encoding(),
        characters,
    })
}

/// Whether Save As would overwrite a file that is already there.
pub fn before_overwriting(path: impl AsRef<Path>) -> Option<Question> {
    let path = path.as_ref();
    path.exists().then(|| Question::Overwrite {
        path: path.to_path_buf(),
    })
}

/// Whether the file has changed on disk since NoteCraft read or wrote it.
pub fn after_outside_change(file: &TextFile) -> Option<Question> {
    match file.external_change() {
        ExternalChange::None => None,
        ExternalChange::Modified | ExternalChange::Removed => Some(Question::OutsideChange {
            path: file.path().to_path_buf(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_untouched_document_raises_no_question() {
        let file = TextFile::untitled();
        assert_eq!(before_discarding(&file), None);
        assert_eq!(before_saving(&file), None);
        assert_eq!(after_outside_change(&file), None);
    }

    #[test]
    fn an_edited_document_raises_the_unsaved_question() {
        let mut file = TextFile::untitled();
        file.insert_str("hello");
        assert_eq!(before_discarding(&file), Some(Question::UnsavedChanges));
    }
}
