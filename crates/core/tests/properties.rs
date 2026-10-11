//! Property tests.
//!
//! `docs/test-strategy.md` §5 asks for the engine's invariants to hold for
//! arbitrary input, not just the cases the example tests happen to name. These
//! properties are the ones the crate documents:
//!
//! * a document read in and never edited is written back byte for byte;
//! * an arbitrary offset never leaves the caret inside a character;
//! * undoing every edit restores the text exactly;
//! * wrapping lays every character out exactly once and never splits one.

use nc_core::Document;
use proptest::prelude::*;

proptest! {
    #[test]
    fn reading_and_writing_back_is_the_identity(text in ".{0,64}") {
        let document = Document::from_text(&text);
        prop_assert!(!document.is_dirty());
        prop_assert_eq!(document.save_text(), text.as_str());
    }

    #[test]
    fn the_line_count_matches_the_terminators(text in ".{0,64}") {
        let document = Document::from_text(&text);
        prop_assert_eq!(document.line_count(), document.lines().count());
        prop_assert_eq!(document.line_count(), document.text().split('\n').count());
    }

    #[test]
    fn an_arbitrary_offset_lands_on_a_character_boundary(
        text in ".{0,64}",
        offset in 0usize..96,
    ) {
        let mut document = Document::from_text(&text);
        document.set_caret(offset);
        prop_assert!(document.text().is_char_boundary(document.caret()));

        document.select(offset, offset / 2);
        prop_assert!(document.text().is_char_boundary(document.selection().anchor()));
        prop_assert!(document.text().is_char_boundary(document.selection().head()));
        prop_assert!(document.selection().start() <= document.selection().end());
    }

    #[test]
    fn undoing_every_edit_restores_the_original_text(
        text in ".{0,48}",
        ops in prop::collection::vec((0u8..3, ".{0,8}", 0usize..64), 0..12),
    ) {
        let mut document = Document::from_text(&text);
        let initial = document.text().to_owned();
        for (kind, argument, offset) in ops {
            document.set_caret(offset);
            match kind {
                0 => {
                    document.insert_str(&argument);
                }
                1 => {
                    document.insert_newline();
                }
                _ => {
                    document.backspace();
                }
            }
        }
        while document.undo() {}
        prop_assert_eq!(document.text(), initial.as_str());
    }

    #[test]
    fn wrapping_lays_every_character_out_exactly_once(
        text in ".{0,48}",
        columns in 1usize..16,
    ) {
        let document = Document::from_text(&text);
        let wrapped = document.wrapped_lines(columns);
        prop_assert!(!wrapped.is_empty());

        let mut covered = vec![0usize; document.text().len()];
        for line in &wrapped {
            prop_assert!(line.start <= line.end);
            prop_assert!(document.text().is_char_boundary(line.start));
            prop_assert!(document.text().is_char_boundary(line.end));
            for covered_byte in covered.iter_mut().take(line.end).skip(line.start) {
                *covered_byte += 1;
            }
        }
        for (index, byte) in document.text().bytes().enumerate() {
            if byte == b'\n' {
                prop_assert_eq!(covered[index], 0, "a terminator was laid out");
            } else {
                prop_assert_eq!(covered[index], 1, "a byte was laid out twice or not at all");
            }
        }
    }
}
