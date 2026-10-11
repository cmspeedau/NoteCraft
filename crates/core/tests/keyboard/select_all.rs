//! Select All (Ctrl+A).
//!
//! Select All is the `Edit` menu's selection-anchor command: it makes the whole
//! document the selection so the next edit replaces it. It is a keyboard command
//! rather than an editing primitive, so it lives in `tests/keyboard/`.

use nc_core::{Document, MemoryClipboard};

#[test]
fn select_all_covers_the_whole_document() {
    let mut document = Document::from_text("alpha\nbeta");
    document.select_all();
    assert!(document.has_selection());
    assert_eq!(document.selection().anchor(), 0);
    assert_eq!(document.selection().head(), document.byte_len());
    assert_eq!(document.selected_text(), Some("alpha\nbeta"));
    assert_eq!(document.caret(), document.byte_len());
}

#[test]
fn select_all_on_an_empty_document_selects_nothing() {
    let mut document = Document::new();
    document.select_all();
    assert!(!document.has_selection());
    assert_eq!(document.selected_text(), None);
}

#[test]
fn select_all_replaces_what_was_selected_before() {
    let mut document = Document::from_text("one two three");
    document.select(4, 7);
    assert_eq!(document.selected_text(), Some("two"));
    document.select_all();
    assert_eq!(document.selected_text(), Some("one two three"));
}

#[test]
fn typing_over_a_select_all_replaces_the_whole_document() {
    let mut document = Document::from_text("delete me");
    document.select_all();
    assert!(document.insert_str("kept"));
    assert_eq!(document.text(), "kept");
    assert_eq!(document.caret(), 4);
}

#[test]
fn a_select_all_can_be_undone_as_one_step() {
    let mut document = Document::from_text("delete me");
    document.select_all();
    document.insert_str("kept");
    assert_eq!(document.undo_depth(), 1);
    assert!(document.undo());
    assert_eq!(document.text(), "delete me");
    assert_eq!(document.selection().anchor(), 0);
    assert_eq!(document.selection().head(), 9);
}

#[test]
fn select_all_then_copy_puts_the_document_on_the_clipboard() {
    let mut document = Document::from_text("one\ntwo");
    let mut clipboard = MemoryClipboard::new();
    document.select_all();
    assert!(document.copy_to(&mut clipboard));
    assert_eq!(clipboard.contents(), Some("one\r\ntwo"));
    assert_eq!(document.text(), "one\ntwo");
}

#[test]
fn select_all_then_cut_empties_the_document_and_can_be_undone() {
    let mut document = Document::from_text("one\ntwo");
    let mut clipboard = MemoryClipboard::new();
    document.select_all();
    assert!(document.cut_to(&mut clipboard));
    assert_eq!(document.text(), "");
    assert_eq!(document.line_count(), 1);
    assert_eq!(clipboard.contents(), Some("one\r\ntwo"));
    assert!(document.undo());
    assert_eq!(document.text(), "one\ntwo");
}

#[test]
fn a_second_select_all_after_editing_covers_the_new_text() {
    let mut document = Document::from_text("abc");
    document.select_all();
    document.insert_str("de");
    document.select_all();
    assert_eq!(document.selected_text(), Some("de"));
}

#[test]
fn select_all_reaches_the_end_of_a_file_that_ends_with_a_terminator() {
    let mut document = Document::from_text("a\nb\n");
    document.select_all();
    assert_eq!(document.byte_len(), 4);
    assert_eq!(document.selection().head(), 4);
    assert_eq!(document.selected_text(), Some("a\nb\n"));
}
