//! Overwrite (overtype) mode.
//!
//! ROADMAP M3 asks for "overwrite behavior if applicable"; the reference's
//! behaviour is not captured and the feature matrix has no row for it, so the
//! engine guarantees only what `Document::insert_impl` documents: an overwrite
//! replaces one grapheme per grapheme typed and never eats the line terminator.

use nc_core::{ClickKind, Document};

fn document(text: &str) -> Document {
    Document::from_text(text)
}

#[test]
fn overwrite_is_off_until_it_is_switched_on() {
    let mut document = document("abcd");
    assert!(!document.overwrite());
    document.set_overwrite(true);
    assert!(document.overwrite());
    document.set_overwrite(false);
    assert!(!document.overwrite());
}

#[test]
fn toggling_overwrite_reports_the_new_state() {
    let mut document = document("abcd");
    assert!(document.toggle_overwrite());
    assert!(document.overwrite());
    assert!(!document.toggle_overwrite());
    assert!(!document.overwrite());
}

#[test]
fn overwriting_replaces_as_many_graphemes_as_were_typed() {
    let mut document = document("abcd");
    document.set_overwrite(true);
    assert!(document.insert_str("XY"));
    assert_eq!(document.text(), "XYcd");
    assert_eq!(document.caret(), 2);
}

#[test]
fn overwriting_past_the_end_of_the_line_inserts() {
    let mut document = document("ab");
    document.click(2, ClickKind::Single);
    document.set_overwrite(true);
    assert!(document.insert_str("c"));
    assert_eq!(document.text(), "abc");
    assert_eq!(document.caret(), 3);
}

#[test]
fn overwriting_never_eats_the_line_terminator() {
    let mut document = document("ab\ncd");
    document.set_overwrite(true);
    assert!(document.insert_str("XYZ"));
    assert_eq!(document.text(), "XYZ\ncd");
    assert_eq!(document.caret(), 3);
}

#[test]
fn overwriting_replaces_whole_graphemes() {
    // "é" is two bytes: an overwrite of one grapheme must take both of them, so
    // the buffer never splits a character.
    let mut document = document("héllo");
    document.click(1, ClickKind::Single);
    document.set_overwrite(true);
    assert!(document.insert_str("X"));
    assert_eq!(document.text(), "hXllo");
    assert_eq!(document.caret(), 2);
}

#[test]
fn overwriting_with_a_multibyte_insert_ends_on_a_character_boundary() {
    let mut document = document("abcdef");
    document.set_overwrite(true);
    assert!(document.insert_str("é"));
    assert_eq!(document.text(), "ébcdef");
    assert!(document.text().is_char_boundary(document.caret()));
}

#[test]
fn an_overwrite_is_its_own_undo_step() {
    let mut document = document("abcd");
    document.set_overwrite(true);
    assert!(document.insert_str("X"));
    assert!(document.insert_str("Y"));
    assert_eq!(document.text(), "XYcd");
    assert_eq!(document.undo_depth(), 2);

    assert!(document.undo());
    assert_eq!(document.text(), "Xbcd");
    assert!(document.undo());
    assert_eq!(document.text(), "abcd");
    assert_eq!(document.caret(), 0);
}

#[test]
fn undoing_an_overwrite_restores_the_replaced_text() {
    let mut document = document("abcd");
    document.set_overwrite(true);
    assert!(document.insert_str("XYZ"));
    assert_eq!(document.text(), "XYZd");
    assert!(document.undo());
    assert_eq!(document.text(), "abcd");
    assert!(document.redo());
    assert_eq!(document.text(), "XYZd");
}

#[test]
fn an_overwrite_still_replaces_a_selection() {
    // With a selection there is nothing to overtype, so the selection rules.
    let mut document = document("abcd");
    document.click(0, ClickKind::Single);
    document.extend_to(2, ClickKind::Single);
    document.set_overwrite(true);
    assert!(document.insert_str("Z"));
    assert_eq!(document.text(), "Zcd");
}

#[test]
fn an_insert_that_carries_a_line_break_replaces_the_selection() {
    // A paste is not a keystroke: it replaces the selection instead of
    // overtyping a line.
    let mut document = document("ab\ncd");
    document.click(0, ClickKind::Single);
    document.extend_to(2, ClickKind::Single);
    document.set_overwrite(true);
    assert!(document.insert_str("X\nY"));
    assert_eq!(document.text(), "X\nY\ncd");
}

#[test]
fn overwriting_off_inserts_without_removing_anything() {
    let mut document = document("abcd");
    assert!(document.insert_str("XY"));
    assert_eq!(document.text(), "XYabcd");
    assert_eq!(document.caret(), 2);
}

#[test]
fn overwriting_is_recorded_as_a_dirty_edit() {
    let mut document = document("abcd");
    document.mark_saved();
    document.set_overwrite(true);
    assert!(document.insert_str("X"));
    assert!(document.is_dirty());
}
