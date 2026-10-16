//! NC-E009 — line endings.
//!
//! The buffer holds `\n` terminators only; the file's own style is remembered on
//! the `Document` and put back on save. A file that is opened and never edited is
//! written back byte for byte, so a mixed file is not silently rewritten. Which
//! style a new document uses by default is a design decision
//! (`docs/ui-reference.md`); this suite pins the behaviour the engine promises,
//! not any particular default.

use nc_core::{Document, LineEnding};

#[test]
fn a_new_document_uses_crlf_and_is_clean() {
    let document = Document::new();
    assert_eq!(document.line_ending(), LineEnding::CrLf);
    assert!(!document.is_dirty());
    assert_eq!(document.text(), "");
    assert_eq!(document.line_count(), 1);
}

#[test]
fn the_first_style_in_a_file_becomes_the_documents_style() {
    assert_eq!(
        Document::from_text("a\r\nb\r\n").line_ending(),
        LineEnding::CrLf
    );
    assert_eq!(Document::from_text("a\nb").line_ending(), LineEnding::Lf);
    assert_eq!(Document::from_text("a\rb").line_ending(), LineEnding::Cr);
}

#[test]
fn a_file_with_no_terminator_uses_the_default_style() {
    let document = Document::from_text("no terminator");
    assert_eq!(document.line_ending(), LineEnding::DEFAULT);
    let report = document.line_ending_report();
    assert_eq!(report.total(), 0);
    assert!(!report.any());
}

#[test]
fn each_style_is_counted_in_the_report() {
    let report = Document::from_text("a\r\nb\r\nc\n").line_ending_report();
    assert_eq!(report.crlf, 2);
    assert_eq!(report.lf, 1);
    assert_eq!(report.cr, 0);
    assert!(report.mixed);
}

#[test]
fn the_buffer_holds_lf_terminators_only() {
    let document = Document::from_text("one\r\ntwo\r\nthree");
    assert_eq!(document.text(), "one\ntwo\nthree");
    assert_eq!(document.line_count(), 3);
    assert_eq!(document.line(1), Some("two"));
}

#[test]
fn an_untouched_document_is_written_back_byte_for_byte() {
    for text in ["a\r\nb\nc", "a\nb", "single", "a\r\nb\r\n"] {
        let document = Document::from_text(text);
        assert!(!document.is_dirty(), "reading {text:?} left it dirty");
        assert_eq!(document.save_text(), text);
    }
}

#[test]
fn a_mixed_file_is_reported_as_mixed_but_saved_unchanged() {
    let text = "a\r\nb\nc";
    let document = Document::from_text(text);
    assert!(document.has_mixed_line_endings());
    assert_eq!(document.save_text(), text);
}

#[test]
fn an_edited_document_is_written_in_its_own_style() {
    let mut document = Document::from_text("one\r\ntwo\r\n");
    let end = document.byte_len();
    document.set_caret(end);
    assert!(document.insert_str("three"));
    assert!(document.is_dirty());
    assert_eq!(document.save_text(), "one\r\ntwo\r\nthree");
}

#[test]
fn an_edited_lf_document_is_written_as_lf() {
    let mut document = Document::from_text("one\ntwo\n");
    let end = document.byte_len();
    document.set_caret(end);
    document.insert_str("three");
    assert_eq!(document.save_text(), "one\ntwo\nthree");
}

#[test]
fn saving_marks_the_document_clean_and_pins_the_bytes() {
    let mut document = Document::from_text("a\nb");
    assert!(!document.is_dirty());
    let end = document.byte_len();
    document.set_caret(end);
    document.insert_str("!");
    assert!(document.is_dirty());
    assert_eq!(document.save_text(), "a\nb!");
    document.mark_saved();
    assert!(!document.is_dirty());
    assert_eq!(document.save_text(), "a\nb!");
}

#[test]
fn text_pasted_with_crlf_keeps_one_terminator_per_break() {
    let mut document = Document::new();
    assert!(document.insert_uncoalesced("one\r\ntwo\rthree"));
    assert_eq!(document.text(), "one\ntwo\nthree");
    assert_eq!(document.line_count(), 3);
}
