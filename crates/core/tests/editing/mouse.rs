//! NC-E005 — mouse selection: click, drag, double-click, triple-click.
//!
//! The reference's exact click rule is a capture item, so these tests pin the
//! approximation `crates/core/src/mouse.rs` names: one UAX #29 word segment for
//! a double-click, the whole line including its terminator for a triple-click,
//! and a granular drag that grows from the far edge of the unit the button went
//! down on.

use nc_core::{CaretMovement, ClickKind, Document, TextBuffer, click_selection, drag_selection};

fn buffer(text: &str) -> TextBuffer {
    TextBuffer::from_normalized(text)
}

#[test]
fn a_run_of_clicks_maps_to_a_kind() {
    assert_eq!(ClickKind::from_count(0), ClickKind::Single);
    assert_eq!(ClickKind::from_count(1), ClickKind::Single);
    assert_eq!(ClickKind::from_count(2), ClickKind::Double);
    assert_eq!(ClickKind::from_count(3), ClickKind::Triple);
    assert_eq!(ClickKind::from_count(4), ClickKind::Single);
}

#[test]
fn a_single_click_places_the_caret() {
    let mut document = Document::from_text("hello world");
    document.click(3, ClickKind::Single);
    assert_eq!(document.caret(), 3);
    assert!(!document.has_selection());
}

#[test]
fn a_single_click_clamps_to_a_character_boundary() {
    let text = "héllo wörld";
    let mut document = Document::from_text(text);
    document.click(2, ClickKind::Single);
    assert!(document.text().is_char_boundary(document.caret()));
    document.click(99, ClickKind::Single);
    assert_eq!(document.caret(), text.len());
}

#[test]
fn a_double_click_selects_the_word_under_the_pointer() {
    let mut document = Document::from_text("hello world");
    document.click(7, ClickKind::Double);
    assert_eq!(document.selected_text(), Some("world"));
}

#[test]
fn a_double_click_on_a_word_start_takes_that_word() {
    let mut document = Document::from_text("hello world");
    document.click(6, ClickKind::Double);
    assert_eq!(document.selected_text(), Some("world"));
}

#[test]
fn a_double_click_past_the_last_word_takes_it() {
    let mut document = Document::from_text("hello world");
    document.click(99, ClickKind::Double);
    assert_eq!(document.selected_text(), Some("world"));
}

#[test]
fn a_double_click_in_the_gap_takes_the_whitespace_run() {
    let mut document = Document::from_text("hello world");
    document.click(5, ClickKind::Double);
    assert_eq!(document.selected_text(), Some(" "));
}

#[test]
fn a_double_click_on_empty_text_selects_nothing() {
    let mut document = Document::from_text("");
    document.click(0, ClickKind::Double);
    assert!(!document.has_selection());
}

#[test]
fn a_triple_click_selects_the_line_with_its_terminator() {
    let mut document = Document::from_text("one\ntwo\nthree");
    document.click(5, ClickKind::Triple);
    assert_eq!(document.selected_text(), Some("two\n"));
}

#[test]
fn a_triple_click_on_the_last_line_selects_it_to_the_end() {
    let mut document = Document::from_text("one\ntwo\nthree");
    document.click(99, ClickKind::Triple);
    assert_eq!(document.selected_text(), Some("three"));
}

#[test]
fn a_drag_extends_the_selection_forwards() {
    let mut document = Document::from_text("hello world");
    document.click(2, ClickKind::Single);
    document.extend_to(8, ClickKind::Single);
    assert_eq!(document.selected_text(), Some("llo wo"));
    assert_eq!(document.selection().anchor(), 2);
    assert_eq!(document.caret(), 8);
}

#[test]
fn a_drag_extends_the_selection_backwards() {
    let mut document = Document::from_text("hello world");
    document.click(8, ClickKind::Single);
    document.extend_to(2, ClickKind::Single);
    assert_eq!(document.selected_text(), Some("llo wo"));
    assert_eq!(document.selection().anchor(), 8);
    assert_eq!(document.caret(), 2);
}

#[test]
fn a_drag_begun_with_a_double_click_extends_by_words() {
    let mut document = Document::from_text("hello world");
    document.click(1, ClickKind::Double);
    document.extend_to(8, ClickKind::Double);
    assert_eq!(document.selected_text(), Some("hello world"));
}

#[test]
fn a_granular_drag_backwards_keeps_the_whole_word_it_started_on() {
    let mut document = Document::from_text("hello world");
    document.click(7, ClickKind::Double);
    document.extend_to(2, ClickKind::Double);
    assert_eq!(document.selected_text(), Some("hello world"));
}

#[test]
fn a_drag_begun_with_a_triple_click_extends_by_lines() {
    let mut document = Document::from_text("one\ntwo\nthree\nfour");
    document.click(2, ClickKind::Triple);
    document.extend_to(10, ClickKind::Triple);
    assert_eq!(document.selected_text(), Some("one\ntwo\nthree\n"));
}

#[test]
fn a_granular_drag_backwards_keeps_the_whole_line_it_started_on() {
    let mut document = Document::from_text("one\ntwo\nthree\nfour");
    document.click(10, ClickKind::Triple);
    document.extend_to(1, ClickKind::Triple);
    assert_eq!(document.selected_text(), Some("one\ntwo\nthree\n"));
}

#[test]
fn a_drag_lands_on_character_boundaries() {
    let mut document = Document::from_text("héllo wörld");
    document.click(1, ClickKind::Single);
    document.extend_to(3, ClickKind::Single);
    let selection = document.selection();
    assert!(document.text().is_char_boundary(selection.anchor()));
    assert!(document.text().is_char_boundary(selection.head()));
}

#[test]
fn a_click_clears_a_forced_column() {
    // A vertical move leaves the caret on the column it was aiming at; a click
    // puts the caret where it landed instead.
    let mut document = Document::from_text("one\ntwo");
    document.click(1, ClickKind::Single);
    document.move_caret(CaretMovement::Down);
    document.click(9, ClickKind::Single);
    assert_eq!(document.caret(), 7);
}

#[test]
fn the_selection_helpers_agree_with_the_document() {
    let text = "hello world";
    let mut document = Document::from_text(text);
    document.click(7, ClickKind::Double);
    assert_eq!(
        click_selection(&buffer(text), 7, ClickKind::Double),
        document.selection()
    );

    let mut dragged = Document::from_text(text);
    dragged.click(1, ClickKind::Double);
    dragged.extend_to(8, ClickKind::Double);
    assert_eq!(
        drag_selection(&buffer(text), 1, 8, ClickKind::Double),
        dragged.selection()
    );
}
