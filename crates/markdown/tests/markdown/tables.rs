//! `NC-M008` Tables.
//!
//! Acceptance: **"Insert/expand semantics"** — a written table is a header row, a
//! separator row, and the requested body rows, in GitHub's pipe syntax, with every
//! row carrying the same number of cells and the caret left where the user has to
//! type next.

use nc_markdown::{BlockKind, Range, blocks, insert_table};

/// The whole document, as a user selecting everything would give it.
fn all(text: &str) -> Range {
    Range::new(0, text.len()).unwrap()
}

#[test]
fn a_table_is_a_header_a_separator_and_the_body_rows() {
    let edited = insert_table("", Range::caret(0), 2, 2).unwrap();
    assert_eq!(
        edited.text, "|   |   |\n| --- | --- |\n|   |   |\n|   |   |",
        "header, separator, then two body rows"
    );
}

#[test]
fn one_column_and_no_body_rows_is_still_a_valid_table() {
    let edited = insert_table("", Range::caret(0), 1, 0).unwrap();
    assert_eq!(edited.text, "|   |\n| --- |");
}

#[test]
fn every_row_carries_the_same_number_of_cells() {
    for columns in 1..=5 {
        let edited = insert_table("", Range::caret(0), columns, 3).unwrap();
        let expected = columns + 1; // one pipe per cell, plus the one that ends the row
        for line in edited.text.lines() {
            let pipes = line.chars().filter(|character| *character == '|').count();
            assert_eq!(pipes, expected, "{columns} columns, row {line:?}");
        }
    }
}

#[test]
fn the_separator_row_is_the_second_row() {
    // GFM only reads a pipe table when row two is the delimiter row.
    let edited = insert_table("", Range::caret(0), 3, 1).unwrap();
    let second = edited.text.lines().nth(1).expect("a second row");
    assert_eq!(second, "| --- | --- | --- |");
}

#[test]
fn the_caret_is_left_in_the_first_header_cell_to_type_in() {
    let edited = insert_table("", Range::caret(0), 2, 1).unwrap();
    assert!(edited.selection.is_empty(), "a caret, not a selection");
    assert!(edited.text.starts_with('|'), "the table starts with a pipe");
    let cell = edited.selection.start;
    assert!(cell > 0 && cell < 4, "in the first cell, got {cell}");
    assert_eq!(&edited.text[0..1], "|");
}

#[test]
fn the_table_replaces_whatever_was_selected() {
    let text = "start MIDDLE end";
    let middle = text.find("MIDDLE").unwrap();
    let selection = Range::new(middle, middle + "MIDDLE".len()).unwrap();

    let edited = insert_table(text, selection, 2, 0).unwrap();

    assert_eq!(edited.text, "start |   |   |\n| --- | --- | end");
}

#[test]
fn a_written_table_is_read_back_as_table_rows() {
    let edited = insert_table("", Range::caret(0), 2, 1).unwrap();
    let read = blocks(&edited.text);
    assert!(
        read.iter().all(|block| block.kind == BlockKind::TableRow),
        "{:?}",
        read.iter().map(|block| block.kind).collect::<Vec<_>>()
    );
    assert_eq!(read.len(), 3, "header, separator, body");
}

#[test]
fn a_table_with_no_columns_is_refused_rather_than_written_empty() {
    assert!(insert_table("", Range::caret(0), 0, 1).is_none());
}

#[test]
fn an_absurd_size_is_refused_rather_than_attempted() {
    assert!(insert_table("", Range::caret(0), 1000, 1).is_none());
    assert!(insert_table("", Range::caret(0), 2, 1000).is_none());
}

#[test]
fn inserting_a_table_leaves_the_rest_of_the_document_alone() {
    let text = "before\n\nafter\n";
    let after = text.find("after").unwrap();
    let edited = insert_table(text, Range::caret(after), 1, 0).unwrap();
    assert!(edited.text.starts_with("before\n\n"), "the front is intact");
    assert!(edited.text.ends_with('\n'), "the end is intact");
    assert!(all(&edited.text).end == edited.text.len());
}
