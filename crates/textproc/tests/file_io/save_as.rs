//! NC-F006 — Save As: writing to a file the document was not opened from.
//!
//! `docs/feature-matrix.md` names `tests/file_io/save_as.rs` for this row, and
//! `ROADMAP.md` M4 lists "Save As and overwrite confirmation" with it. The
//! dialog and the over-*writing* are the window's business; what this crate owes
//! the row is the part that can be checked: the chosen encoding is the one the
//! file is written in, the document follows to the new path, and a Save As that
//! cannot be written leaves the document exactly as it was.
//!
//! The overwrite question itself is asked by [`nc_textproc::prompt`], whose own
//! tests cover it; here it is used the way the window would, to show that the
//! state it asks about is the state a Save As sees.

use std::path::PathBuf;

use nc_textproc::prompt::{self, Question};
use nc_textproc::{Encoding, TextFile};

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
fn save_as_writes_the_chosen_encoding_and_follows_there() {
    let scratch = Scratch::new("save-as-encoding");
    let path = scratch.path("wide.txt");

    let mut file = TextFile::untitled();
    assert!(file.insert_str("one"));

    // The encoding comes from the dialog, not from the document: a new document
    // is UTF-8 until its Save As says otherwise.
    assert_eq!(file.encoding(), Encoding::Utf8);
    let saved = file
        .save_as(&path, Encoding::Utf16Le, true)
        .expect("a save as");
    assert_eq!(saved.path, path);
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        &[0xFF, 0xFE, b'o', 0x00, b'n', 0x00, b'e', 0x00],
        "UTF-16 LE with its byte order mark"
    );

    // The document is now that file, in that encoding, and clean.
    assert!(file.has_path());
    assert_eq!(file.path(), path);
    assert_eq!(file.encoding(), Encoding::Utf16Le);
    assert_eq!(file.encoding_label(), "UTF-16 LE");
    assert!(file.has_bom());
    assert!(!file.is_dirty());

    // And the next plain save is written the same way — the choice stuck.
    assert!(file.insert_str("!"));
    file.save().expect("a save");
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        &[0xFF, 0xFE, b'o', 0x00, b'n', 0x00, b'e', 0x00, b'!', 0x00],
        "the encoding chosen in Save As is the one the file keeps"
    );
    assert!(!file.is_dirty());
}

#[test]
fn save_as_onto_a_file_that_is_there_is_a_question_first() {
    let scratch = Scratch::new("save-as-overwrite");
    let path = scratch.path("there.txt");
    std::fs::write(&path, b"keep me").expect("the file");

    // The window asks before replacing it; the question names the file.
    assert_eq!(
        prompt::before_overwriting(&path),
        Some(Question::Overwrite { path: path.clone() })
    );
    // A path with nothing behind it raises nothing.
    assert_eq!(prompt::before_overwriting(scratch.path("new.txt")), None);

    // Answering the question means going ahead: the file is replaced whole,
    // with no trace of what was there before.
    let mut file = TextFile::untitled();
    assert!(file.insert_str("replacement"));
    file.save_as(&path, Encoding::Utf8, false)
        .expect("a save as");
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"replacement",
        "Save As replaces the file rather than writing into it"
    );
}

#[test]
fn saving_as_elsewhere_leaves_the_original_alone() {
    let scratch = Scratch::new("save-as-elsewhere");
    let original = scratch.path("original.txt");
    let copy = scratch.path("copy.txt");

    // An ANSI file with a character only the code page can write.
    std::fs::write(&original, b"caf\xE9").expect("the file");
    let mut file = TextFile::open(&original).expect("the file opens");
    assert_eq!(file.encoding(), Encoding::Ansi);

    let end = file.text().len();
    file.document_mut().set_caret(end);
    assert!(file.insert_str("!"));

    file.save_as(&copy, Encoding::Ansi, false)
        .expect("a save as");
    assert_eq!(std::fs::read(&copy).expect("the copy"), b"caf\xE9!");
    assert_eq!(
        std::fs::read(&original).expect("the original"),
        b"caf\xE9",
        "the file the document came from is untouched"
    );

    // The document belongs to the copy now: further edits go there.
    assert_eq!(file.path(), copy);
    assert!(file.insert_str("?"));
    file.save().expect("a save");
    assert_eq!(std::fs::read(&copy).expect("the copy"), b"caf\xE9!?");
    assert_eq!(std::fs::read(&original).expect("the original"), b"caf\xE9");
    assert!(!file.is_dirty());
}

