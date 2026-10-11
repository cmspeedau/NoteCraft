//! NC-F002 — large files, and files with many lines.
//!
//! The reading and writing rules are the same at size as they are on a small
//! note; these are the cases where an accidental quadratic or a byte-at-a-time
//! path shows up as a test that takes minutes instead of one that takes a
//! moment. One mebibyte is a note, not a document, and is enough to catch both.

use std::path::PathBuf;

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
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A file of `lines` lines, each as wide as the line number padded to 8.
fn contents(lines: usize) -> String {
    let mut text = String::with_capacity(lines * 24);
    for number in 0..lines {
        text.push_str(&format!("{number:08}  a line of text\r\n"));
    }
    text
}

#[test]
fn a_megabyte_file_opens_with_the_right_text_and_lines() {
    let scratch = Scratch::new("stress-large");
    let path = scratch.0.join("large.txt");
    let expected = contents(50_000);
    std::fs::write(&path, &expected).expect("a large file");
    assert!(expected.len() > 1_000_000, "{} bytes", expected.len());

    let mut file = TextFile::open(&path).expect("the file opens");
    assert_eq!(file.line_ending(), LineEnding::CrLf);
    // Every line ends in a terminator, so there is an empty line after the last
    // one — the line the caret would sit on at the end of the file.
    assert_eq!(file.line_count(), 50_001);
    // The buffer holds the same text with one byte fewer per line: the
    // terminator is a single '\n' where the file had '\r\n'.
    assert_eq!(file.text().len(), expected.len() - 50_000);

    // Untouched, it goes back to disk byte for byte.
    file.save().expect("a save");
    assert_eq!(std::fs::read(&path).expect("the file"), expected.as_bytes());
}

#[test]
fn a_large_file_takes_edits_and_undoes_them() {
    let scratch = Scratch::new("stress-edit");
    let path = scratch.0.join("large.txt");
    std::fs::write(&path, contents(20_000)).expect("a large file");

    let mut file = TextFile::open(&path).expect("the file opens");
    let before = file.text().to_string();
    let lines = file.line_count();
    assert_eq!(lines, 20_001, "20,000 lines and the empty one after them");

    file.select(0, 0);
    // Typing at one spot coalesces into a single undo group, so this is one
    // step to the user however many keystrokes it takes (`undo.rs`). The count
    // is kept modest because the line index is rebuilt after every edit, by
    // design for now (`docs/architecture.md` §4).
    let edits = 500;
    let mut inserted = 0;
    for index in 0..edits {
        let typed = format!("{index} ");
        inserted += typed.len();
        assert!(file.insert_str(&typed), "edit {index}");
    }
    assert!(
        inserted > edits,
        "each edit is at least one byte and a space"
    );
    assert_eq!(file.text().len(), before.len() + inserted);
    assert!(file.is_dirty());

    assert!(file.undo());
    assert_eq!(file.text(), before);
    assert!(!file.can_undo());
    // The file on disk was never touched by any of it.
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        contents(20_000).as_bytes()
    );
}

#[test]
fn saving_over_and_over_keeps_writing_the_document() {
    let scratch = Scratch::new("stress-saves");
    let path = scratch.0.join("many.txt");

    let mut file = TextFile::untitled();
    for round in 0..100 {
        file.select(file.text().len(), file.text().len());
        assert!(file.insert_str(&format!("round {round} ")));
        file.save_as(&path, file.encoding(), false).expect("a save");
        assert!(!file.is_dirty());
    }

    let text = std::fs::read_to_string(&path).expect("the file");
    assert!(text.starts_with("round 0 round 1 "));
    assert!(text.ends_with("round 99 "));
    assert_eq!(std::fs::read(&path).expect("the file").len(), text.len());

    let reopened = TextFile::open(&path).expect("the file opens");
    assert_eq!(reopened.text(), file.text());
}
