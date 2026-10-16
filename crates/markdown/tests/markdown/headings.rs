//! `NC-M005` Headings.
//!
//! Acceptance: **"ATX output"**. NoteCraft writes `#` through `######` followed by
//! one space, which is what these tests compare against, on whole strings.

use nc_markdown::{BlockKind, Heading, Range, blocks, set_heading};

/// The selection covering `needle`.
fn selection_of(haystack: &str, needle: &str) -> Range {
    let start = haystack
        .find(needle)
        .expect("the selection is present in the text");
    Range::new(start, start + needle.len()).expect("whole characters")
}

#[test]
fn a_heading_is_written_as_atx_with_one_space() {
    let text = "Chapter one";
    let edited = set_heading(text, selection_of(text, "Chapter one"), Heading::Level(1)).unwrap();
    assert_eq!(edited.text, "# Chapter one");
}

#[test]
fn each_level_writes_its_own_hash_run() {
    for level in 1..=6 {
        let text = "Title";
        let edited = set_heading(
            text,
            selection_of(text, "Title"),
            Heading::from_level(level).unwrap(),
        )
        .unwrap();
        assert_eq!(
            edited.text,
            format!("{} Title", "#".repeat(level as usize)),
            "level {level}"
        );
    }
}

#[test]
fn there_is_no_seventh_level_to_write() {
    assert_eq!(Heading::from_level(7), None, "ATX stops at six");
}

#[test]
fn asking_for_the_level_it_already_carries_makes_it_body_text() {
    let text = "## Chapter one";
    let edited = set_heading(text, selection_of(text, "Chapter"), Heading::Level(2)).unwrap();
    assert_eq!(edited.text, "Chapter one");
}

#[test]
fn a_different_level_replaces_the_run_that_was_there_rather_than_adding_one() {
    let text = "# Chapter one";
    let edited = set_heading(text, selection_of(text, "Chapter"), Heading::Level(3)).unwrap();
    assert_eq!(edited.text, "### Chapter one");
}

#[test]
fn every_line_the_selection_touches_becomes_a_heading() {
    let text = "one\ntwo\nthree";
    let edited = set_heading(text, Range::new(0, 7).unwrap(), Heading::Level(2)).unwrap();
    assert_eq!(edited.text, "## one\n## two\nthree");
}

#[test]
fn a_hash_that_is_part_of_a_word_is_left_alone_and_still_gets_a_heading() {
    // `#hashtag` is not a heading, so the command has something to add rather than
    // something to replace.
    let text = "#hashtag trends";
    let edited = set_heading(text, selection_of(text, "trends"), Heading::Level(1)).unwrap();
    assert_eq!(edited.text, "# #hashtag trends");
}

#[test]
fn a_list_marker_gives_way_to_the_heading() {
    let text = "- Chapter one";
    let edited = set_heading(text, selection_of(text, "Chapter"), Heading::Level(2)).unwrap();
    assert_eq!(edited.text, "## Chapter one", "not \"- ## Chapter one\"");
}

#[test]
fn indentation_in_front_of_a_heading_is_the_authors_and_stays() {
    let text = "  Chapter one";
    let edited = set_heading(text, selection_of(text, "Chapter"), Heading::Level(1)).unwrap();
    assert_eq!(edited.text, "  # Chapter one");
}

#[test]
fn a_caret_on_a_line_headings_that_line_alone() {
    let text = "one\ntwo\nthree";
    let on_two = text.find("two").unwrap();
    let edited = set_heading(text, Range::caret(on_two), Heading::Level(1)).unwrap();
    assert_eq!(edited.text, "one\n# two\nthree");
}

#[test]
fn the_written_heading_reads_back_as_that_level() {
    let text = "Title";
    let edited = set_heading(text, selection_of(text, "Title"), Heading::Level(2)).unwrap();
    let read = blocks(&edited.text);
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].kind, BlockKind::Heading(2));
    assert_eq!(read[0].content(), "Title");
}
