//! NC-D001/NC-D002 — the ANSI code page, the fallback that can read anything.
//!
//! ANSI here is the Windows Western code page, which the reference uses as its
//! "ANSI" (`docs/reference-matrix.md` §4). Every byte has one character and
//! every character it holds has one byte, so reading any byte string as ANSI
//! never substitutes and never loses anything. The locale-specific code pages
//! the reference picks per locale are a `TO_CAPTURE` item, so only this table
//! changes when they are captured.

use nc_textproc::ansi::{self, decode_byte, encode_char};
use nc_textproc::{Encoding, Evidence, detect, interpret};

#[test]
fn the_high_bytes_are_the_code_page_characters() {
    // The bytes Windows-1252 differs from Latin-1 in.
    assert_eq!(decode_byte(0x80), '€');
    assert_eq!(decode_byte(0x85), '…');
    assert_eq!(decode_byte(0x91), '\u{2018}');
    assert_eq!(decode_byte(0x92), '\u{2019}');
    assert_eq!(decode_byte(0x93), '\u{201C}');
    assert_eq!(decode_byte(0x94), '\u{201D}');
    assert_eq!(decode_byte(0x96), '–');
    assert_eq!(decode_byte(0x97), '—');
    assert_eq!(decode_byte(0xE9), 'é');
    assert_eq!(decode_byte(0x41), 'A');
}

#[test]
fn the_bytes_the_code_page_leaves_undefined_still_round_trip() {
    for byte in [0x81, 0x8D, 0x8F, 0x90, 0x9D] {
        let character = decode_byte(byte);
        assert_eq!(encode_char(character), Some(byte), "{byte:#04X}");
    }
}

#[test]
fn every_byte_reads_back_as_itself() {
    let bytes: Vec<u8> = (0..=255).collect();
    let text = ansi::decode(&bytes);
    assert_eq!(text.chars().count(), 256);
    let written: Vec<u8> = text.chars().map(|c| encode_char(c).unwrap()).collect();
    assert_eq!(written, bytes);
}

#[test]
fn a_failed_utf8_read_falls_back_to_ansi() {
    let detection = detect(b"caf\xE9 \x80\x96");
    assert_eq!(detection.encoding, Encoding::Ansi);
    assert_eq!(detection.evidence, Evidence::NotUtf8);
    assert_eq!(interpret(b"caf\xE9 \x80\x96").text, "café €–");
}

#[test]
fn characters_outside_the_code_page_have_no_byte() {
    for character in ['☃', '日', '🙂'] {
        assert_eq!(encode_char(character), None, "{character:?}");
    }
    assert_eq!(encode_char('€'), Some(0x80));
}

#[test]
fn reading_and_writing_ansi_is_reversible() {
    for byte in 0..=255u8 {
        let bytes = [byte, byte, 0x41];
        let read = interpret(&bytes);
        // A byte at or above 0x80 never starts a valid UTF-8 sequence on its
        // own, so the file is read as the code page; plain ASCII is UTF-8.
        let expected = if byte < 0x80 {
            Encoding::Utf8
        } else {
            Encoding::Ansi
        };
        assert_eq!(read.encoding(), expected, "{byte:#04X}");
        let written = nc_textproc::encoding::encode(&read.text, read.encoding(), read.has_bom())
            .expect("a write of text read from these bytes");
        assert_eq!(written, bytes, "{byte:#04X}");
    }
}
