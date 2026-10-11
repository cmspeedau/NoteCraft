//! NC-E006 — Word wrap.
//!
//! `docs/feature-matrix.md` NC-E006. The layout is pure arithmetic, so wrap
//! points are checkable without a viewport.

use nc_core::{Document, TabStop, wrap_line};

fn segments(text: &str, columns: usize) -> Vec<String> {
    wrap_line(text, 0, columns, TabStop::DEFAULT)
        .into_iter()
        .map(|wrapped| text[wrapped.start..wrapped.end].to_owned())
        .collect()
}

#[test]
fn word_wrap_is_off_on_a_new_document() {
    let mut document = Document::new();
    assert!(!document.word_wrap());
    document.set_word_wrap(true);
    assert!(document.word_wrap());
}

#[test]
fn a_short_line_is_one_segment() {
    assert_eq!(segments("short", 40), vec!["short"]);
}

#[test]
fn an_empty_line_is_one_empty_segment() {
    assert_eq!(segments("", 40), vec![""]);
}

#[test]
fn a_long_line_breaks_at_whitespace() {
    // Width 10, filled greedily: the break goes after the last whitespace that
    // fits. "alpha beta" is exactly 10 columns, so the space before "gamma"
    // would overflow and the break falls back to the space after "alpha".
    assert_eq!(
        segments("alpha beta gamma", 10),
        vec!["alpha ", "beta gamma"]
    );
}

#[test]
fn a_line_ending_exactly_at_the_width_is_kept_whole() {
    // The space after "quick" still fits, so the segment fills the viewport.
    assert_eq!(segments("the quick brown", 10), vec!["the quick ", "brown"]);
}

#[test]
fn a_run_wider_than_the_viewport_is_broken_anyway() {
    let segments = segments("abcdefghijklmnop", 5);
    assert!(
        segments.len() > 1,
        "a long run must not overflow the viewport"
    );
    assert_eq!(segments.concat(), "abcdefghijklmnop");
}

#[test]
fn every_byte_is_laid_out_exactly_once() {
    let text = "alpha beta gamma delta\nepsilon\n\nzeta";
    let document = Document::from_text(text);
    let wrapped = document.wrapped_lines(8);

    let mut rebuilt = String::new();
    for (index, line) in wrapped.iter().enumerate() {
        assert!(line.start <= line.end);
        if index + 1 < wrapped.len() {
            // A segment ends where the next begins, or just before a
            // terminator when the next segment starts a new logical line.
            let next = wrapped[index + 1];
            assert!(
                line.end == next.start || next.start == line.end + 1,
                "gap between {line:?} and {next:?}"
            );
        }
        rebuilt.push_str(&text[line.start..line.end]);
        if index + 1 < wrapped.len() && wrapped[index + 1].start == line.end + 1 {
            rebuilt.push('\n');
        }
    }
    assert_eq!(rebuilt, text);
}

#[test]
fn every_logical_line_gets_at_least_one_segment() {
    let document = Document::from_text("a\n\nb");
    let wrapped = document.wrapped_lines(10);
    assert_eq!(wrapped.len(), 3);
    assert_eq!(wrapped[1].len(), 0);
}

#[test]
fn tabs_advance_to_the_next_tab_stop_while_wrapping() {
    // "\t" is 8 columns wide at the default tab stop, so an 8-column viewport
    // fits the tab alone.
    let segments = segments("\tabc", 8);
    assert_eq!(segments, vec!["\t", "abc"]);
}

#[test]
fn wrapping_a_wide_character_counts_one_column() {
    // The reference's handling of double-width glyphs is a capture item; the
    // engine counts graphemes, which this pins.
    let segments = segments("漢字漢字", 3);
    assert_eq!(segments, vec!["漢字漢", "字"]);
}
