//! `NC-M006` Ordered and unordered lists, nesting, and indentation.
//!
//! Acceptance: **"Nested indent correct"**. Nesting is two spaces per level, and
//! the tests check that a marker change keeps the nesting, that indenting nests a
//! line, and that the parser reads the nesting back as the same width.

use nc_markdown::{BlockKind, ListKind, Range, blocks, indent_lines, outdent_lines, set_list};

/// The whole document, as a user selecting everything would give it.
fn all(text: &str) -> Range {
    Range::new(0, text.len()).unwrap()
}

#[test]
fn a_bulleted_list_writes_a_dash_before_every_selected_line() {
    let text = "one\ntwo\nthree";
    let edited = set_list(text, all(text), ListKind::Bullet).unwrap();
    assert_eq!(edited.text, "- one\n- two\n- three");
}

#[test]
fn a_numbered_list_numbers_from_one_across_the_selection() {
    let text = "one\ntwo\nthree";
    let edited = set_list(text, all(text), ListKind::Numbered).unwrap();
    assert_eq!(edited.text, "1. one\n2. two\n3. three");
}

#[test]
fn applying_the_kind_the_lines_already_are_strips_the_markers() {
    let bulleted = "- one\n- two\n";
    let edited = set_list(bulleted, all(bulleted), ListKind::Bullet).unwrap();
    assert_eq!(edited.text, "one\ntwo\n");

    let numbered = "1. one\n2. two\n";
    let edited = set_list(numbered, all(numbered), ListKind::Numbered).unwrap();
    assert_eq!(edited.text, "one\ntwo\n");
}

#[test]
fn switching_kind_replaces_the_marker_rather_than_adding_a_second_one() {
    let text = "- one\n- two\n";
    let edited = set_list(text, all(text), ListKind::Numbered).unwrap();
    assert_eq!(edited.text, "1. one\n2. two\n");
}

#[test]
fn indenting_a_line_nests_it_by_one_level() {
    let text = "- one\n- two\n";
    let on_two = text.find("- two").unwrap();
    let edited = indent_lines(text, Range::caret(on_two)).unwrap();
    assert_eq!(edited.text, "- one\n  - two\n");
}

#[test]
fn outdenting_removes_exactly_the_indent_that_indenting_added() {
    let text = "- one\n  - two\n";
    let on_two = text.find("- two").unwrap();
    let edited = outdent_lines(text, Range::caret(on_two)).unwrap();
    assert_eq!(edited.text, "- one\n- two\n");
}

#[test]
fn outdenting_an_unindented_line_changes_nothing() {
    let text = "- one\n";
    let edited = outdent_lines(text, Range::caret(0)).unwrap();
    assert_eq!(edited.text, text);
}

#[test]
fn nesting_survives_a_change_of_list_kind() {
    let text = "- one\n  - two\n";
    let on_two = text.find("- two").unwrap();
    let edited = set_list(text, Range::caret(on_two), ListKind::Numbered).unwrap();
    assert_eq!(
        edited.text, "- one\n  1. two\n",
        "the indent is not the marker"
    );
}

#[test]
fn two_levels_of_nesting_read_back_with_their_indent_widths() {
    let text = "- one\n  - two\n    - three\n";
    let read = blocks(text);
    let indents: Vec<usize> = read
        .iter()
        .filter_map(|block| match block.kind {
            BlockKind::ListItem { indent, .. } => Some(indent),
            _ => None,
        })
        .collect();
    assert_eq!(indents, vec![0, 2, 4], "one unit per level");
}

#[test]
fn a_star_or_plus_bullet_is_still_a_bullet() {
    for marker in ["*", "+"] {
        let text = &format!("{marker} one\n");
        let edited = set_list(text, all(text), ListKind::Bullet).unwrap();
        assert_eq!(edited.text, "one\n", "`{marker}` bullet stripped");
    }
}

#[test]
fn an_asterisk_of_emphasis_is_not_mistaken_for_a_bullet() {
    let text = "*emphasis* stays";
    let edited = set_list(text, all(text), ListKind::Bullet).unwrap();
    assert_eq!(edited.text, "- *emphasis* stays");
}

#[test]
fn a_version_number_is_not_a_list_item() {
    let text = "1.2.3 was released";
    let read = blocks(text);
    assert_eq!(read[0].kind, BlockKind::Paragraph, "not a numbered item");
}

#[test]
fn a_heading_gives_way_to_a_list_marker() {
    let text = "# one";
    let edited = set_list(text, all(text), ListKind::Bullet).unwrap();
    assert_eq!(edited.text, "- one", "not \"- # one\"");
}
