//! NC-E001 — Undo and Redo.
//!
//! `docs/feature-matrix.md` NC-E001. The reference's exact grouping of a key
//! run into undo steps is a design decision; these tests pin the behaviour the
//! engine *does* guarantee, so the approximation cannot drift silently.

use nc_core::Document;

#[test]
fn undo_removes_typed_text() {
    let mut document = Document::new();
    assert!(document.insert_str("hello"));
    assert_eq!(document.text(), "hello");

    assert!(document.undo());
    assert_eq!(document.text(), "");
    assert_eq!(document.caret(), 0);
    assert!(!document.can_undo());
}

#[test]
fn a_typing_run_is_one_undo_step() {
    let mut document = Document::new();
    for character in ["a", "b", "c"] {
        assert!(document.insert_str(character));
    }
    assert_eq!(document.text(), "abc");
    assert_eq!(document.undo_depth(), 1);

    document.undo();
    assert_eq!(document.text(), "");
}

#[test]
fn a_newline_starts_its_own_step() {
    let mut document = Document::new();
    document.insert_str("a");
    document.insert_newline();
    document.insert_str("b");
    assert_eq!(document.text(), "a\nb");
    assert_eq!(document.undo_depth(), 3);

    document.undo();
    assert_eq!(document.text(), "a\n");
    document.undo();
    assert_eq!(document.text(), "a");
    document.undo();
    assert_eq!(document.text(), "");
}

#[test]
fn redo_reapplies_an_undone_edit() {
    let mut document = Document::new();
    document.insert_str("one");
    document.insert_newline();
    document.insert_str("two");

    document.undo();
    document.undo();
    assert_eq!(document.text(), "one");
    assert!(document.can_redo());

    document.redo();
    assert_eq!(document.text(), "one\n");
    document.redo();
    assert_eq!(document.text(), "one\ntwo");
    assert!(!document.can_redo());
}

#[test]
fn a_new_edit_discards_the_redo_stack() {
    let mut document = Document::new();
    document.insert_str("first");
    document.undo();
    assert!(document.can_redo());

    document.insert_str("second");
    assert!(!document.can_redo());
    document.redo();
    assert_eq!(document.text(), "second");
}

#[test]
fn undo_restores_the_selection_it_replaced() {
    let mut document = Document::from_text("keep this");
    document.select(0, 4);
    assert_eq!(document.selected_text(), Some("keep"));

    document.insert_str("lose");
    assert_eq!(document.text(), "lose this");

    document.undo();
    assert_eq!(document.text(), "keep this");
    assert_eq!(document.selection().range(), 0..4);
    assert_eq!(document.selected_text(), Some("keep"));
}

#[test]
fn deletion_is_its_own_undo_step() {
    let mut document = Document::from_text("abcd");
    document.set_caret(4);
    assert!(document.backspace());
    assert_eq!(document.text(), "abc");
    assert_eq!(document.caret(), 3);

    document.undo();
    assert_eq!(document.text(), "abcd");
    assert_eq!(document.caret(), 4);
}

#[test]
fn undo_and_redo_on_an_empty_history_are_refused() {
    let mut document = Document::new();
    assert!(!document.undo());
    assert!(!document.redo());
    assert!(!document.can_undo());
    assert!(!document.can_redo());
}

#[test]
fn a_fresh_document_is_not_modified() {
    let mut document = Document::new();
    assert!(!document.is_dirty());
    document.insert_str("x");
    assert!(document.is_dirty());
    document.mark_saved();
    assert!(!document.is_dirty());
}
