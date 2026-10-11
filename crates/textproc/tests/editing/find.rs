//! NC-S001/NC-S002 — finding text.
//!
//! Find, Find next and Find previous are one question asked from the engine's
//! side: where is this text, travelling from the caret? The answer is a byte
//! range in the document, so it is handed to the caret, to Replace and to the
//! status bar without translation.
//!
//! The reference behaviour held to here: a match that *starts* at the caret
//! counts as the next one, a match that ends at the caret counts as the
//! previous one, and searching is case-insensitive until *Match case* is
//! switched on.

use nc_core::Document;
use nc_textproc::{SearchOptions, SearchResult, find_next, find_previous};

#[test]
fn find_next_moves_to_the_occurrence_after_the_caret() {
    let document = Document::from_text("one two one two");
    let text = document.text();
    let found = find_next(text, "one", 3, SearchOptions::new(), false);
    assert_eq!(
        found.matched().map(|found| found.text(text)),
        Some("one"),
        "the occurrence after the caret"
    );
    assert_eq!(found.matched().map(|found| found.range()), Some(8..11));
}

#[test]
fn a_match_starting_at_the_caret_counts_as_next() {
    let text = "one two one";
    let found = find_next(text, "one", 8, SearchOptions::new(), false);
    assert_eq!(found.matched().map(|found| found.range()), Some(8..11));
}

#[test]
fn a_match_ending_at_the_caret_counts_as_previous() {
    let text = "one two one";
    let found = find_previous(text, "one", 3, SearchOptions::new(), false);
    assert_eq!(found.matched().map(|found| found.range()), Some(0..3));
}

#[test]
fn find_previous_moves_backwards() {
    let text = "one two one two";
    let found = find_previous(text, "two", text.len(), SearchOptions::new(), false);
    assert_eq!(found.matched().map(|found| found.range()), Some(12..15));
    let found = find_previous(text, "two", 8, SearchOptions::new(), false);
    assert_eq!(found.matched().map(|found| found.range()), Some(4..7));
}

#[test]
fn searching_is_case_insensitive_by_default() {
    let text = "Alpha alpha";
    let found = find_next(text, "ALPHA", 0, SearchOptions::new(), false);
    assert_eq!(found.matched().map(|found| found.range()), Some(0..5));
}

#[test]
fn a_needle_can_span_a_line_ending() {
    let document = Document::from_text("one\ntwo");
    let text = document.text();
    let found = find_next(text, "e\nt", 0, SearchOptions::new(), false);
    assert_eq!(found.matched().map(|found| found.range()), Some(2..5));
    // The match is a real slice of the document, terminators and all.
    assert_eq!(found.matched().map(|found| found.text(text)), Some("e\nt"));
}

#[test]
fn text_that_is_not_there_is_reported_as_no_match() {
    let text = "one two one";
    assert_eq!(
        find_next(text, "three", 0, SearchOptions::new(), true),
        SearchResult::NoMatch
    );
    assert_eq!(
        find_previous(text, "three", text.len(), SearchOptions::new(), true),
        SearchResult::NoMatch
    );
}

#[test]
fn nothing_is_found_in_an_empty_document() {
    let document = Document::new();
    assert_eq!(
        find_next(document.text(), "one", 0, SearchOptions::new(), true),
        SearchResult::NoMatch
    );
}

#[test]
fn a_needle_longer_than_the_document_is_not_found() {
    assert_eq!(
        find_next("one", "one two", 0, SearchOptions::new(), true),
        SearchResult::NoMatch
    );
}

#[test]
fn an_empty_needle_is_not_found_anywhere() {
    // The reference disables Find until something is typed.
    let text = "one two one";
    assert_eq!(
        find_next(text, "", 0, SearchOptions::new(), true),
        SearchResult::NoMatch
    );
}

#[test]
fn a_caret_past_the_end_searches_from_the_end() {
    let text = "one two one";
    let found = find_next(text, "one", text.len() + 40, SearchOptions::new(), true);
    assert_eq!(found.matched().map(|found| found.range()), Some(0..3));
}

#[test]
fn a_caret_inside_a_character_searches_from_a_boundary() {
    let text = "naïve plain";
    // Byte 3 is inside the `ï`; the search still answers with a whole-character
    // range, and never panics on a caret that is not a character boundary.
    let found = find_next(text, "plain", 3, SearchOptions::new(), false);
    assert_eq!(found.matched().map(|found| found.range()), Some(7..12));
    assert_eq!(found.matched().map(|found| found.text(text)), Some("plain"));
}
