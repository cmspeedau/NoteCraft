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

// ---- The other half of the criterion: expanding a table that is already there --
//
// One command does both jobs, so the document decides. The capture has no dialog
// asking for a size or a direction and the registry has one table command, so
// "expand" is what the keystroke means when it lands inside a table
// (`docs/ui-reference.md` §12).

use nc_markdown::expand_table;

/// Header, separator, one body row.
const TABLE: &str = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";

#[test]
fn a_caret_in_a_body_row_adds_a_row_below_it() {
    let on_two = TABLE.find("| 1 |").unwrap();
    let edited = expand_table(TABLE, Range::caret(on_two + 2)).unwrap();

    assert_eq!(
        edited.text, "| a | b |\n| --- | --- |\n| 1 | 2 |\n|   |   |\n",
        "the new row goes under the caret's row, not at the end of the file"
    );
}

#[test]
fn a_caret_in_the_header_or_the_separator_adds_the_row_below_the_separator() {
    // A row between a header and its delimiter is not a row, it is a broken table:
    // GFM reads row two as the delimiter and everything after it as the body.
    for (label, needle) in [("header", "| a |"), ("separator", "| --- |")] {
        let mut text = TABLE.to_owned();
        let caret = text.find(needle).unwrap() + 2;
        let edited = expand_table(&text, Range::caret(caret)).unwrap();
        let rows: Vec<&str> = edited.text.lines().collect();
        assert_eq!(
            rows[1], "| --- | --- |",
            "{label}: row two stays the delimiter"
        );
        assert_eq!(
            rows[2], "|   |   |",
            "{label}: the new row is the first body row"
        );
        assert_eq!(rows[3], "| 1 | 2 |", "{label}: the old body row moves down");
        text.clear();
    }
}

#[test]
fn the_added_row_matches_the_widest_row_of_the_table() {
    // Ragged on purpose: the command lengthens the table, it does not resize it.
    let text = "| a | b | c |\n| --- | --- | --- |\n| 1 |\n";
    let on_short = text.find("| 1 |").unwrap();
    let edited = expand_table(text, Range::caret(on_short)).unwrap();

    assert!(
        edited.text.ends_with("| 1 |\n|   |   |   |\n"),
        "{:?}",
        edited.text
    );
    assert!(
        edited.text.contains("| 1 |\n"),
        "the short row is left short — nothing was padded"
    );
}

#[test]
fn the_caret_lands_in_the_new_rows_first_cell() {
    // The same place `insert_table` leaves it — one pipe and a space into the row —
    // so typing goes on filling the table whichever way the row arrived.
    let on_two = TABLE.find("| 1 |").unwrap();
    let edited = expand_table(TABLE, Range::caret(on_two)).unwrap();
    assert!(edited.selection.is_empty(), "a caret, not a selection");

    let caret = edited.selection.start;
    let row_start = edited.text[..caret]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    assert!(
        edited.text[row_start..].starts_with("|   |   |"),
        "the caret sits in the row the command just added: {:?}",
        &edited.text[row_start..]
    );
    assert_eq!(&edited.text[row_start..caret], "| ");
}

#[test]
fn growing_a_table_leaves_every_other_line_alone() {
    let text = format!("before\n\n{TABLE}\nafter\n");
    let on_two = text.find("| 1 |").unwrap();
    let edited = expand_table(&text, Range::caret(on_two)).unwrap();

    assert!(edited.text.starts_with("before\n\n"), "the front is intact");
    assert!(edited.text.ends_with("\nafter\n"), "and the back");
    assert!(
        edited.text.contains("| a | b |\n| --- | --- |\n"),
        "and the rows above are byte for byte what they were"
    );
}

#[test]
fn a_grown_table_still_reads_as_a_table() {
    let on_two = TABLE.find("| 1 |").unwrap();
    let edited = expand_table(TABLE, Range::caret(on_two)).unwrap();
    let kinds: Vec<BlockKind> = blocks(&edited.text)
        .iter()
        .map(|block| block.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            BlockKind::TableRow,
            BlockKind::TableRow,
            BlockKind::TableRow,
            BlockKind::TableRow,
        ],
        "every line is still read as a row of one table"
    );
}

#[test]
fn a_one_row_table_grows_under_itself() {
    let edited = expand_table("| only |\n", Range::caret(2)).unwrap();
    assert_eq!(edited.text, "| only |\n|   |\n");
}

#[test]
fn outside_a_table_there_is_nothing_to_grow() {
    // This `None` is what makes the shell write a new table instead.
    assert!(expand_table("prose only\n", Range::caret(3)).is_none());
    assert!(expand_table("", Range::caret(0)).is_none());
}

#[test]
fn a_line_that_merely_starts_with_a_pipe_is_not_a_table_yet() {
    // The parser needs a pipe at each end, and the command agrees with it rather
    // than keeping its own definition of a table.
    assert!(expand_table("| no closing pipe\n", Range::caret(2)).is_none());
}

#[test]
fn a_selection_anywhere_in_the_table_grows_it() {
    let whole = all(TABLE);
    let edited = expand_table(TABLE, whole).unwrap();
    assert_eq!(
        edited.text.lines().count(),
        TABLE.lines().count() + 1,
        "one row added"
    );
}
