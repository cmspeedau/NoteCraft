//! `NC-M003` Bold / Italic (`Ctrl+B` / `Ctrl+I`) and `NC-M004` Strikethrough.
//!
//! Acceptance: **"Serialization exact"** for bold and italic, and **"`~~`
//! serialization"** for strikethrough. "Exact" is taken seriously here: each test
//! compares whole strings, so a stray space or a doubled marker fails the build.

use nc_markdown::{Inline, Range, toggle_inline};

/// The selection covering `needle`, which is how these tests say what the user
/// selected without hard-coding byte offsets.
fn selection_of(haystack: &str, needle: &str) -> Range {
    let start = haystack
        .find(needle)
        .expect("the selection is present in the text");
    Range::new(start, start + needle.len()).expect("whole characters")
}

#[test]
fn bold_writes_exactly_two_asterisks() {
    let text = "press the button";
    let edited = toggle_inline(text, selection_of(text, "button"), Inline::Bold).unwrap();
    assert_eq!(edited.text, "press the **button**");
}

#[test]
fn italic_writes_exactly_one_asterisk() {
    let text = "make it so";
    let edited = toggle_inline(text, selection_of(text, "so"), Inline::Italic).unwrap();
    assert_eq!(edited.text, "make it *so*");
}

#[test]
fn strikethrough_writes_exactly_two_tildes() {
    let text = "keep the old value";
    let edited = toggle_inline(text, selection_of(text, "old value"), Inline::Strike).unwrap();
    assert_eq!(edited.text, "keep the ~~old value~~");
}

#[test]
fn the_selection_carries_the_markers_so_one_more_press_undoes_it() {
    let text = "press the button";
    let once = toggle_inline(text, selection_of(text, "button"), Inline::Bold).unwrap();
    assert_eq!(once.selection, selection_of(&once.text, "**button**"));

    let twice = toggle_inline(&once.text, once.selection, Inline::Bold).unwrap();
    assert_eq!(
        twice.text, text,
        "wrapping and unwrapping cost one press each"
    );
    assert_eq!(twice.selection, selection_of(text, "button"));
}

#[test]
fn selecting_the_words_of_an_already_marked_run_unmarks_them() {
    let text = "press the **button** now";
    let edited = toggle_inline(text, selection_of(text, "button"), Inline::Bold).unwrap();

    let expected = "press the button now";
    assert_eq!(edited.text, expected);
    assert_eq!(edited.selection, selection_of(expected, "button"));
}

#[test]
fn a_caret_gets_both_markers_and_room_to_type_between_them() {
    let text = "press the button";
    let inside_word = text.find("button").unwrap() + 3;

    let edited = toggle_inline(text, Range::caret(inside_word), Inline::Bold).unwrap();

    assert_eq!(edited.text, "press the but****ton");
    assert!(edited.selection.is_empty(), "a caret stays a caret");
    let at = edited.selection.start;
    assert_eq!(&edited.text[at - 2..at], "**", "the caret sits inside");
    assert_eq!(&edited.text[at..at + 2], "**", "the markers, not before");
}

#[test]
fn a_selection_spanning_multiple_bytes_per_character_is_wrapped_at_its_edges() {
    let text = "cafés and naïve résumé";
    let edited = toggle_inline(text, selection_of(text, "naïve"), Inline::Bold).unwrap();
    assert_eq!(edited.text, "cafés and **naïve** résumé");
}

#[test]
fn a_selection_that_splits_a_character_does_nothing_at_all() {
    let text = "naïve";
    let continuation_byte = text.find('ï').unwrap() + 1;
    assert!(
        toggle_inline(text, Range::caret(continuation_byte), Inline::Bold).is_none(),
        "a range that is not a character boundary must not produce a partial edit"
    );
}

#[test]
fn markers_are_added_where_asked_and_nothing_else_is_touched() {
    // The commands are deliberately tolerant rather than clever: they add their
    // own bytes and leave the rest of the line exactly as it was, even where that
    // produces nesting a renderer would resolve its own way.
    let text = "# Title with a *star* here";
    let edited = toggle_inline(text, selection_of(text, "a *star*"), Inline::Italic).unwrap();
    assert_eq!(edited.text, "# Title with *a *star** here");
}

#[test]
fn a_selection_at_the_very_start_and_end_of_the_text_works() {
    let text = "whole";
    let edited = toggle_inline(text, Range::new(0, text.len()).unwrap(), Inline::Bold).unwrap();
    assert_eq!(edited.text, "**whole**");
    assert_eq!(edited.selection, Range::new(0, text.len() + 4).unwrap());
}
