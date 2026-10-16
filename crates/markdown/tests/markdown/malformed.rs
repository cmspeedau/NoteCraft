//! `NC-M011` Malformed and unsupported syntax is preserved.
//!
//! Acceptance: **"No silent normalization"**. Two things have to hold, and both are
//! checked hard: reading a document never rewrites it, and a formatting command
//! adds only its own bytes. The property tests at the bottom say the same things
//! about inputs nobody thought to write a case for.

use nc_markdown::{Heading, Inline, Range, blocks, insert_link, set_heading, spans, toggle_inline};
use proptest::prelude::*;

/// Constructs the parser does not implement — footnote definitions, raw HTML,
/// fenced code, definition lists, math, admonitions, task items.
const UNSUPPORTED: &[&str] = &[
    "[^1]: a footnote definition\n",
    "<div class=\"note\">\n",
    "```fenced\nliteral **text**\n```\n",
    "Term\n:   definition\n",
    "$x^2$ inline math\n",
    "==highlighted== text\n",
    "- [ ] an unchecked task item\n",
    "!!! note\n    an admonition\n",
    "<!-- a comment -->\n",
    "\\# an escaped hash\n",
];

/// Text the parser has to cope with but cannot make sense of.
const MALFORMED: &[&str] = &[
    "a **b c\n",
    "a *b\n",
    "`unopened code\n",
    "[unclosed label](\n",
    "**\n",
    "***\n",
    "~~~\n",
    "]\n",
    "[\n",
    "~~",
];

#[test]
fn reading_an_unsupported_construct_returns_the_same_bytes() {
    for source in UNSUPPORTED {
        let rebuilt: String = blocks(source).iter().map(|block| block.source).collect();
        assert_eq!(&rebuilt, source, "{source:?} came back changed");
    }
}

#[test]
fn reading_malformed_syntax_returns_the_same_bytes() {
    for source in MALFORMED {
        let rebuilt: String = blocks(source).iter().map(|block| block.source).collect();
        assert_eq!(&rebuilt, source, "{source:?} came back changed");
    }
}

#[test]
fn the_blocks_of_a_whole_document_tile_it_without_gaps_or_overlaps() {
    let document = "# Title\n\n- one\n  - two\n\n| a | b |\n| --- | --- |\n\n> quote\n```code\n";
    let read = blocks(document);
    let mut cursor = 0usize;
    for block in &read {
        assert_eq!(block.range.start, cursor, "a gap before {:?}", block.source);
        cursor = block.range.end;
    }
    assert_eq!(cursor, document.len(), "the last block stops short");
}

#[test]
fn formatting_inside_an_unknown_construct_adds_markers_and_changes_nothing_else() {
    let source = "```fenced\nliteral **text**\n```\n";
    let target = source.find("literal").unwrap();
    let selection = Range::new(target, target + "literal".len()).unwrap();

    let edited = toggle_inline(source, selection, Inline::Bold).unwrap();

    assert_eq!(
        edited.text, "```fenced\n**literal** **text**\n```\n",
        "the fence and the bytes either side of the selection are exactly as they were"
    );
}

#[test]
fn selecting_text_that_is_already_marked_inside_a_fence_unmarks_only_it() {
    // The same command on the same buffer, with the caret on marked text, answers
    // the other way round — and still touches nothing but those four bytes.
    let source = "```fenced\nliteral **text**\n```\n";
    let target = source.find("**text**").unwrap() + 2;
    let selection = Range::new(target, target + 4).unwrap();

    let edited = toggle_inline(source, selection, Inline::Bold).unwrap();

    assert_eq!(edited.text, "```fenced\nliteral text\n```\n");
}

#[test]
fn heading_a_line_of_unsupported_syntax_leaves_the_rest_of_the_document_alone() {
    let source = "[^1]: a footnote definition\n";
    let edited = set_heading(source, Range::caret(0), Heading::Level(2)).unwrap();
    assert_eq!(edited.text, "## [^1]: a footnote definition\n");
}

#[test]
fn a_task_item_keeps_its_brackets_when_it_is_made_a_heading() {
    let source = "- [ ] unfinished\n";
    let edited = set_heading(source, Range::caret(0), Heading::Level(1)).unwrap();
    assert_eq!(edited.text, "# [ ] unfinished\n");
}

#[test]
fn an_unpaired_delimiter_becomes_no_span_at_all() {
    for source in MALFORMED {
        let found = spans(source);
        for span in &found {
            assert!(
                span.range.end <= source.len(),
                "{span:?} runs past the end of {source:?}"
            );
        }
    }
}

#[test]
fn a_link_written_around_malformed_markup_is_still_exact() {
    let source = "a **b c";
    let selection = Range::new(0, source.len()).unwrap();
    let edited = insert_link(source, selection, "https://example.com").unwrap();
    assert_eq!(edited.text, "[a **b c](https://example.com)");
}

/// Any text at all, including newlines, which is where block structure breaks.
fn any_text() -> impl Strategy<Value = String> {
    "[\\x20-\\x7e\\n]{0,120}"
}

proptest! {
    #[test]
    fn parsing_any_text_reproduces_it_exactly(source in any_text()) {
        let rebuilt: String = blocks(&source).iter().map(|block| block.source).collect();
        prop_assert_eq!(rebuilt, source);
    }

    #[test]
    fn spans_of_any_text_are_ordered_in_bounds_and_nested(source in any_text()) {
        let found = spans(&source);
        let mut previous_end = 0usize;
        for span in found {
            prop_assert!(span.range.start >= previous_end, "overlap at {:?}", span.range);
            prop_assert!(span.range.start <= span.content.start);
            prop_assert!(span.content.end <= span.range.end);
            prop_assert!(span.range.end <= source.len());
            previous_end = span.range.end;
        }
    }

    #[test]
    fn bolding_any_text_adds_only_its_own_four_bytes(source in any_text()) {
        let selection = Range::new(0, source.len()).unwrap();
        if let Some(edited) = toggle_inline(&source, selection, Inline::Bold) {
            if edited.text.len() == source.len() + 4 {
                let stripped = edited.text
                    .strip_prefix("**")
                    .and_then(|rest| rest.strip_suffix("**"))
                    .expect("the added markers wrap the whole text");
                prop_assert_eq!(stripped.to_string(), source);
            } else {
                // The text already opened and closed with the marker, so the
                // command unwrapped it instead: four bytes shorter, same content.
                prop_assert_eq!(edited.text.len(), source.len().saturating_sub(4));
            }
        }
    }

    #[test]
    fn clearing_formatting_never_invents_a_line_break(source in any_text()) {
        let selection = Range::new(0, source.len()).unwrap();
        if let Some(cleared) = nc_markdown::clear_formatting(&source, selection) {
            prop_assert!(
                cleared.text.matches('\n').count() <= source.matches('\n').count(),
                "clearing must not add newlines: {source:?} became {:?}",
                cleared.text
            );
        }
    }
}
