//! NC-D001/NC-D002 — UTF-16 and UTF-32, which are read by their marks.
//!
//! A wide encoding without a BOM cannot be told from ANSI, so a file in one is
//! UTF-16 or UTF-32 because it carries the mark, in either byte order
//! (`docs/reference-matrix.md` §4). A mark whose body is not decodable falls
//! back to ANSI rather than substituting characters.

use nc_textproc::encoding::encode;
use nc_textproc::{Encoding, Evidence, interpret};

fn utf16(text: &str, big_endian: bool) -> Vec<u8> {
    let units = text.encode_utf16().collect::<Vec<_>>();
    let mut bytes = Vec::new();
    for unit in units {
        let pair = if big_endian {
            unit.to_be_bytes()
        } else {
            unit.to_le_bytes()
        };
        bytes.extend_from_slice(&pair);
    }
    bytes
}

fn utf32(text: &str, big_endian: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    for character in text.chars() {
        let unit = character as u32;
        let quad = if big_endian {
            unit.to_be_bytes()
        } else {
            unit.to_le_bytes()
        };
        bytes.extend_from_slice(&quad);
    }
    bytes
}

#[test]
fn little_endian_utf16_is_read_by_its_mark() {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend_from_slice(&utf16("hi", false));
    let read = interpret(&bytes);
    assert_eq!(read.encoding(), Encoding::Utf16Le);
    assert_eq!(read.detection.evidence, Evidence::Utf16LeBom);
    assert!(read.has_bom());
    assert_eq!(read.text, "hi");
}

#[test]
fn big_endian_utf16_is_read_by_its_mark() {
    let mut bytes = vec![0xFE, 0xFF];
    bytes.extend_from_slice(&utf16("hi", true));
    let read = interpret(&bytes);
    assert_eq!(read.encoding(), Encoding::Utf16Be);
    assert_eq!(read.detection.evidence, Evidence::Utf16BeBom);
    assert_eq!(read.text, "hi");
}

#[test]
fn wide_text_survives_both_byte_orders() {
    for text in ["café", "日本語", "🙂"] {
        for big_endian in [false, true] {
            let mut bytes = if big_endian {
                vec![0xFE, 0xFF]
            } else {
                vec![0xFF, 0xFE]
            };
            bytes.extend_from_slice(&utf16(text, big_endian));
            assert_eq!(interpret(&bytes).text, text);
        }
    }
}

#[test]
fn utf32_is_told_from_utf16_by_its_longer_mark() {
    // A little-endian UTF-32 mark begins with the UTF-16 little-endian one, so
    // the longer mark has to be tested first.
    let mut little = vec![0xFF, 0xFE, 0x00, 0x00];
    little.extend_from_slice(&utf32("hi", false));
    let read = interpret(&little);
    assert_eq!(read.encoding(), Encoding::Utf32Le);
    assert_eq!(read.detection.evidence, Evidence::Utf32LeBom);
    assert_eq!(read.text, "hi");

    let mut big = vec![0x00, 0x00, 0xFE, 0xFF];
    big.extend_from_slice(&utf32("hi", true));
    let read = interpret(&big);
    assert_eq!(read.encoding(), Encoding::Utf32Be);
    assert_eq!(read.detection.evidence, Evidence::Utf32BeBom);
    assert_eq!(read.text, "hi");
}

#[test]
fn a_wide_mark_over_an_undecodable_body_falls_back_to_ansi() {
    // An odd number of UTF-16 bytes: the last half of a character is missing.
    let bytes = [0xFF, 0xFE, 0x41];
    let read = interpret(&bytes);
    assert_eq!(read.encoding(), Encoding::Ansi);
    assert_eq!(read.detection.evidence, Evidence::UndecodableBom);
    // Nothing was substituted: the three bytes are read as three characters.
    assert_eq!(read.text, "ÿþA");
}

#[test]
fn an_unpaired_surrogate_is_not_substituted() {
    // A high surrogate with no low surrogate after it.
    let bytes = [0xFF, 0xFE, 0x00, 0xD8, 0x41, 0x00];
    let read = interpret(&bytes);
    assert_eq!(read.encoding(), Encoding::Ansi);
    assert_eq!(read.detection.evidence, Evidence::UndecodableBom);
    assert!(!read.text.contains('\u{FFFD}'));
}

#[test]
fn reading_and_writing_a_wide_encoding_is_reversible() {
    for (mark, big_endian) in [(vec![0xFF, 0xFE], false), (vec![0xFE, 0xFF], true)] {
        for text in ["hi", "café", "日本語"] {
            let mut bytes = mark.clone();
            bytes.extend_from_slice(&utf16(text, big_endian));
            let read = interpret(&bytes);
            let written =
                encode(&read.text, read.encoding(), read.has_bom()).expect("a wide write");
            assert_eq!(written, bytes, "{text:?}");
        }
    }
}

#[test]
fn a_wide_encoding_cannot_be_written_without_its_mark() {
    // The mark is what identifies the encoding, so a save in one always
    // carries it, whatever the mark flag says.
    for encoding in [
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Utf32Le,
        Encoding::Utf32Be,
    ] {
        assert!(encoding.needs_bom(), "{encoding}");
        assert!(!encoding.bom_bytes().is_empty(), "{encoding}");
    }
    assert!(!Encoding::Utf8.needs_bom());
    assert!(Encoding::Ansi.bom_bytes().is_empty());
}
