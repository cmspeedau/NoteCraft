//! NC-S004 — the Find options.
//!
//! The reference's Find dialog carries two switches and a direction: *Match
//! case*, *Match whole word only*, and up or down. Both switches start off.
//! These tests pin what each one does to a search, and — because a search folds
//! case to compare it — that switching them on never touches the document.

use nc_textproc::{SearchOptions, SearchResult, find_all, find_next};

#[test]
fn match_case_off_finds_either_case() {
    let text = "Alpha beta alpha";
    let found = find_all(text, "alpha", SearchOptions::new());
    assert_eq!(found.len(), 2, "both spellings match");
    assert_eq!(found[0].range(), 0..5);
    assert_eq!(found[1].range(), 11..16);
}

#[test]
fn match_case_on_finds_only_that_case() {
    let text = "Alpha beta alpha";
    let options = SearchOptions::new().matching_case();
    let found = find_all(text, "alpha", options);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text(text), "alpha");
    assert_eq!(
        find_next(text, "ALPHA", 0, options, true),
        SearchResult::NoMatch
    );
}

#[test]
fn whole_word_only_skips_matches_inside_words() {
    let text = "cat catalogue cat";
    let options = SearchOptions::new().whole_words();
    let found = find_all(text, "cat", options);
    assert_eq!(found.len(), 2, "`catalogue` is not a whole word match");
    assert_eq!(found[0].range(), 0..3);
    assert_eq!(found[1].range(), 14..17);
}

#[test]
fn whole_word_only_matches_at_the_edges_of_the_text() {
    let text = "cat";
    let options = SearchOptions::new().whole_words();
    assert_eq!(
        find_next(text, "cat", 0, options, false)
            .matched()
            .map(|found| found.range()),
        Some(0..3)
    );
}

#[test]
fn a_word_ends_at_punctuation_and_at_a_line_ending() {
    let text = "a cat, cat\ndog";
    let options = SearchOptions::new().whole_words();
    let found = find_all(text, "cat", options);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].range(), 2..5);
    assert_eq!(found[1].range(), 7..10);
}

#[test]
fn an_underscore_is_part_of_a_word() {
    let text = "cat cat_ cat";
    let options = SearchOptions::new().whole_words();
    let found = find_all(text, "cat", options);
    assert_eq!(found.len(), 2, "`cat_` is one word");
    assert_eq!(found[0].range(), 0..3);
    assert_eq!(found[1].range(), 9..12);
}

#[test]
fn a_needle_that_is_not_a_whole_word_anywhere_is_not_found() {
    let text = "catalogue";
    let options = SearchOptions::new().whole_words();
    assert_eq!(find_all(text, "cat", options), Vec::new());
}

#[test]
fn the_two_switches_work_together() {
    let text = "cat CAT catalogue cat";
    let options = SearchOptions::new().whole_words().matching_case();
    let found = find_all(text, "cat", options);
    assert_eq!(found.len(), 2, "whole words, lower case only");
    assert_eq!(found[0].range(), 0..3);
    assert_eq!(found[1].range(), 18..21, "the last word, after catalogue");
}

#[test]
fn an_empty_needle_finds_nothing() {
    let text = "one two one";
    assert_eq!(find_all(text, "", SearchOptions::new()), Vec::new());
}

#[test]
fn folding_case_covers_letters_outside_ascii() {
    let text = "Résumé RÉSUMÉ";
    let lowercase = find_all(text, "résumé", SearchOptions::new());
    assert_eq!(lowercase.len(), 2, "either case, whatever the accents");
    assert!(
        lowercase
            .iter()
            .all(|found| found.text(text) == "Résumé" || found.text(text) == "RÉSUMÉ")
    );

    let uppercase = find_all(text, "RÉSUMÉ", SearchOptions::new());
    assert_eq!(uppercase.len(), 2);
    assert_eq!(uppercase[0].range(), 0..8, "byte offsets, not characters");
}

#[test]
fn folding_case_covers_greek_sigma() {
    // Folding maps a capital sigma to σ wherever it stands — it is not the
    // context-sensitive final-sigma rule — so the sigma at the end of a word is
    // found by σ and not by ς.
    let text = "ΟΔΟΣ";
    assert_eq!(find_all(text, "σ", SearchOptions::new()).len(), 1);
    assert_eq!(find_all(text, "ς", SearchOptions::new()), Vec::new());
    assert_eq!(
        find_all(text, "οδοσ", SearchOptions::new())[0].range(),
        0..8
    );
}

#[test]
fn searching_never_changes_the_document() {
    let document = nc_core::Document::from_text("Résumé résumé");
    let before = document.text().to_owned();
    let _ = find_all(document.text(), "RÉSUMÉ", SearchOptions::new());
    assert_eq!(document.text(), before);
    assert!(!document.is_dirty(), "a search is not an edit");
}

#[test]
fn every_match_can_be_read_back_out_of_the_document() {
    let document = nc_core::Document::from_text("one two one two");
    let text = document.text();
    for found in find_all(text, "two", SearchOptions::new()) {
        assert!(text.is_char_boundary(found.start));
        assert!(text.is_char_boundary(found.end));
        assert_eq!(found.text(text), "two");
    }
}

#[test]
fn matches_do_not_overlap() {
    let text = "aaa";
    let found = find_all(text, "aa", SearchOptions::new());
    assert_eq!(found.len(), 1, "found once from the left, not twice");
    assert_eq!(found[0].range(), 0..2);
}

#[test]
fn the_defaults_are_off_and_forward() {
    let options = SearchOptions::default();
    assert!(!options.match_case);
    assert!(!options.whole_word);
    assert_eq!(options, SearchOptions::new());
}
