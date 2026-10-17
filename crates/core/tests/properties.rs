//! Property tests.
//!
//! `docs/test-strategy.md` §5 asks for the engine's invariants to hold for
//! arbitrary input, not just the cases the example tests happen to name. These
//! properties are the ones the crate documents:
//!
//! * a document read in and never edited is written back byte for byte;
//! * an arbitrary offset never leaves the caret inside a character;
//! * undoing every edit restores the text exactly;
//! * a click or a drag never leaves the selection inside a character;
//! * a move rearranges the same text and undoes as one edit;
//! * an overwrite never eats a line terminator or splits a character;
//! * wrapping lays every character out exactly once and never splits one.

use nc_core::{ClickKind, Document, TextBuffer, click_selection, drag_selection};
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

    #[test]
    fn a_click_lands_on_a_character_boundary(
        text in ".{0,48}",
        offset in 0usize..64,
        kind in 0u8..3,
    ) {
        // The buffer holds `\n` terminators only, so build it from the
        // normalized text rather than from the raw input.
        let document = Document::from_text(&text);
        let normalized = document.text().to_owned();
        let buffer = TextBuffer::from_normalized(&normalized);
        let kind = match kind {
            0 => ClickKind::Single,
            1 => ClickKind::Double,
            _ => ClickKind::Triple,
        };
        let selection = click_selection(&buffer, offset, kind);
        prop_assert!(normalized.is_char_boundary(selection.anchor()));
        prop_assert!(normalized.is_char_boundary(selection.head()));
        prop_assert!(selection.start() <= selection.end());
        prop_assert!(selection.end() <= normalized.len());
        prop_assert!(buffer.slice(selection.range()).is_some());
    }

    #[test]
    fn a_drag_lands_on_a_character_boundary(text in ".{0,48}", anchor in 0usize..64, head in 0usize..64) {
        let document = Document::from_text(&text);
        let normalized = document.text().to_owned();
        let buffer = TextBuffer::from_normalized(&normalized);
        for kind in [ClickKind::Single, ClickKind::Double, ClickKind::Triple] {
            let selection = drag_selection(&buffer, anchor, head, kind);
            prop_assert!(normalized.is_char_boundary(selection.anchor()));
            prop_assert!(normalized.is_char_boundary(selection.head()));
            prop_assert!(selection.end() <= normalized.len());
            prop_assert!(buffer.slice(selection.range()).is_some());
            // The end that was not dragged does not move.
            let mut document = Document::from_text(&normalized);
            document.click(anchor, kind);
            document.extend_to(head, kind);
            prop_assert_eq!(document.selection(), selection);
        }
    }

    #[test]
    fn a_move_rearranges_the_text_and_undoes_in_one_step(
        text in ".{0,40}",
        anchor in 0usize..48,
        head in 0usize..48,
        target in 0usize..48,
    ) {
        let mut document = Document::from_text(&text);
        document.select(anchor, head);
        let before = document.text().to_owned();
        let depth = document.undo_depth();
        if document.move_selection_to(target) {
            prop_assert_eq!(document.undo_depth(), depth + 1);
            // A move neither invents nor destroys text: only its order changes.
            let mut expected: Vec<u8> = before.bytes().collect();
            let mut actual: Vec<u8> = document.text().bytes().collect();
            expected.sort_unstable();
            actual.sort_unstable();
            prop_assert_eq!(expected, actual);
            prop_assert_eq!(document.text().len(), before.len());
            // The moved span is selected afterwards.
            prop_assert_eq!(document.selected_text().map(str::len), Some(document.selection().len()));

            prop_assert!(document.undo());
            prop_assert_eq!(document.text(), before.as_str());
        }
    }

    #[test]
    fn an_overwrite_never_eats_a_terminator_or_splits_a_character(
        text in ".{0,48}",
        offset in 0usize..64,
        typed in "[^\n\r]{0,6}",
    ) {
        let mut document = Document::from_text(&text);
        let original = document.text().to_owned();
        document.set_overwrite(true);
        document.set_caret(offset);
        if document.insert_str(&typed) {
            prop_assert!(document.text().is_char_boundary(document.caret()));
            prop_assert_eq!(
                document.text().matches('\n').count(),
                original.matches('\n').count(),
                "an overwrite crossed a line terminator"
            );
        }
        while document.undo() {}
        prop_assert_eq!(document.text(), original.as_str());
    }

    // `ROADMAP.md` M11: "run randomized editing sequences." The properties above drive
    // one kind of operation at a time; the one that mixes operations (`undoing_every_edit…`)
    // only interleaves insert, newline and backspace, checks nothing in between, and never
    // redoes. This one drives an arbitrary mixture of every editing, caret and selection
    // command — typing, newline, backspace, forward delete, deleting and replacing a
    // selection, plain selection, click/drag selection, and an overwrite — asserting after
    // EACH step that the caret and the selection still sit on character boundaries and
    // inside the text, and that the whole sequence unwinds to the original text and redoes
    // back to the final one. A panic, a caret stranded mid-character, or an undo/redo that
    // loses or duplicates an edit fails here, whatever the mixture that produced it.
    #[test]
    fn a_randomized_sequence_keeps_the_text_intact_and_round_trips(
        text in ".{0,48}",
        steps in prop::collection::vec(
            (0u8..9, "[^\n\r]{0,5}", 0usize..64, 0usize..64),
            0..16,
        ),
    ) {
        let mut document = Document::from_text(&text);
        let initial = document.text().to_owned();

        for (kind, typed, offset, other) in steps {
            match kind {
                0 => {
                    document.set_caret(offset);
                    document.insert_str(&typed);
                }
                1 => {
                    document.set_caret(offset);
                    document.insert_newline();
                }
                2 => {
                    document.set_caret(offset);
                    document.backspace();
                }
                3 => {
                    document.set_caret(offset);
                    document.delete_forward();
                }
                4 => {
                    document.select(offset, other);
                    document.delete_selection();
                }
                5 => {
                    // Typing over a selection replaces it.
                    document.select(offset, other);
                    document.insert_str(&typed);
                }
                6 => {
                    document.select(offset, other);
                }
                7 => {
                    let kind = match other % 3 {
                        0 => ClickKind::Single,
                        1 => ClickKind::Double,
                        _ => ClickKind::Triple,
                    };
                    document.click(offset, kind);
                    document.extend_to(other, kind);
                }
                _ => {
                    // One keystroke in overwrite, then leave it as we found it.
                    document.set_overwrite(true);
                    document.set_caret(offset);
                    document.insert_str(&typed);
                    document.set_overwrite(false);
                }
            }

            // Whatever just ran, the caret and the selection stay on character
            // boundaries and inside the text.
            let after = document.text();
            let caret = document.caret();
            prop_assert!(
                after.is_char_boundary(caret),
                "the caret sits inside a character at {caret}"
            );
            let selection = document.selection();
            prop_assert!(after.is_char_boundary(selection.anchor()));
            prop_assert!(after.is_char_boundary(selection.head()));
            prop_assert!(selection.start() <= selection.end());
            prop_assert!(
                selection.end() <= after.len(),
                "the selection runs past the end ({} > {})",
                selection.end(),
                after.len()
            );
        }

        let final_text = document.text().to_owned();

        // Every edit unwinds back to where the sequence started…
        while document.undo() {}
        prop_assert_eq!(document.text(), initial.as_str());

        // …and redoing all of it lands exactly on the last state. An undo/redo stack
        // that dropped a step, or grouped two edits and replayed them wrong, breaks here.
        while document.redo() {}
        prop_assert_eq!(document.text(), final_text.as_str());
    }
}
