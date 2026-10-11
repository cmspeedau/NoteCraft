//! NC-D004/NC-E009 — line endings, at the file level.
//!
//! `nc-core` decides the styles; this is the same rule seen through the file: a
//! file that is opened and saved untouched comes back byte for byte, whatever
//! mixture of terminators it held, and a file that is edited is written in the
//! style of its first terminator (or CRLF, if it had none).

use std::path::{Path, PathBuf};

use nc_core::LineEnding;
use nc_textproc::TextFile;

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

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).expect("a file to read");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bytes_at(path: &Path) -> Vec<u8> {
    std::fs::read(path).expect("the file just written")
}

#[test]
fn a_crlf_file_is_read_as_crlf_with_lf_in_the_buffer() {
    let scratch = Scratch::new("eol-crlf");
    let path = scratch.file("crlf.txt", b"one\r\ntwo\r\nthree");

    let file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.line_ending(), LineEnding::CrLf);
    assert_eq!(file.text(), "one\ntwo\nthree");
    assert_eq!(file.line_count(), 3);
    assert!(!file.has_mixed_line_endings());
}

#[test]
fn an_lf_file_stays_lf() {
    let scratch = Scratch::new("eol-lf");
    let path = scratch.file("lf.txt", b"one\ntwo\n");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.line_ending(), LineEnding::Lf);
    // An opened file starts with the caret at the beginning.
    assert!(file.insert_str("three"));
    file.save().expect("a save");

    assert_eq!(bytes_at(&path), b"threeone\ntwo\n");
}

#[test]
fn an_untouched_file_is_saved_byte_for_byte() {
    let scratch = Scratch::new("eol-untouched");
    for (name, text) in [
        ("crlf.txt", "one\r\ntwo\r\nthree"),
        ("lf.txt", "one\ntwo\n"),
        ("cr.txt", "one\rtwo"),
        ("none.txt", "one line, no terminator"),
        ("empty.txt", ""),
        ("bom-crlf.txt", "\u{FEFF}one\r\n"),
    ] {
        let path = scratch.file(name, text.as_bytes());
        let before = bytes_at(&path);

        let mut file = TextFile::open(&path).expect(name);
        assert!(!file.is_dirty(), "{name} opened dirty");
        file.save().expect("a save");

        assert_eq!(bytes_at(&path), before, "{name} was rewritten");
    }
}

#[test]
fn a_mixed_file_is_reported_as_mixed_and_saved_unchanged() {
    let scratch = Scratch::new("eol-mixed");
    let path = scratch.file("mixed.txt", b"one\r\ntwo\nthree\r");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.has_mixed_line_endings());
    assert_eq!(file.line_ending(), LineEnding::CrLf);
    // The lone `\r` at the end terminates the last line, so the text ends in a
    // line break even though the file has nothing after it.
    assert_eq!(file.text(), "one\ntwo\nthree\n");

    file.save().expect("a save");
    assert_eq!(bytes_at(&path), b"one\r\ntwo\nthree\r");
}

#[test]
fn an_edited_file_is_written_in_its_own_style() {
    let scratch = Scratch::new("eol-edited");
    let path = scratch.file("mixed.txt", b"one\r\ntwo\nthree");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("!"));
    assert!(file.is_dirty());
    file.save().expect("a save");

    // Editing settles the file on one style: the first one it had.
    assert_eq!(bytes_at(&path), b"!one\r\ntwo\r\nthree");

    // The mixed endings were in the file that was read; what is on disk now is
    // all one style, as reopening it shows.
    let reopened = TextFile::open(&path).expect("the file opens");
    assert!(!reopened.has_mixed_line_endings());
    assert_eq!(reopened.line_ending(), LineEnding::CrLf);
}

#[test]
fn a_new_document_is_saved_with_crlf() {
    let scratch = Scratch::new("eol-new");
    let path = scratch.0.join("new.txt");

    let mut file = TextFile::untitled();
    assert_eq!(file.line_ending(), LineEnding::DEFAULT);
    assert_eq!(file.line_ending(), LineEnding::CrLf);
    assert!(file.insert_str("one"));
    assert!(file.insert_newline());
    assert!(file.insert_str("two"));
    file.save_as(&path, file.encoding(), false).expect("a save");

    assert_eq!(bytes_at(&path), b"one\r\ntwo");
}

#[test]
fn a_file_with_no_terminator_gains_the_default_style_when_it_gains_a_line() {
    let scratch = Scratch::new("eol-switch");
    let path = scratch.file("none.txt", b"one");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.line_ending(), LineEnding::DEFAULT);
    file.document_mut().set_caret(3);
    assert!(file.insert_newline());
    assert!(file.insert_str("two"));
    file.save().expect("a save");

    // There was no terminator to copy, so the new one is the default style.
    assert_eq!(bytes_at(&path), b"one\r\ntwo");
    assert_eq!(file.line_ending(), LineEnding::CrLf);
}
