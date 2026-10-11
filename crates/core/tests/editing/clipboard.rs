//! NC-E002 — Copy, Cut, and Paste.
//!
//! `docs/feature-matrix.md` NC-E002. The document copies through the
//! [`nc_core::Clipboard`] trait, so the whole round trip is testable off
//! Windows; the Win32 clipboard implements the same trait behind `cfg(windows)`.

use nc_core::{Clipboard, Document, MemoryClipboard};

#[test]
fn copy_puts_the_selection_on_the_clipboard() {
    let mut clipboard = MemoryClipboard::new();
    let mut document = Document::from_text("hello world");
    document.select(6, 11);

    assert!(document.copy_to(&mut clipboard));
    assert_eq!(clipboard.contents(), Some("world"));
    // Copying leaves the document alone.
    assert_eq!(document.text(), "hello world");
    assert!(!document.is_dirty());
}

#[test]
fn copy_without_a_selection_copies_nothing() {
    let mut clipboard = MemoryClipboard::new();
    let document = Document::from_text("hello");

    assert!(!document.copy_to(&mut clipboard));
    assert_eq!(clipboard.contents(), None);
}

#[test]
fn copy_of_a_multi_line_selection_uses_crlf() {
    let mut clipboard = MemoryClipboard::new();
    let mut document = Document::from_text("one\ntwo\nthree");
    document.select(0, 7);

    assert!(document.copy_to(&mut clipboard));
    assert_eq!(clipboard.contents(), Some("one\r\ntwo"));
}

#[test]
fn cut_removes_the_selection_and_keeps_it_on_the_clipboard() {
    let mut clipboard = MemoryClipboard::new();
    let mut document = Document::from_text("hello world");
    document.select(5, 11);

    assert!(document.cut_to(&mut clipboard));
    assert_eq!(document.text(), "hello");
    assert_eq!(clipboard.contents(), Some(" world"));
    assert_eq!(document.caret(), 5);
}

#[test]
fn paste_replaces_the_selection() {
    let mut clipboard = MemoryClipboard::new();
    clipboard.set_text("there".to_owned());
    let mut document = Document::from_text("hello world");
    document.select(6, 11);

    assert!(document.paste_from(&clipboard));
    assert_eq!(document.text(), "hello there");
    assert_eq!(document.caret(), 11);
}

#[test]
fn paste_normalises_crlf_to_the_buffer_terminator() {
    let mut clipboard = MemoryClipboard::new();
    clipboard.set_text("a\r\nb".to_owned());
    let mut document = Document::new();

    assert!(document.paste_from(&clipboard));
    assert_eq!(document.text(), "a\nb");
    assert_eq!(document.line_count(), 2);
}

#[test]
fn a_paste_is_one_undo_step_even_after_another_paste() {
    let mut clipboard = MemoryClipboard::new();
    clipboard.set_text("x".to_owned());
    let mut document = Document::new();

    document.paste_from(&clipboard);
    document.paste_from(&clipboard);
    assert_eq!(document.text(), "xx");
    assert_eq!(document.undo_depth(), 2);

    document.undo();
    assert_eq!(document.text(), "x");
}

#[test]
fn pasting_an_empty_clipboard_changes_nothing() {
    let mut document = Document::new();
    let clipboard = MemoryClipboard::new();

    assert!(!document.paste_from(&clipboard));
    assert!(!document.is_dirty());
}

#[test]
fn paste_follows_a_copy_between_documents() {
    let mut clipboard = MemoryClipboard::new();
    let mut source = Document::from_text("shared text");
    source.select_all();
    assert!(source.copy_to(&mut clipboard));

    let mut target = Document::new();
    assert!(target.paste_from(&clipboard));
    assert_eq!(target.text(), "shared text");
}
