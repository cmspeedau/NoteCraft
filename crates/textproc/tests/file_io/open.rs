//! NC-F004 — opening a file.
//!
//! Opening puts the bytes on screen without changing them: the encoding and the
//! line-ending style are worked out from what is in the file, the document
//! starts clean and with nothing to undo, and two opens of the same file are
//! two independent documents — which is what makes them two tabs
//! (`docs/architecture.md` §4, `docs/feature-matrix.md` NC-F004).

use std::path::PathBuf;

use nc_core::LineEnding;
use nc_textproc::{Encoding, Error, Evidence, ExternalChange, TextFile};

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
fn an_opened_file_lands_in_a_clean_document() {
    let scratch = Scratch::new("open-clean");
    let path = scratch.file("note.txt", b"one\r\ntwo\r\n");

    let file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.path(), path);
    assert!(file.has_path());
    assert_eq!(file.text(), "one\ntwo\n");
    assert_eq!(file.line_ending(), LineEnding::CrLf);
    assert_eq!(file.line_count(), 3);
    assert_eq!(file.encoding(), Encoding::Utf8);
    assert!(!file.has_bom());
    assert_eq!(file.detection().evidence, Evidence::ValidUtf8);
    assert!(!file.is_dirty(), "opening is not an edit");
    assert!(!file.can_undo(), "opening is not an edit");
    assert!(!file.can_redo());
    assert_eq!(file.external_change(), ExternalChange::None);
    assert_eq!(file.on_disk(), Some(b"one\r\ntwo\r\n".as_slice()));
}

#[test]
fn an_empty_file_opens_as_an_empty_document() {
    let scratch = Scratch::new("open-empty");
    let path = scratch.file("empty.txt", b"");

    let file = TextFile::open(&path).expect("the file opens");
    assert!(file.is_empty());
    assert_eq!(file.len(), 0);
    assert_eq!(file.text(), "");
    assert_eq!(file.line_count(), 1, "an empty file is one empty line");
    assert_eq!(file.encoding(), Encoding::Utf8);
    assert!(!file.has_bom());
    // A file with no terminator at all has no style to keep, so the document
    // carries the default one.
    assert_eq!(file.line_ending(), LineEnding::CrLf);
    assert!(!file.has_mixed_line_endings());
}

#[test]
fn the_encoding_and_the_style_come_from_the_bytes() {
    let scratch = Scratch::new("open-detect");

    let marked = scratch.file("marked.txt", b"\xEF\xBB\xBFhi");
    let file = TextFile::open(&marked).expect("the file opens");
    assert_eq!(file.encoding(), Encoding::Utf8);
    assert!(file.has_bom());
    assert_eq!(file.text(), "hi", "the mark is not text");

    let ansi = scratch.file("ansi.txt", b"caf\xE9");
    let file = TextFile::open(&ansi).expect("the file opens");
    assert_eq!(file.encoding(), Encoding::Ansi);
    assert_eq!(file.text(), "café");

    let wide = scratch.file("wide.txt", b"\xFF\xFEo\x00n\x00e\x00");
    let file = TextFile::open(&wide).expect("the file opens");
    assert_eq!(file.encoding(), Encoding::Utf16Le);
    assert_eq!(file.text(), "one");

    let lf = scratch.file("lf.txt", b"one\ntwo\n");
    let file = TextFile::open(&lf).expect("the file opens");
    assert_eq!(file.line_ending(), LineEnding::Lf);
    assert_eq!(file.text(), "one\ntwo\n");
}

#[test]
fn opening_the_same_file_twice_gives_two_documents() {
    let scratch = Scratch::new("open-twice");
    let path = scratch.file("note.txt", b"one");

    let mut first = TextFile::open(&path).expect("the file opens");
    let second = TextFile::open(&path).expect("the file opens again");

    assert!(first.insert_str("!"));
    assert_eq!(first.text(), "!one");
    assert_eq!(second.text(), "one", "the other tab has its own document");
    assert!(first.is_dirty());
    assert!(!second.is_dirty());
}

#[test]
fn a_file_that_is_not_there_says_which_one() {
    let scratch = Scratch::new("open-missing");
    let missing = scratch.0.join("nowhere.txt");

    match TextFile::open(&missing) {
        Err(Error::Read { path, .. }) => assert_eq!(path, missing),
        other => panic!("a missing file should be a read error, got {other:?}"),
    }
}

#[test]
fn a_directory_is_not_a_file_to_open() {
    let scratch = Scratch::new("open-directory");

    match TextFile::open(&scratch.0) {
        Err(Error::Read { path, .. }) => assert_eq!(path, scratch.0),
        other => panic!("a directory should be a read error, got {other:?}"),
    }
}

#[test]
fn open_as_reads_the_encoding_it_was_told() {
    let scratch = Scratch::new("open-as");
    // Bytes that are not UTF-8, read as ANSI on purpose: the Open dialog's
    // encoding list (`docs/ui-reference.md` §7).
    let path = scratch.file("note.txt", b"caf\xE9");

    let file = TextFile::open_as(&path, Encoding::Ansi, false).expect("the file opens as ANSI");
    assert_eq!(file.text(), "café");
    assert_eq!(file.encoding(), Encoding::Ansi);
    assert_eq!(file.detection().evidence, Evidence::Chosen);
    assert_eq!(file.on_disk(), Some(b"caf\xE9".as_slice()));
    assert!(!file.is_dirty());
}

#[test]
fn open_as_reports_bytes_the_encoding_cannot_read() {
    let scratch = Scratch::new("open-as-bad");
    let path = scratch.file("note.txt", b"caf\xE9");

    match TextFile::open_as(&path, Encoding::Utf8, false) {
        Err(Error::Decode {
            path: named,
            encoding,
            ..
        }) => {
            assert_eq!(named, path);
            assert_eq!(encoding, Encoding::Utf8);
        }
        other => panic!("bytes that are not UTF-8 should not open as UTF-8, got {other:?}"),
    }
}

#[test]
fn open_as_remembers_a_mark_so_a_save_reproduces_it() {
    let scratch = Scratch::new("open-as-bom");
    let path = scratch.file("note.txt", b"\xFF\xFEo\x00n\x00e\x00");

    let mut file =
        TextFile::open_as(&path, Encoding::Utf16Le, true).expect("the file opens as UTF-16 LE");
    assert_eq!(file.text(), "one");
    assert!(file.has_bom());

    file.save().expect("the file saves");
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"\xFF\xFEo\x00n\x00e\x00"
    );
}

#[test]
fn open_as_reports_a_missing_file() {
    let scratch = Scratch::new("open-as-missing");
    let missing = scratch.0.join("nowhere.txt");

    match TextFile::open_as(&missing, Encoding::Utf8, false) {
        Err(Error::Read { path, .. }) => assert_eq!(path, missing),
        other => panic!("a missing file should be a read error, got {other:?}"),
    }
}

#[test]
fn opening_does_not_touch_the_bytes_on_disk() {
    let scratch = Scratch::new("open-untouched");
    // A mixture the reader has to cope with without rewriting anything: a mark
    // its body does not live up to, a bare CR, and a byte that is not UTF-8.
    let bytes = b"\xEF\xBB\xBF\xFF\xFEa\rb\xE9".to_vec();
    let path = scratch.file("odd.bin", &bytes);

    let file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.on_disk(), Some(bytes.as_slice()));
    assert_eq!(std::fs::read(&path).expect("the file"), bytes);
    assert!(!file.is_dirty());
}
