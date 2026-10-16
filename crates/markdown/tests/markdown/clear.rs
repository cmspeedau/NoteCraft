//! `NC-M009` Clear formatting (optional).
//!
//! Acceptance: **"Result"** — the command is defined by what is left behind. The
//! tests assert the exact remaining text, including the things that must *not* be
//! taken away: underscores, and a marker with nothing to pair with.

use nc_markdown::{Range, clear_formatting};

/// The whole document, as a user selecting everything would give it.
fn all(text: &str) -> Range {
    Range::new(0, text.len()).unwrap()
}

/// The selection covering `needle`.
fn selection_of(haystack: &str, needle: &str) -> Range {
    let start = haystack
        .find(needle)
        .expect("the selection is present in the text");
    Range::new(start, start + needle.len()).expect("whole characters")
}

#[test]
fn emphasis_markers_go_and_the_words_stay() {
    for (marked, expected) in [
        ("**important** news", "important news"),
        ("*quick* news", "quick news"),
        ("~~deleted~~ news", "deleted news"),
    ] {
        let cleared = clear_formatting(marked, all(marked)).unwrap();
        assert_eq!(cleared.text, expected, "clearing {marked:?}");
    }
}

#[test]
fn all_three_kinds_go_from_one_line_at_once() {
    let text = "*one* and ~~two~~ and **three**";
    let cleared = clear_formatting(text, all(text)).unwrap();
    assert_eq!(cleared.text, "one and two and three");
}

#[test]
fn block_markers_go_with_the_indentation_in_front_of_them() {
    for (marked, expected) in [
        ("## Heading", "Heading"),
        ("  ## Heading", "Heading"),
        ("- item", "item"),
        ("  - item", "item"),
        ("3. item", "item"),
        ("> quoted", "quoted"),
    ] {
        let cleared = clear_formatting(marked, all(marked)).unwrap();
        assert_eq!(cleared.text, expected, "clearing {marked:?}");
    }
}

#[test]
fn every_line_a_selection_touches_is_cleared() {
    let text = "# one\n- two\nplain\n";
    let cleared = clear_formatting(text, all(text)).unwrap();
    assert_eq!(cleared.text, "one\ntwo\nplain\n");
}

#[test]
fn an_underscore_is_not_formatting_and_survives() {
    let text = "snake_case_word and snake_case_thing";
    let cleared = clear_formatting(text, all(text)).unwrap();
    assert_eq!(cleared.text, text, "removing them would corrupt the text");
}

#[test]
fn an_asterisk_with_nothing_to_pair_with_is_the_users_own() {
    let text = "2 * 3 = 6";
    let cleared = clear_formatting(text, all(text)).unwrap();
    assert_eq!(cleared.text, text);
}

#[test]
fn text_that_carries_no_formatting_is_returned_byte_for_byte() {
    let text = "Ordinary prose, with a comma.\n";
    let cleared = clear_formatting(text, all(text)).unwrap();
    assert_eq!(cleared.text, text);
    assert_eq!(
        cleared.selection,
        all(text),
        "and the selection is untouched"
    );
}

#[test]
fn clearing_a_line_with_no_marker_leaves_its_indentation_alone() {
    // Indentation in prose with no list or heading above it is the author's, so
    // there is nothing here that reads as formatting.
    let text = "    an indented paragraph";
    let cleared = clear_formatting(text, all(text)).unwrap();
    assert_eq!(cleared.text, text);
}

#[test]
fn only_the_selection_is_cleared() {
    let text = "**one** and **two**";
    let cleared = clear_formatting(text, selection_of(text, "**one**")).unwrap();
    assert_eq!(cleared.text, "one and **two**");
}

#[test]
fn a_cleared_line_can_be_formatted_again() {
    use nc_markdown::{Heading, set_heading};
    let text = "## Heading";
    let cleared = clear_formatting(text, all(text)).unwrap();
    let reheaded = set_heading(&cleared.text, all(&cleared.text), Heading::Level(3)).unwrap();
    assert_eq!(reheaded.text, "### Heading");
}