#[test]
fn saving_as_utf8_gives_the_original_file_a_mark_and_a_wide_one_takes_it_away() {
    let scratch = Scratch::new("save-as-bom");
    let path = scratch.path("note.txt");

    // A file saved as UTF-8 with a mark keeps it when it is reopened.
    let mut file = TextFile::untitled();
    assert!(file.insert_str("héllo"));
    file.save_as(&path, Encoding::Utf8, true)
        .expect("a save as");
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        &[0xEF, 0xBB, 0xBF, b'h', 0xC3, 0xA9, b'l', b'l', b'o']
    );

    let reopened = TextFile::open(&path).expect("the file opens");
    assert_eq!(reopened.encoding(), Encoding::Utf8);
    assert!(reopened.has_bom());
    assert_eq!(reopened.text(), "héllo");

    // Saving the same document back as ANSI without a mark drops it: what is on
    // disk is what the last Save As said, not what the file used to say.
    let mut reopened = reopened;
    reopened
        .save_as(&path, Encoding::Ansi, false)
        .expect("a save as");
    assert_eq!(std::fs::read(&path).expect("the file"), b"h\xE9llo");
    assert!(reopened.has_path(), "still the same file");
    assert_eq!(reopened.encoding(), Encoding::Ansi);
    assert!(!reopened.has_bom());
}

#[test]
fn a_save_as_that_cannot_be_written_leaves_the_document_as_it_was() {
    let scratch = Scratch::new("save-as-failure");
    let good = scratch.path("note.txt");
    let nowhere = scratch.path("no-such-directory").join("note.txt");

    let mut file = TextFile::untitled();
    assert!(file.insert_str("one"));

    // Writing into a directory that is not there fails, and the document still
    // has no file and still has its changes.
    let error = file
        .save_as(&nowhere, Encoding::Utf8, false)
        .expect_err("a save as that cannot be written");
    assert!(
        error.to_string().contains("could not write"),
        "the error says what failed, got {error}"
    );
    assert!(!file.has_path(), "a failed Save As does not adopt a path");
    assert!(file.is_dirty());
    assert_eq!(file.text(), "one");

    // The failure left nothing behind, and the document can still be saved
    // somewhere that exists.
    assert!(!nowhere.exists());
    file.save_as(&good, Encoding::Utf8, false)
        .expect("a save as");
    assert_eq!(std::fs::read(&good).expect("the file"), b"one");
    assert!(!file.is_dirty());
}

#[test]
fn a_save_as_into_an_encoding_that_cannot_hold_the_text_reports_the_characters() {
    let scratch = Scratch::new("save-as-unrepresentable");
    let path = scratch.path("wide.txt");

    let mut file = TextFile::untitled();
    assert!(file.insert_str("café 🎉"));

    // The dialog is where the answer is given, but the failure has to come back
    // with the characters that raised it, so the window can name them.
    let error = file
        .save_as(&path, Encoding::Ansi, false)
        .expect_err("ANSI cannot hold an emoji");
    match error {
        nc_textproc::Error::Encode(encode_error) => {
            assert_eq!(encode_error.encoding, Encoding::Ansi);
            // The code page holds `é`; the emoji is what it cannot write.
            assert_eq!(encode_error.characters, vec!['🎉']);
        }
        other => panic!("the failure should name the characters, got {other}"),
    }
    assert!(!path.exists(), "nothing was written");
    assert!(!file.has_path(), "nor did the document move");
    assert!(file.is_dirty());

    // The same text goes to the same path once the encoding can hold it.
    file.save_as(&path, Encoding::Utf8, false)
        .expect("a save as");
    let reopened = TextFile::open(&path).expect("the file opens");
    assert_eq!(reopened.text(), "café 🎉");
}
