//! NC-D004 — what a failed read or write does, and what it does not do.
//!
//! A write that cannot happen leaves the file exactly as it was: NoteCraft
//! writes through a neighbouring temporary file and renames it into place, so a
//! half-written note is never what the user finds. A read that cannot happen
//! says so, and names the file.

use std::path::PathBuf;

use nc_textproc::encoding::Encoding;
use nc_textproc::{Error, TextFile};

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

#[test]
fn a_read_that_fails_names_the_file() {
    let scratch = Scratch::new("errors-read");
    let missing = scratch.0.join("not-there.txt");

    let error = TextFile::open(&missing).expect_err("there is no such file");
    match &error {
        Error::Read { path, source } => {
            assert_eq!(path, &missing);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected a read error, got {other:?}"),
    }
    assert!(error.to_string().contains("not-there.txt"));
}

#[test]
fn saving_a_document_with_no_path_says_so() {
    let scratch = Scratch::new("errors-nopath");
    let mut file = TextFile::untitled();
    assert!(!file.has_path());
    assert!(file.insert_str("one"));

    let error = file.save().expect_err("nowhere to save to");
    assert!(matches!(error, Error::NoPath), "{error:?}");
    assert!(error.to_string().contains("path"));

    // Nothing was created anywhere: the document still has no file.
    assert!(!file.has_path());
    let entries = std::fs::read_dir(&scratch.0)
        .expect("the scratch directory")
        .count();
    assert_eq!(entries, 0);
}

#[test]
fn a_write_that_cannot_happen_leaves_the_file_as_it_was() {
    let scratch = Scratch::new("errors-write");
    let path = scratch.file("note.txt", b"one");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("!"));

    // A directory is not a file, so the write through the temporary file
    // cannot land.
    let error = file
        .save_as(&scratch.0, Encoding::Utf8, false)
        .expect_err("a directory cannot be written as a file");

    match &error {
        Error::Write { path: written, .. } => assert_eq!(written, &scratch.0),
        other => panic!("expected a write error, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&path).expect("the file is still there"),
        b"one"
    );
}

#[test]
fn a_write_over_a_read_only_file_fails_and_leaves_it_alone() {
    let scratch = Scratch::new("errors-readonly");
    let path = scratch.file("locked.txt", b"one");

    let mut permissions = std::fs::metadata(&path).expect("the file").permissions();
    permissions.set_readonly(true);
    if std::fs::set_permissions(&path, permissions).is_err() {
        return;
    }
    // A privileged user can write a read-only file, and then there is nothing
    // this test can say.
    if std::fs::OpenOptions::new().write(true).open(&path).is_ok() {
        return;
    }

    let mut file = TextFile::open(&path).expect("the file opens for reading");
    assert!(file.insert_str("!"));

    let error = file.save().expect_err("a read-only file cannot be written");
    match &error {
        Error::Write { path: written, .. } => assert_eq!(written, &path),
        other => panic!("expected a write error, got {other:?}"),
    }
    assert!(file.is_dirty(), "a failed save leaves the document dirty");
    assert_eq!(
        std::fs::read(&path).expect("the file is still there"),
        b"one"
    );
}

#[test]
fn text_the_code_page_cannot_hold_is_reported_before_it_is_lost() {
    let scratch = Scratch::new("errors-encode");
    let path = scratch.file("ansi.txt", b"caf\xE9");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.encoding(), Encoding::Ansi);
    assert_eq!(file.text(), "café");

    // An opened file starts with the caret at the beginning; the edit is meant
    // to follow the text that is there. `set_caret` takes a byte offset.
    let end = file.text().len();
    file.document_mut().set_caret(end);
    assert!(file.insert_str(" 🙂→"));
    let error = file
        .save()
        .expect_err("the code page has no byte for these");
    match &error {
        Error::Encode(cannot) => {
            assert_eq!(cannot.encoding, Encoding::Ansi);
            assert_eq!(cannot.characters, vec!['🙂', '→']);
        }
        other => panic!("expected an encode error, got {other:?}"),
    }
    assert!(error.to_string().contains("2 character(s)"));
    // Nothing was written: the file is untouched.
    assert_eq!(
        std::fs::read(&path).expect("the file is still there"),
        b"caf\xE9"
    );

    // The user's answer to that question is what writes the question marks.
    let (saved, replaced) = file.save_lossy().expect("a lossy save");
    assert_eq!(replaced, vec!['🙂', '→']);
    // One byte per character in the code page: "caf" + "é" + " " + "??".
    assert_eq!(saved.bytes, 7);
    assert_eq!(
        std::fs::read(&path).expect("the file was written"),
        b"caf\xE9 ??"
    );
    assert!(!file.is_dirty());
}

#[test]
fn saving_as_a_chosen_encoding_writes_that_encoding() {
    let scratch = Scratch::new("errors-saveas");
    let path = scratch.0.join("chosen.txt");

    let mut file = TextFile::untitled();
    assert!(file.insert_str("café"));
    file.save_as(&path, Encoding::Ansi, false)
        .expect("an ANSI save");

    assert_eq!(std::fs::read(&path).expect("the file"), b"caf\xE9");
    assert_eq!(file.encoding(), Encoding::Ansi);

    let reopened = TextFile::open(&path).expect("the file opens");
    assert_eq!(reopened.encoding(), Encoding::Ansi);
    assert_eq!(reopened.text(), "café");
}

#[test]
fn a_failed_save_as_leaves_a_new_document_with_no_path() {
    let scratch = Scratch::new("errors-saveas-fail");
    let mut file = TextFile::untitled();
    assert!(file.insert_str("café 🙂"));

    let path = scratch.0.join("x.txt");
    assert!(file.save_as(path, Encoding::Ansi, false).is_err());
    assert!(!file.has_path(), "a failed Save As is not a path");
    assert!(file.is_dirty());
}
