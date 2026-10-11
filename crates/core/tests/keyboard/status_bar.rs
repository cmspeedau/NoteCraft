//! NC-E010 — the line and column the status bar shows.
//!
//! The arithmetic from a byte offset to a 1-based line and column belongs to
//! the engine, not the window, so it can be tested without a window
//! (`docs/architecture.md` §4). What the status bar *says* — the wording and
//! which of its fields are shown — is a captured gap; these tests pin the
//! numbers, and nothing here decides how they are spelled.
//!
//! The caret is the end of the selection, so a status bar reading with text
//! selected shows where the selection ends.

use nc_core::Document;

#[test]
fn an_empty_document_reads_as_line_one_column_one() {
    let document = Document::new();
    let position = document.status_position();
    assert_eq!((position.line, position.column), (1, 1));
}

#[test]
fn the_start_of_the_text_is_line_one_column_one() {
    let document = Document::from_text("one two");
    assert_eq!(
        (
            document.status_position().line,
            document.status_position().column
        ),
        (1, 1)
    );
}

#[test]
fn the_column_counts_from_one() {
    let mut document = Document::from_text("one two");
    document.set_caret(3);
    let position = document.status_position();
    assert_eq!(
        (position.line, position.column),
        (1, 4),
        "three characters in"
    );
}

#[test]
fn the_line_counts_from_one_and_starts_after_a_line_ending() {
    let mut document = Document::from_text("one\ntwo");
    document.set_caret(4);
    let position = document.status_position();
    assert_eq!(
        (position.line, position.column),
        (2, 1),
        "the start of line 2"
    );

    document.set_caret(7);
    let position = document.status_position();
    assert_eq!(
        (position.line, position.column),
        (2, 4),
        "the end of line 2"
    );
}

#[test]
fn a_column_is_measured_in_characters_not_bytes() {
    let mut document = Document::from_text("naïve");
    // Byte 5 is where the `e` starts — four characters in, but six bytes in,
    // because `ï` is two bytes.
    document.set_caret(5);
    let position = document.status_position();
    assert_eq!(
        (position.line, position.column),
        (1, 5),
        "the column counts characters, not the bytes under them"
    );
    document.set_caret(document.text().len());
    assert_eq!(
        (
            document.status_position().line,
            document.status_position().column
        ),
        (1, 6),
        "five characters, so the end of the line is column 6"
    );
}

#[test]
fn a_character_outside_the_basic_plane_counts_as_one() {
    let mut document = Document::from_text("👍 ok");
    document.set_caret(4);
    let position = document.status_position();
    assert_eq!(position.column, 2, "one character, one column");
}

#[test]
fn the_column_restarts_at_each_line() {
    let mut document = Document::from_text("one\ntwo\nthree");
    assert_eq!(document.status_position().column, 1);
    document.set_caret(4);
    assert_eq!(
        (
            document.status_position().line,
            document.status_position().column
        ),
        (2, 1)
    );
    document.set_caret(8);
    assert_eq!(
        (
            document.status_position().line,
            document.status_position().column
        ),
        (3, 1)
    );
}

#[test]
fn a_document_ending_with_a_line_ending_has_an_empty_last_line() {
    let mut document = Document::from_text("one\n");
    document.set_caret(4);
    let position = document.status_position();
    assert_eq!((position.line, position.column), (2, 1));
}

#[test]
fn selected_text_reads_from_the_end_of_the_selection() {
    let mut document = Document::from_text("one\ntwo");
    document.select(0, 4);
    assert!(document.has_selection());
    let position = document.status_position();
    assert_eq!(
        (position.line, position.column),
        (2, 1),
        "the selection ends at the start of line 2"
    );
}

#[test]
fn the_reading_follows_the_caret_through_a_backwards_selection() {
    // `Document::select(anchor, head)` — the *head* is the point that moved, and
    // that is the caret the status bar reads, so dragging a selection upwards
    // moves the reading with the pointer rather than jumping to the far end.
    let mut document = Document::from_text("one\ntwo");
    document.select(7, 4);
    let position = document.status_position();
    assert_eq!(
        (position.line, position.column),
        (2, 1),
        "the caret was dragged back to the start of line 2"
    );

    document.select(4, 7);
    let position = document.status_position();
    assert_eq!(
        (position.line, position.column),
        (2, 4),
        "and it follows the caret forward again"
    );
}

#[test]
fn a_caret_past_the_end_is_clamped_to_the_document() {
    let mut document = Document::from_text("one\ntwo");
    document.set_caret(999);
    let position = document.status_position();
    assert_eq!((position.line, position.column), (2, 4));
}
