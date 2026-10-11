//! NC-E001 — whether a document has changed since it was last saved.
//!
//! Every edit raises the flag and a successful save lowers it; undo and redo
//! raise it too, since the document is no longer what was written even when the
//! text has come back to the same characters. The window is what asks the user
//! before closing (`docs/reference-matrix.md` §5).

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
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn typed(text: &str) -> TextFile {
    let mut file = TextFile::untitled();
    assert!(file.insert_str(text), "typing {text:?}");
    file
}

#[test]
fn a_new_document_is_clean() {
    let file = TextFile::untitled();
    assert!(!file.has_path());
    assert!(!file.is_dirty());
    assert!(file.text().is_empty());
}

#[test]
fn typing_raises_the_flag() {
    let file = typed("one");
    assert!(file.is_dirty());
    assert!(file.can_undo());
}

#[test]
fn deleting_each_way_raises_the_flag() {
    let mut backspace = typed("ab");
    assert!(backspace.backspace());
    assert!(backspace.is_dirty());

    let mut forward = typed("ab");
    forward.select(0, 1);
    assert!(forward.delete_forward());
    assert!(forward.is_dirty());

    let mut selection = typed("ab");
    selection.select(0, 2);
    assert!(selection.delete_selection());
    assert!(selection.is_dirty());
    assert_eq!(selection.text(), "");
}

#[test]
fn an_edit_that_changes_nothing_leaves_the_document_clean() {
    let mut file = TextFile::untitled();
    assert!(!file.insert_str(""), "typing nothing is not an edit");
    assert!(!file.backspace(), "backspace at the start");
    assert!(!file.delete_forward(), "delete at the end");
    assert!(!file.delete_selection(), "no selection");
    assert!(!file.undo(), "nothing to undo");
    assert!(!file.redo(), "nothing to redo");
    assert!(!file.is_dirty());
}

#[test]
fn saving_lowers_the_flag() {
    let scratch = Scratch::new("dirty-save");
    let path = scratch.0.join("note.txt");

    let mut file = typed("one");
    let saved = file.save_as(&path, file.encoding(), false).expect("a save");
    assert_eq!(saved.path, path);
    assert_eq!(saved.bytes, 3);
    assert!(!file.is_dirty());
    assert_eq!(file.external_change(), ExternalChange::None);

    let mut reopened = TextFile::open(&path).expect("the file opens");
    assert!(!reopened.is_dirty());
    assert!(reopened.insert_str("!"));
    assert!(reopened.is_dirty());
    reopened.save().expect("a save");
    assert!(!reopened.is_dirty());

    // Saving again with nothing changed writes the same bytes and stays clean.
    reopened.save().expect("a second save");
    assert!(!reopened.is_dirty());
}

#[test]
fn undo_raises_the_flag_even_when_the_text_comes_back() {
    let scratch = Scratch::new("dirty-undo");
    let path = scratch.0.join("note.txt");

    let mut file = typed("one");
    file.save_as(&path, file.encoding(), false).expect("a save");
    assert!(!file.is_dirty());

    // Its own undo group, so one undo takes back this edit alone.
    assert!(file.document_mut().insert_uncoalesced("!"));
    assert!(file.is_dirty());

    assert!(file.undo());
    assert_eq!(file.text(), "one");
    // The text is what was saved, but the document has been through an edit
    // since, so it is still the user's to keep.
    assert!(file.is_dirty(), "undo must not look like a save");

    assert!(file.can_redo());
    assert!(file.redo());
    assert_eq!(file.text(), "one!");
    assert!(file.is_dirty());
}

#[test]
fn reloading_discards_the_edits_and_clears_the_flag() {
    let scratch = Scratch::new("dirty-reload");
    let path = scratch.0.join("note.txt");
    std::fs::write(&path, b"one").expect("a file");

    let mut file = TextFile::open(&path).expect("the file opens");
    assert!(file.insert_str("!"));
    assert!(file.is_dirty());
    // An opened file starts with the caret at the beginning, so the edit goes
    // in front of the text.
    assert_eq!(file.text(), "!one");

    file.reload().expect("a reload");
    assert!(!file.is_dirty());
    assert_eq!(file.text(), "one");
    assert!(!file.can_undo());
}

#[test]
fn a_save_that_fails_leaves_the_document_dirty() {
    let mut file = typed("one");
    assert!(file.save().is_err(), "a save with no path");
    assert!(file.is_dirty(), "a failed save must not look like a save");
}
