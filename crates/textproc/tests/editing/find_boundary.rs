//! NC-S005 — what happens at the ends of the document.
//!
//! Find wraps: reaching the end (or the start, travelling backwards) does not
//! end the search, it continues from the other end. The reference does not do
//! this silently — it stops first and says the end of the document was
//! reached, and only carries on when asked — so a search has two ways of
//! travelling: `wraps` true carries on and reports `Wrapped`, and `wraps`
//! false stops and reports `Boundary`.
//!
//! The distinction matters to the shell: `Boundary` means "there is text to
//! find, but not this way", while `NoMatch` means "the text is not in this
//! document at all", and the reference words the two notices differently.

use nc_textproc::{Match, SearchOptions, SearchResult, find_next, find_previous};

#[test]
fn wrapping_from_the_end_finds_the_first_match() {
    let text = "one two three";
    let found = find_next(text, "one", 1, SearchOptions::new(), true);
    assert_eq!(found, SearchResult::Wrapped(Match::new(0, 3)));
    assert!(found.found());
}

#[test]
fn without_wrapping_the_search_stops_at_the_end() {
    let text = "one two three";
    let found = find_next(text, "one", 1, SearchOptions::new(), false);
    assert_eq!(found, SearchResult::Boundary);
    assert_eq!(found.matched(), None, "a notice is not a match");
}

#[test]
fn wrapping_backwards_from_the_start_finds_the_last_match() {
    let text = "one two one";
    let found = find_previous(text, "one", 0, SearchOptions::new(), true);
    assert_eq!(found, SearchResult::Wrapped(Match::new(8, 11)));
}

#[test]
fn without_wrapping_a_backward_search_stops_at_the_start() {
    let text = "one two one";
    assert_eq!(
        find_previous(text, "one", 0, SearchOptions::new(), false),
        SearchResult::Boundary
    );
}

#[test]
fn the_only_match_in_the_document_wraps_onto_itself() {
    // One occurrence: wrapping from just past it comes back to it, and the
    // shell can then say the whole document was searched.
    let text = "one two three";
    let found = find_next(text, "one", 1, SearchOptions::new(), true);
    assert_eq!(found, SearchResult::Wrapped(Match::new(0, 3)));
}

#[test]
fn a_search_that_finds_nothing_still_says_no_match_when_wrapping() {
    let text = "one two one";
    assert_eq!(
        find_next(text, "three", 0, SearchOptions::new(), true),
        SearchResult::NoMatch,
        "wrapping cannot conjure text that is not there"
    );
}

#[test]
fn a_search_at_the_very_end_of_the_document_wraps() {
    let text = "one two one";
    let found = find_next(text, "one", text.len(), SearchOptions::new(), true);
    assert_eq!(found, SearchResult::Wrapped(Match::new(0, 3)));
}

#[test]
fn a_backward_search_at_the_very_end_finds_the_last_match() {
    let text = "one two one";
    let found = find_previous(text, "one", text.len(), SearchOptions::new(), false);
    assert_eq!(found, SearchResult::Found(Match::new(8, 11)));
}

#[test]
fn the_options_are_honoured_while_wrapping() {
    let text = "One one";
    let options = SearchOptions::new().matching_case();
    let found = find_next(text, "One", 1, options, true);
    assert_eq!(found, SearchResult::Wrapped(Match::new(0, 3)));
    let found = find_next(text, "one", 1, options, true);
    assert_eq!(found, SearchResult::Found(Match::new(4, 7)));
}
