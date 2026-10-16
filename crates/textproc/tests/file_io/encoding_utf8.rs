//! NC-D001/NC-D002 — reading UTF-8, and what counts as UTF-8.
//!
//! A BOM-less file is UTF-8 when its bytes are valid UTF-8 and ANSI otherwise;
//! a UTF-8 mark is believed only when the body lives up to it. Nothing here
//! substitutes a character for input it cannot read
//! (`docs/ui-reference.md`).

use nc_textproc::encoding::{encode, interpret};
use nc_textproc::{Detection, Encoding, Evidence, TextFile, detect};

#[test]
fn plain_ascii_is_utf8_without_a_mark() {
    let detection = detect(b"note\r\ntext");
    assert_eq!(detection.encoding, Encoding::Utf8);
    assert_eq!(detection.evidence, Evidence::ValidUtf8);
    assert!(!detection.bom);
}

#[test]
fn valid_multibyte_is_utf8() {
    for text in ["café", "日本語", "🙂", "a\u{0}b"] {
        let detection = detect(text.as_bytes());
        assert_eq!(detection.encoding, Encoding::Utf8, "{text:?}");
        assert_eq!(detection.evidence, Evidence::ValidUtf8, "{text:?}");
    }
}

#[test]
fn a_mark_makes_a_file_utf8_and_is_not_part_of_the_text() {
    let bytes = [b"\xEF\xBB\xBF".as_slice(), b"hi"].concat();
    let detection = detect(&bytes);
    assert_eq!(detection.encoding, Encoding::Utf8);
    assert_eq!(detection.evidence, Evidence::Utf8Bom);
    assert!(detection.bom);

    let read = interpret(&bytes);
    assert_eq!(read.text, "hi");
    assert_eq!(read.encoding(), Encoding::Utf8);
    assert!(read.has_bom());
}

#[test]
fn bytes_that_are_not_utf8_are_read_as_ansi() {
    // 0xE9 is "é" in the Windows code page, and is not valid UTF-8.
    let detection = detect(b"caf\xE9");
    assert_eq!(detection.encoding, Encoding::Ansi);
    assert_eq!(detection.evidence, Evidence::NotUtf8);
    assert!(!detection.bom);
    assert_eq!(interpret(b"caf\xE9").text, "café");
}

#[test]
fn a_mark_the_body_does_not_live_up_to_is_not_utf8() {
    let bytes = [b"\xEF\xBB\xBF".as_slice(), b"\xFF\xFE\x41\x00"].concat();
    let detection = detect(&bytes);
    assert_eq!(detection.encoding, Encoding::Ansi);
    assert_eq!(detection.evidence, Evidence::UndecodableBom);
    // Read as ANSI, the mark's own bytes are read as characters rather than
    // dropped, so nothing is lost on the way back out.
    assert_eq!(interpret(&bytes).text, "ï»¿ÿþA\u{0}");
}

#[test]
fn reading_and_writing_utf8_is_reversible() {
    for bytes in [
        b"\xEF\xBB\xBFhello".to_vec(),
        "café 🙂".as_bytes().to_vec(),
        b"".to_vec(),
    ] {
        let read = interpret(&bytes);
        let written = encode(&read.text, read.encoding(), read.has_bom()).expect("a UTF-8 write");
        assert_eq!(written, bytes);
    }
}

#[test]
fn a_new_document_is_written_as_utf8_without_a_mark() {
    let detection = Detection::new_document();
    assert_eq!(detection.encoding, Encoding::Utf8);
    assert_eq!(detection.evidence, Evidence::NewDocument);
    assert!(!detection.bom);

    let file = TextFile::untitled();
    assert_eq!(file.encoding(), Encoding::Utf8);
    assert!(!file.has_bom());
    assert_eq!(file.encoding_label(), "UTF-8");
}
