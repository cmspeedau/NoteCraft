//! NC-F005 — saving a file.
//!
//! A save writes the document back to the file it came from, in the encoding it
//! was read in: the mark is kept if the file had one, the line-ending style is
//! the file's own, and a save with nothing changed writes the old bytes back
//! exactly as they were. This is where "no silent data loss" is measured
//! (`ROADMAP.md` M4 exit criteria).

use std::path::PathBuf;

use nc_core::LineEnding;
use nc_textproc::{Encoding, Error, ExternalChange, TextFile};

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
        std::fs::write(&path, bytes).expect("a file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn an_untouched_file_is_saved_byte_for_byte() {
    let scratch = Scratch::new("save-untouched");
    // Everything the reader has to be faithful about at once: a mark, a
    // terminator mixture, and a high byte that is not UTF-8.
    let fixtures: Vec<(&str, Vec<u8>)> = vec![
        ("plain.txt", b"one\r\ntwo\r\n".to_vec()),
        ("marked.txt", b"\xEF\xBB\xBFone".to_vec()),
        ("wide.txt", b"\xFF\xFEo\x00n\x00e\x00".to_vec()),
        ("ansi.txt", b"caf\xE9 \x80 \x96".to_vec()),
        ("mixed.txt", b"one\r\ntwo\nthree\r".to_vec()),
        ("odd.txt", b"\xEF\xBB\xBF\xFF\xFEa\rb\xE9".to_vec()),
    ];

    for (name, bytes) in fixtures {
        let path = scratch.file(name, &bytes);
        let mut file = TextFile::open(&path).expect("the file opens");
        assert!(!file.is_dirty(), "{name} opens clean");

        let saved = file
            .save()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(saved.path, path, "{name}");
        assert_eq!(saved.bytes, bytes.len(), "{name} wrote every byte");
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            bytes,
            "{name} came back byte for byte"
        );
        assert!(!file.is_dirty(), "{name} is clean after saving");
    }
}

#[test]
fn an_edited_file_keeps_its_encoding_and_its_style() {
    let scratch = Scratch::new("save-style");

    // UTF-16 with a mark: the mark and the byte order survive the edit.
    let path = scratch.file("wide.txt", b"\xFF\xFEo\x00n\x00e\x00");
    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("!"));
    file.save().expect("a save");
    assert_eq!(file.encoding(), Encoding::Utf16Le);
    assert!(file.has_bom());
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"\xFF\xFE!\x00o\x00n\x00e\x00"
    );

    // ANSI stays ANSI, and the é goes back to its single byte.
    let path = scratch.file("ansi.txt", b"caf\xE9");
    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("!"));
    file.save().expect("a save");
    assert_eq!(std::fs::read(&path).expect("the file"), b"!caf\xE9");

    // LF stays LF: an edit does not impose the default style on a file.
    let path = scratch.file("lf.txt", b"one\ntwo\n");
    let mut file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.line_ending(), LineEnding::Lf);
    file.select(file.text().len(), file.text().len());
    assert!(file.insert_str("three\n"));
    file.save().expect("a save");
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"one\ntwo\nthree\n"
    );
}

#[test]
fn a_save_writes_where_it_read_and_says_so() {
    let scratch = Scratch::new("save-path");
    let path = scratch.file("note.txt", b"one");
    let other = scratch.file("other.txt", b"keep me");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("!"));
    let saved = file.save().expect("a save");

    assert_eq!(saved.path, path);
    assert_eq!(saved.bytes, 4);
    assert_eq!(std::fs::read(&path).expect("the file"), b"!one");
    // Saving one file did not go near another.
    assert_eq!(std::fs::read(&other).expect("the other file"), b"keep me");
    assert_eq!(file.external_change(), ExternalChange::None);
}

#[test]
fn a_document_with_no_path_has_nowhere_to_save() {
    let mut file = TextFile::untitled();
    assert!(file.insert_str("one"));
    match file.save() {
        Err(Error::NoPath) => {}
        other => panic!("a document with no path should have nowhere to save, got {other:?}"),
    }
    // The edit is still there: a save that could not happen changed nothing.
    assert_eq!(file.text(), "one");
    assert!(file.is_dirty());
}

#[test]
fn saving_over_a_file_that_changed_underneath_still_writes_ours() {
    let scratch = Scratch::new("save-outside");
    let path = scratch.file("note.txt", b"one");

    let mut file = TextFile::open(&path).expect("the file opens");
    std::fs::write(&path, b"something else entirely").expect("an outside write");
    assert_eq!(file.external_change(), ExternalChange::Modified);

    assert!(file.insert_str("!"));
    file.save().expect("a save");
    // A plain save writes the document; it is the window that decides whether
    // to ask first (`docs/reference-matrix.md` §5).
    assert_eq!(std::fs::read(&path).expect("the file"), b"!one");
    assert_eq!(file.external_change(), ExternalChange::None);
}
