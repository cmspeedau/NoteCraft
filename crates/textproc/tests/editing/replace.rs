//! NC-S003 — replacing, including Replace All.
//!
//! Replace All is `find_all` followed by one call to
//! `Document::replace_ranges`, so the whole run is a single undo step: undo
//! once and the document is exactly as it was, however many occurrences were
//! replaced. That is the reference's behaviour, and it is what makes a
//! mis-typed Replace All cheap to take back.
//!
//! Replacement text goes through the same path as typed and pasted text, so a
//! replacement containing CRLF or a lone CR is normalised to the buffer's LF
//! before it lands — the M4 rule that line endings are the document's, not the
//! input's.

use std::ops::Range;

use nc_core::Document;
use nc_textproc::{SearchOptions, find_all};

/// Replaces every match of `needle`, the way the shell's Replace All does.
fn replace_all(document: &mut Document, needle: &str, replacement: &str) -> usize {
    let matches = find_all(document.text(), needle, SearchOptions::new());
    let replacements: Vec<(Range<usize>, &str)> = matches
        .iter()
        .map(|found| (found.range(), replacement))
        .collect();
    document.replace_ranges(&replacements)
}

#[test]
fn replace_all_replaces_every_occurrence() {
    let mut document = Document::from_text("one two one two");
    let replaced = replace_all(&mut document, "one", "1");
    assert_eq!(replaced, 2);
    assert_eq!(document.text(), "1 two 1 two");
    assert!(document.is_dirty());
}

#[test]
fn replace_all_is_one_undo_step() {
    let mut document = Document::from_text("one two one two");
    replace_all(&mut document, "one", "1");
    assert_eq!(document.undo_depth(), 1, "one step, however many matches");
    assert!(document.undo());
    assert_eq!(document.text(), "one two one two");
    assert!(document.redo());
    assert_eq!(document.text(), "1 two 1 two");
}

#[test]
fn replace_all_copes_with_a_different_length_replacement() {
    let mut document = Document::from_text("one two one");
    let replaced = replace_all(&mut document, "one", "seventeen");
    assert_eq!(replaced, 2);
    assert_eq!(document.text(), "seventeen two seventeen");
    assert!(document.undo());
    assert_eq!(document.text(), "one two one");
}

#[test]
fn a_replacement_may_contain_the_needle_without_matching_itself() {
    // Every match is found before any of them is replaced, so a replacement
    // that contains the needle does not feed itself.
    let mut document = Document::from_text("aa aa");
    let replaced = replace_all(&mut document, "aa", "aaaa");
    assert_eq!(replaced, 2);
    assert_eq!(document.text(), "aaaa aaaa");
}

#[test]
fn replace_all_with_no_matches_is_not_an_edit() {
    let mut document = Document::from_text("one two one");
    let replaced = replace_all(&mut document, "three", "3");
    assert_eq!(replaced, 0);
    assert_eq!(document.text(), "one two one");
    assert!(!document.is_dirty(), "nothing changed, so nothing is dirty");
    assert!(!document.undo(), "and there is nothing to undo");
}

#[test]
fn an_empty_replacement_deletes_the_matches() {
    let mut document = Document::from_text("one two one");
    let replaced = replace_all(&mut document, "one", "");
    assert_eq!(replaced, 2);
    assert_eq!(document.text(), " two ");
    assert!(document.undo());
    assert_eq!(document.text(), "one two one");
}

#[test]
fn replace_all_keeps_the_options_it_was_given() {
    let mut document = Document::from_text("Cat cat");
    let matches = find_all(document.text(), "cat", SearchOptions::new().matching_case());
    let replacements: Vec<(Range<usize>, &str)> =
        matches.iter().map(|found| (found.range(), "dog")).collect();
    assert_eq!(document.replace_ranges(&replacements), 1);
    assert_eq!(document.text(), "Cat dog");
}

#[test]
fn replacing_a_span_moves_the_caret_past_the_replacement() {
    let mut document = Document::from_text("one two one");
    assert!(document.replace_range(4..7, "three"));
    assert_eq!(document.text(), "one three one");
    assert_eq!(document.caret(), 9, "the caret sits after the new text");
}

#[test]
fn a_replacement_with_other_line_endings_is_normalised() {
    let mut document = Document::from_text("one\ntwo");
    // The range covers the first line *and* its terminator, so the CRLF in the
    // replacement is what supplies the break.
    assert!(document.replace_range(0..4, "1\r\n"));
    assert_eq!(document.text(), "1\ntwo", "the document's terminators win");
}

#[test]
fn a_span_that_splits_a_character_is_widened_to_whole_characters() {
    let mut document = Document::from_text("naïve");
    // `ï` starts at byte 2 and is two bytes long, so byte 3 is inside it.
    assert!(document.replace_range(3..5, "i"));
    assert_eq!(document.text(), "naie");
}

#[test]
fn replace_all_then_find_finds_nothing_left_to_replace() {
    let mut document = Document::from_text("one two one two");
    replace_all(&mut document, "one", "1");
    let left = find_all(document.text(), "one", SearchOptions::new());
    assert!(left.is_empty(), "a second Replace All has nothing to do");
}
