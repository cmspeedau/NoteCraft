//! `NC-M007` Hyperlinks (`Ctrl+K`).
//!
//! Acceptance: **"Link syntax exact"**. The tests compare the whole written line,
//! so the brackets, the parenthesis and the target are each pinned down.

use nc_markdown::{Range, SpanKind, insert_link, spans};

/// The selection covering `needle`.
fn selection_of(haystack: &str, needle: &str) -> Range {
    let start = haystack
        .find(needle)
        .expect("the selection is present in the text");
    Range::new(start, start + needle.len()).expect("whole characters")
}

#[test]
fn a_label_and_a_target_are_written_in_link_syntax() {
    let text = "see the docs for details";
    let edited = insert_link(text, selection_of(text, "the docs"), "https://example.com").unwrap();

    assert_eq!(
        edited.text,
        "see [the docs](https://example.com) for details"
    );
}

#[test]
fn the_selection_ends_up_covering_the_whole_link() {
    let text = "see the docs";
    let edited = insert_link(text, selection_of(text, "the docs"), "https://example.com").unwrap();

    let written = "[the docs](https://example.com)";
    assert_eq!(edited.selection, selection_of(&edited.text, written));
}

#[test]
fn a_caret_writes_an_empty_link_and_leaves_the_caret_to_type_the_label() {
    let text = "press here";
    let at = text.find("here").unwrap();

    let edited = insert_link(text, Range::caret(at), "").unwrap();

    assert_eq!(edited.text, "press []()here");
    assert_eq!(
        edited.selection,
        Range::caret(at + 1),
        "the caret is inside the brackets, ready to type"
    );
}

#[test]
fn a_target_can_be_supplied_for_a_label_the_user_has_not_typed_yet() {
    let text = "press here";
    let at = text.find("here").unwrap();

    let edited = insert_link(text, Range::caret(at), "https://example.com").unwrap();

    assert_eq!(edited.text, "press [](https://example.com)here");
    assert_eq!(edited.selection, Range::caret(at + 1));
}

#[test]
fn a_relative_target_is_written_exactly_as_given() {
    let text = "the guide";
    let edited = insert_link(text, all(text), "./notes/guide.md").unwrap();
    assert_eq!(edited.text, "[the guide](./notes/guide.md)");
}

#[test]
fn a_link_wraps_markup_that_is_already_there_without_rewriting_it() {
    let text = "**bold** words";
    let edited = insert_link(text, selection_of(text, "**bold**"), "https://example.com").unwrap();
    assert_eq!(edited.text, "[**bold**](https://example.com) words");
}

#[test]
fn a_written_link_is_read_back_as_a_link_span() {
    let text = "see the docs";
    let edited = insert_link(text, selection_of(text, "the docs"), "https://example.com").unwrap();

    let found = spans(&edited.text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind, SpanKind::Link);
    assert_eq!(found[0].content.slice(&edited.text), "the docs");
    assert_eq!(
        found[0].range.slice(&edited.text),
        "[the docs](https://example.com)"
    );
}

/// The whole document, as a user selecting everything would give it.
fn all(text: &str) -> Range {
    Range::new(0, text.len()).unwrap()
}
