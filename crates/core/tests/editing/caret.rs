//! NC-E003 — Caret movement.
//!
//! `docs/feature-matrix.md` NC-E003. Every movement is a pure function, so the
//! arrow keys, Home/End, Ctrl+Arrow, and Page Up/Down are all pinned here
//! before any window exists.

use nc_core::{CaretMovement, Document, PAGE_LINES};

fn document(text: &str, caret: usize) -> Document {
    let mut document = Document::from_text(text);
    document.set_caret(caret);
    document
}

#[test]
fn left_and_right_step_by_a_column() {
    let mut document = document("abc", 1);
    document.move_caret(CaretMovement::Right);
    assert_eq!(document.caret(), 2);
    document.move_caret(CaretMovement::Left);
    assert_eq!(document.caret(), 1);
}

#[test]
fn right_at_the_end_of_a_line_crosses_the_terminator() {
    let mut document = document("abc\ndef", 3);
    document.move_caret(CaretMovement::Right);
    assert_eq!(document.caret(), 4);
    assert_eq!(document.caret_position().line, 1);
}

#[test]
fn left_at_the_start_of_a_line_lands_at_the_previous_line_end() {
    let mut document = document("abc\ndef", 4);
    document.move_caret(CaretMovement::Left);
    assert_eq!(document.caret(), 3);
    assert_eq!(document.caret_position().line, 0);
}

#[test]
fn movement_is_clamped_at_both_ends_of_the_document() {
    let mut document = document("abc", 0);
    document.move_caret(CaretMovement::Left);
    assert_eq!(document.caret(), 0);

    document.set_caret(3);
    document.move_caret(CaretMovement::Right);
    assert_eq!(document.caret(), 3);
}

#[test]
fn word_movement_skips_whitespace() {
    let mut document = document("foo bar baz", 0);
    document.move_caret(CaretMovement::WordRight);
    assert_eq!(document.caret(), 3);
    document.move_caret(CaretMovement::WordRight);
    assert_eq!(document.caret(), 7);

    document.set_caret(11);
    document.move_caret(CaretMovement::WordLeft);
    assert_eq!(document.caret(), 8);
    document.move_caret(CaretMovement::WordLeft);
    assert_eq!(document.caret(), 4);
    document.move_caret(CaretMovement::WordLeft);
    assert_eq!(document.caret(), 0);
}

#[test]
fn line_and_document_bounds() {
    let mut document = document("abc\ndef\nghi", 5);
    document.move_caret(CaretMovement::LineStart);
    assert_eq!(document.caret(), 4);
    document.move_caret(CaretMovement::LineEnd);
    assert_eq!(document.caret(), 7);
    document.move_caret(CaretMovement::DocumentStart);
    assert_eq!(document.caret(), 0);
    document.move_caret(CaretMovement::DocumentEnd);
    assert_eq!(document.caret(), 11);
}

#[test]
fn vertical_movement_remembers_the_column() {
    // Line 1 is shorter than the caret's column on line 0.
    let mut document = document("aaaa\nb\ncccc", 2);
    document.move_caret(CaretMovement::Down);
    assert_eq!(document.caret_position().line, 1);

    // Coming back down returns to the original column, not to column 1.
    document.move_caret(CaretMovement::Down);
    assert_eq!(
        document.caret_position(),
        nc_core::LineColumn { line: 2, column: 2 }
    );

    document.move_caret(CaretMovement::Up);
    document.move_caret(CaretMovement::Up);
    assert_eq!(
        document.caret_position(),
        nc_core::LineColumn { line: 0, column: 2 }
    );
}

#[test]
fn horizontal_movement_forgets_the_desired_column() {
    let mut document = document("aaaa\nb\ncccc", 2);
    document.move_caret(CaretMovement::Down);
    // Clamped to the short line, but the desired column is still 2.
    assert_eq!(
        document.caret_position(),
        nc_core::LineColumn { line: 1, column: 1 }
    );
    document.move_caret(CaretMovement::Right);
    assert_eq!(
        document.caret_position(),
        nc_core::LineColumn { line: 2, column: 0 }
    );
    // The horizontal move dropped the desired column, so Up uses the caret's
    // own column (0) rather than the 2 remembered two moves ago.
    document.move_caret(CaretMovement::Up);
    assert_eq!(
        document.caret_position(),
        nc_core::LineColumn { line: 1, column: 0 }
    );
    document.move_caret(CaretMovement::Up);
    assert_eq!(
        document.caret_position(),
        nc_core::LineColumn { line: 0, column: 0 }
    );
}

#[test]
fn up_at_the_first_line_and_down_at_the_last_are_no_ops() {
    let mut document = document("abc\ndef", 1);
    document.move_caret(CaretMovement::Up);
    assert_eq!(document.caret_position().line, 0);
    document.set_caret(5);
    document.move_caret(CaretMovement::Down);
    assert_eq!(document.caret_position().line, 1);
}

#[test]
fn page_movement_moves_by_a_page_and_stops_at_the_bounds() {
    let text = (0..25)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut document = Document::from_text(&text);
    document.set_caret(0);

    document.move_caret(CaretMovement::PageDown);
    assert_eq!(document.caret_position().line, PAGE_LINES);
    document.move_caret(CaretMovement::PageDown);
    assert_eq!(document.caret_position().line, PAGE_LINES * 2);
    // A third page runs into the last line rather than past it.
    document.move_caret(CaretMovement::PageDown);
    assert_eq!(document.caret_position().line, 24);
    document.move_caret(CaretMovement::PageUp);
    assert_eq!(document.caret_position().line, 24 - PAGE_LINES);
    document.move_caret(CaretMovement::PageUp);
    assert_eq!(document.caret_position().line, 24 - PAGE_LINES * 2);
    document.move_caret(CaretMovement::PageUp);
    assert_eq!(document.caret_position().line, 0);
}

#[test]
fn extending_keeps_the_anchor_and_moves_the_head() {
    let mut document = document("abc", 1);
    document.move_caret_extending(CaretMovement::Right);
    assert_eq!(document.selection().anchor(), 1);
    assert_eq!(document.selection().head(), 2);
    assert_eq!(document.selected_text(), Some("b"));

    document.move_caret_extending(CaretMovement::Left);
    assert_eq!(document.selection().anchor(), 1);
    assert_eq!(document.selection().head(), 1);
    assert!(!document.has_selection());
}

#[test]
fn a_plain_move_collapses_the_selection() {
    let mut document = document("abcdef", 0);
    document.select(0, 4);
    document.move_caret(CaretMovement::Down);
    assert!(!document.has_selection());
}
