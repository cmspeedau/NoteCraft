//! NC-F001 — a new document, and reopening a file.
//!
//! `docs/feature-matrix.md` names `tests/editing/new_document.rs` for this row.
//! A new document is defined by what it *would be written as* — UTF-8 with no
//! byte order mark, CRLF, empty — so its tests sit with file I/O rather than in
//! `nc-core`, whose own `Document::new` is covered by `crates/core/tests/`.
//!
//! Reopening is the other half of the row: what a new document is written as is
//! what opening it again reads back (`ROADMAP.md` M4, "New document and reopen
//! behavior").

use std::path::PathBuf;

use nc_core::LineEnding;
use nc_textproc::{Encoding, ExternalChange, TextFile};

/// A scratch directory of its own, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("notecraft-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        Self(directory)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_new_document_is_empty_utf8_crlf_and_has_no_file() {
    let file = TextFile::untitled();

    // Nothing is behind it yet, and that is not an outside change: a file that
    // was never written has not been modified or removed.
    assert!(!file.has_path());
    assert_eq!(file.path(), std::path::Path::new(""));
    assert_eq!(file.external_change(), ExternalChange::None);

    // UTF-8 without a mark (`docs/ui-reference.md`), and CRLF, which is the
    // engine's rule
    // for a document with no file to copy a style from.
    assert_eq!(file.encoding(), Encoding::Utf8);
    assert_eq!(file.encoding_label(), "UTF-8");
    assert!(!file.has_bom());
    assert_eq!(file.line_ending(), LineEnding::CrLf);

    // Empty, one line, clean, and with nothing to undo.
    assert_eq!(file.text(), "");
    assert_eq!(file.line_count(), 1, "an empty document is one empty line");
    assert!(file.is_empty());
    assert!(file.on_disk().is_none());
    assert!(!file.is_dirty());
    assert!(!file.can_undo());
    assert!(!file.can_redo());
}

#[test]
fn a_new_document_is_written_as_crlf_utf8_without_a_mark() {
    let scratch = Scratch::new("new-document");
    let path = scratch.path("untitled.txt");

    let mut file = TextFile::untitled();
    assert!(file.insert_str("one"));
    assert!(file.insert_newline());
    assert!(file.insert_str("two"));
    assert!(file.is_dirty());

    let saved = file
        .save_as(&path, Encoding::Utf8, false)
        .expect("a save as");
    assert_eq!(saved.path, path);
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"one\r\ntwo",
        "CRLF, UTF-8, and no byte order mark"
    );

    // The document now has a file, is clean, and is written as the encoding it
    // was given; the next plain save goes to the same place unchanged.
    assert!(file.has_path());
    assert!(!file.is_dirty());
    assert_eq!(file.encoding(), Encoding::Utf8);
    assert!(!file.has_bom());
    file.save().expect("a save");
    assert_eq!(std::fs::read(&path).expect("the file"), b"one\r\ntwo");
}

#[test]
fn an_untouched_new_document_saves_as_an_empty_file() {
    let scratch = Scratch::new("new-document-empty");
    let path = scratch.path("blank.txt");

    let mut file = TextFile::untitled();
    let saved = file
        .save_as(&path, Encoding::Utf8, false)
        .expect("a save as");
    assert_eq!(saved.bytes, 0);
    assert_eq!(std::fs::read(&path).expect("the file"), b"");
    assert!(!file.is_dirty());
}

#[test]
fn reopening_a_file_gives_back_what_was_written() {
    let scratch = Scratch::new("new-document-reopen");
    let path = scratch.path("note.txt");

    let mut file = TextFile::untitled();
    assert!(file.insert_str("one"));
    assert!(file.insert_newline());
    assert!(file.insert_str("two"));
    file.save_as(&path, Encoding::Utf8, false)
        .expect("a save as");

    // Reopening the same bytes is the round trip: the text, the style it was
    // written in, and the encoding all come back, and the file opens clean.
    let reopened = TextFile::open(&path).expect("the file opens");
    assert_eq!(reopened.text(), "one\ntwo");
    assert_eq!(reopened.line_ending(), LineEnding::CrLf);
    assert_eq!(reopened.encoding(), Encoding::Utf8);
    assert!(!reopened.has_bom());
    assert_eq!(reopened.line_count(), 2);
    assert!(!reopened.is_dirty());
    assert_eq!(reopened.external_change(), ExternalChange::None);

    // And saving it untouched writes those same bytes back.
    let mut reopened = reopened;
    reopened.save().expect("a save");
    assert_eq!(std::fs::read(&path).expect("the file"), b"one\r\ntwo");
}

#[test]
fn reopening_a_file_that_cannot_be_read_reports_where_it_looked() {
    let scratch = Scratch::new("new-document-missing");
    let path = scratch.path("not-there.txt");

    match TextFile::open(&path) {
        Ok(_) => panic!("a file that is not there should not open"),
        Err(error) => {
            let message = error.to_string();
            assert!(
                message.contains("could not read") && message.contains("not-there.txt"),
                "the error names the failure and the file it looked for, got {message}"
            );
        }
    }
}
