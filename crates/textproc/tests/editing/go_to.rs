//! NC-S006 — *Go to line*.
//!
//! The dialog takes a line number and the caret goes to the start of that
//! line. Two halves, tested apart: `parse_line_number` reads what was typed,
//! and `go_to_line` turns a line number into an offset. Neither moves a caret,
//! so the shell decides what to do with `None` (leave the dialog alone) and
//! what to do with an offset (put the caret there) — and the whole feature can
//! be tested without a dialog.
//!
//! Lines are counted from 1, the same count the status bar shows
//! (NC-E010), so the number the user reads off the status bar is the number
//! they can type into the dialog.

use nc_textproc::search::go_to_line as _go_to_line_through_the_module;
use nc_textproc::{go_to_line, parse_line_number};

#[test]
fn the_first_line_starts_at_the_start_of_the_text() {
    let text = "one\ntwo\nthree";
    assert_eq!(go_to_line(text, 1), 0);
}

#[test]
fn each_line_starts_after_the_line_ending_before_it() {
    let text = "one\ntwo\nthree";
    assert_eq!(go_to_line(text, 2), 4);
    assert_eq!(go_to_line(text, 3), 8);
}

#[test]
fn a_line_no_reaches_the_end_of_the_text() {
    let text = "one\ntwo\nthree";
    assert_eq!(go_to_line(text, 99), 8, "the start of the last line");
}

#[test]
fn line_zero_clamps_to_the_start() {
    let text = "one\ntwo";
    assert_eq!(go_to_line(text, 0), 0, "clamped, not a failure");
}

#[test]
fn text_without_line_endings_is_one_line() {
    let text = "one two three";
    assert_eq!(go_to_line(text, 1), 0);
    assert_eq!(go_to_line(text, 2), 0, "there is no second line to reach");
}

#[test]
fn an_empty_document_has_one_line() {
    assert_eq!(go_to_line("", 1), 0);
    assert_eq!(go_to_line("", 4), 0);
}

#[test]
fn a_trailing_line_ending_leaves_an_empty_last_line() {
    let text = "one\n";
    assert_eq!(go_to_line(text, 2), 4);
    assert_eq!(go_to_line(text, 2), text.len());
}

#[test]
fn a_line_ending_written_as_crlf_points_past_the_whole_ending() {
    // The engine normalises CRLF to LF as the file is read, so by the time a
    // line number is resolved there is only one terminator to step over.
    let document = nc_core::Document::from_text("one\r\ntwo\r\nthree");
    let text = document.text();
    assert_eq!(go_to_line(text, 2), 4);
    assert_eq!(go_to_line(text, 3), 8);
}

#[test]
fn the_caret_lands_somewhere_a_line_starts() {
    let document = nc_core::Document::from_text("alpha\nbeta\ngamma");
    let text = document.text().to_owned();
    for line in 1..=3 {
        let offset = go_to_line(&text, line);
        assert!(text.is_char_boundary(offset));
        assert!(
            offset == 0 || text[..offset].ends_with('\n'),
            "line {line} begins a line"
        );
    }
}

#[test]
fn a_line_number_is_the_digits_of_one() {
    assert_eq!(parse_line_number("12"), Some(12));
    assert_eq!(parse_line_number("1"), Some(1));
    assert_eq!(
        parse_line_number("0"),
        Some(0),
        "the clamp is go_to_line's job"
    );
}

#[test]
fn surrounding_spaces_are_ignored() {
    assert_eq!(parse_line_number("  12 "), Some(12));
}

#[test]
fn anything_that_is_not_a_line_number_is_nothing() {
    assert_eq!(parse_line_number(""), None, "nothing typed yet");
    assert_eq!(parse_line_number("   "), None);
    assert_eq!(parse_line_number("twelve"), None);
    assert_eq!(parse_line_number("12a"), None);
    assert_eq!(parse_line_number("-3"), None, "there are no negative lines");
    assert_eq!(parse_line_number("1 2"), None);
    assert_eq!(parse_line_number("1.5"), None);
}

#[test]
fn a_line_number_too_large_for_the_machine_is_nothing() {
    // The reference shows the dialog as typed rather than moving the caret.
    assert_eq!(parse_line_number("99999999999999999999999"), None);
}

#[test]
fn a_typed_line_number_reaches_its_line() {
    let text = "one\ntwo\nthree";
    let offset = parse_line_number(" 3 ").map(|line| go_to_line(text, line));
    assert_eq!(offset, Some(8));
}

#[test]
fn go_to_line_is_reachable_by_its_module_path() {
    assert_eq!(_go_to_line_through_the_module("a\nb", 2), 2);
}

#[test]
fn the_line_the_status_bar_shows_is_the_line_the_dialog_takes() {
    // NC-E010 on one side and NC-S006 on the other: reading a line off the
    // status bar and typing it into the dialog reaches that line.
    let mut document = nc_core::Document::from_text("alpha\nbeta\ngamma");
    document.set_caret(11);
    let line = document.status_position().line;
    assert_eq!(line, 3);
    assert_eq!(go_to_line(document.text(), line), 11);
}
