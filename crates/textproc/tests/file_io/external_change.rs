//! NC-D006 — noticing that the file changed underneath the document.
//!
//! NoteCraft answers whether the bytes on disk are still what it last read or
//! wrote; deciding what to do about it — ask, reload, or ignore — is the
//! window's rule (`docs/ui-reference.md`).

use std::path::PathBuf;

use nc_textproc::{ExternalChange, TextFile};

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
fn a_document_that_has_never_been_saved_has_no_change_to_report() {
    let file = TextFile::untitled();
    assert_eq!(file.external_change(), ExternalChange::None);
}

#[test]
fn a_file_that_is_as_we_left_it_is_unchanged() {
    let scratch = Scratch::new("external-none");
    let path = scratch.file("note.txt", b"one\r\ntwo");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.external_change(), ExternalChange::None);

    assert!(file.insert_str("!"));
    file.save().expect("a save");
    assert_eq!(file.external_change(), ExternalChange::None);
}

#[test]
fn different_bytes_underneath_are_a_change() {
    let scratch = Scratch::new("external-modified");
    let path = scratch.file("note.txt", b"one");

    let file = TextFile::open(&path).expect("the file opens");
    std::fs::write(&path, b"another program was here").expect("an outside write");

    assert_eq!(file.external_change(), ExternalChange::Modified);
}

#[test]
fn bytes_written_back_unchanged_are_not_a_change() {
    let scratch = Scratch::new("external-same");
    let path = scratch.file("note.txt", b"one\r\n");

    let file = TextFile::open(&path).expect("the file opens");
    // A program that touches its files without changing them is not a change.
    std::fs::write(&path, b"one\r\n").expect("a rewrite of the same bytes");

    assert_eq!(file.external_change(), ExternalChange::None);
}

#[test]
fn a_file_that_was_removed_is_a_change() {
    let scratch = Scratch::new("external-removed");
    let path = scratch.file("note.txt", b"one");

    let file = TextFile::open(&path).expect("the file opens");
    std::fs::remove_file(&path).expect("an outside delete");

    assert_eq!(file.external_change(), ExternalChange::Removed);
}

#[test]
fn reloading_takes_the_new_bytes_and_their_encoding() {
    let scratch = Scratch::new("external-reload");
    let path = scratch.file("note.txt", b"one");

    let mut file = TextFile::open(&path).expect("the file opens");
    // The other program wrote a marked UTF-16 file over ours.
    let mut wide = vec![0xFF, 0xFE];
    for unit in "two".encode_utf16() {
        wide.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&path, &wide).expect("an outside write");
    assert_eq!(file.external_change(), ExternalChange::Modified);

    file.reload().expect("a reload");
    assert_eq!(file.text(), "two");
    assert_eq!(file.encoding(), nc_textproc::encoding::Encoding::Utf16Le);
    assert_eq!(file.external_change(), ExternalChange::None);
    assert!(!file.is_dirty());
}

#[test]
fn saving_over_an_outside_change_writes_the_document_and_forgets_it() {
    let scratch = Scratch::new("external-save");
    let path = scratch.file("note.txt", b"one");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("mine"));
    std::fs::write(&path, b"theirs").expect("an outside write");
    assert_eq!(file.external_change(), ExternalChange::Modified);

    // NoteCraft does not resolve the conflict; the caller does, and saving is
    // how the user's answer "keep mine" is carried out.
    file.save().expect("a save");
    assert_eq!(std::fs::read(&path).expect("the file"), b"mineone");
    assert_eq!(file.external_change(), ExternalChange::None);
}

#[test]
fn keeping_the_local_version_marks_it_dirty_without_touching_the_text() {
    let scratch = Scratch::new("external-keep");
    let path = scratch.file("note.txt", b"one");
    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("mine"));
    file.save().expect("a save");
    assert!(!file.is_dirty());

    std::fs::write(&path, b"theirs").expect("an outside write");
    assert_eq!(file.external_change(), ExternalChange::Modified);

    file.keep_local_version().expect("keeping the local text");

    // The user's text stands, and it is now their unsaved work against the file.
    assert_eq!(file.text(), "mineone");
    assert!(file.is_dirty());
    assert_eq!(file.external_change(), ExternalChange::None);

    file.save().expect("a save");
    assert_eq!(std::fs::read(&path).expect("the file"), b"mineone");
    assert_eq!(file.external_change(), ExternalChange::None);
}

#[test]
fn keeping_the_local_version_of_a_removed_file_lets_a_save_recreate_it() {
    let scratch = Scratch::new("external-keep-removed");
    let path = scratch.file("note.txt", b"one");

    let mut file = TextFile::open(&path).expect("the file opens");
    std::fs::remove_file(&path).expect("an outside delete");
    assert_eq!(file.external_change(), ExternalChange::Removed);

    file.keep_local_version().expect("keeping the local text");

    assert_eq!(file.text(), "one");
    assert!(file.is_dirty());
    assert_eq!(file.external_change(), ExternalChange::None);

    file.save().expect("a save recreates the file");
    assert_eq!(std::fs::read(&path).expect("the file"), b"one");
}
