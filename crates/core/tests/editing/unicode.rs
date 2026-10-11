//! NC-E008 — Unicode text editing.
//!
//! `docs/feature-matrix.md` NC-E008. The buffer holds UTF-8, so an edit can
//! never split a character or a grapheme cluster: a stale byte offset is
//! clamped to a character boundary instead of corrupting the text.

use nc_core::{CaretMovement, Document, LineColumn};

#[test]
fn a_multi_byte_character_is_one_caret_step() {
    let mut document = Document::from_text("é");
    document.set_caret(0);
    document.move_caret(CaretMovement::Right);
    assert_eq!(document.caret(), 2);
    document.move_caret(CaretMovement::Left);
    assert_eq!(document.caret(), 0);
}

#[test]
fn a_stale_offset_inside_a_character_is_clamped() {
    let mut document = Document::from_text("é");
    document.set_caret(1);
    // Byte 1 is inside "é", so the caret lands on the boundary before it.
    assert_eq!(document.caret(), 0);
    assert_eq!(document.caret_position(), LineColumn { line: 0, column: 0 });
}

#[test]
fn a_combining_sequence_is_one_column_and_one_backspace() {
    // "e" plus COMBINING ACUTE ACCENT.
    let mut document = Document::from_text("e\u{301}");
    assert_eq!(document.caret_position().column, 0);
    document.set_caret(document.byte_len());
    assert_eq!(document.caret_position().column, 1);

    document.move_caret(CaretMovement::Left);
    assert_eq!(document.caret(), 0);
    assert_eq!(document.selected_text(), None);

    document.set_caret(document.byte_len());
    assert!(document.backspace());
    assert_eq!(document.text(), "");
}

#[test]
fn an_emoji_zwj_sequence_is_one_backspace() {
    // A family emoji: three code points joined by ZWJ.
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    let mut document = Document::from_text(family);
    assert_eq!(document.byte_len(), family.len());
    document.set_caret(document.byte_len());
    assert_eq!(document.caret_position().column, 1);

    assert!(document.backspace());
    assert_eq!(document.text(), "");
}

#[test]
fn deleting_forward_never_splits_a_grapheme() {
    let mut document = Document::from_text("漢字");
    document.set_caret(0);
    assert!(document.delete_forward());
    assert_eq!(document.text(), "字");
    assert_eq!(document.caret(), 0);
}

#[test]
fn a_caret_offset_beyond_the_end_is_clamped() {
    let mut document = Document::from_text("abc");
    document.set_caret(9_999);
    assert_eq!(document.caret(), 3);
    document.set_caret(2);
    document.move_caret(CaretMovement::Right);
    assert_eq!(document.caret(), 3);
}

#[test]
fn multi_byte_text_survives_an_undo_and_redo_round_trip() {
    let mut document = Document::new();
    document.insert_str("héllo 漢字 🎉");
    let text = document.text().to_owned();
    document.undo();
    assert_eq!(document.text(), "");
    document.redo();
    assert_eq!(document.text(), text);
}

#[test]
fn a_stale_offset_pasted_in_is_still_normalised() {
    let mut document = Document::new();
    // CRLF inside a multi-byte string is normalised, not mangled.
    document.insert_str("é\r\n漢");
    assert_eq!(document.text(), "é\n漢");
    assert_eq!(document.line_count(), 2);
    assert_eq!(document.line(1), Some("漢"));
}
