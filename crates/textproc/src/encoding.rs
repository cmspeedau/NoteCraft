//! Encodings: what the bytes of a file mean, and how to write them back.
//!
//! NoteCraft reads ANSI or UTF-8 (with or without a byte order mark), and
//! UTF-16 and UTF-32 in either byte order — the encodings the reference reads
//! and writes (`docs/reference-matrix.md` §4, NC-D001/NC-D002). A new document
//! is written as UTF-8 without a BOM, and the encoding a file was read with is
//! remembered and used again when it is saved (NC-D004).
//!
//! Two rules hold for everything in this module:
//!
//! 1. **Reading never substitutes characters.** Bytes that cannot be decoded as
//!    the encoding their BOM claims are read as ANSI instead, and ANSI maps
//!    every byte to a character and back. No document opened by [`interpret`]
//!    contains a replacement character in place of unreadable input.
//! 2. **Reading is reversible.** Encoding what [`interpret`] returned, with the
//!    encoding and BOM it reported, gives back the exact bytes that went in, for
//!    every possible byte string. This is what makes saving an unedited file
//!    byte for byte faithful.
//!
//! A BOM-less file is UTF-8 if its bytes are valid UTF-8 and ANSI otherwise.
//! There is no guess from the shape of the bytes: a UTF-16 file is UTF-16
//! because it carries a BOM, which is how the reference reads them.

use crate::ansi;
use crate::error::DecodeError;

/// The bytes a UTF-8 file may start with.
pub const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// The bytes a little-endian UTF-16 file starts with.
pub const UTF16LE_BOM: &[u8] = &[0xFF, 0xFE];
/// The bytes a big-endian UTF-16 file starts with.
pub const UTF16BE_BOM: &[u8] = &[0xFE, 0xFF];
/// The bytes a little-endian UTF-32 file starts with.
///
/// It begins with [`UTF16LE_BOM`], so it is tested for first.
pub const UTF32LE_BOM: &[u8] = &[0xFF, 0xFE, 0x00, 0x00];
/// The bytes a big-endian UTF-32 file starts with.
pub const UTF32BE_BOM: &[u8] = &[0x00, 0x00, 0xFE, 0xFF];

/// The encoding of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// The Windows code page, [`crate::ansi`].
    Ansi,
    /// UTF-8, with or without a BOM.
    Utf8,
    /// UTF-16, little-endian (`0xFF 0xFE`).
    Utf16Le,
    /// UTF-16, big-endian (`0xFE 0xFF`).
    Utf16Be,
    /// UTF-32, little-endian (`0xFF 0xFE 0x00 0x00`).
    Utf32Le,
    /// UTF-32, big-endian (`0x00 0x00 0xFE 0xFF`).
    Utf32Be,
}

impl Encoding {
    /// Every encoding NoteCraft reads, in the order the Save As list shows them.
    pub const ALL: &'static [Self] = &[
        Self::Ansi,
        Self::Utf8,
        Self::Utf16Le,
        Self::Utf16Be,
        Self::Utf32Le,
        Self::Utf32Be,
    ];

    /// The name of the encoding, as a menu shows it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ansi => "ANSI",
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Utf32Le => "UTF-32 LE",
            Self::Utf32Be => "UTF-32 BE",
        }
    }

    /// The byte order mark this encoding starts a file with, if it has one.
    pub const fn bom_bytes(self) -> &'static [u8] {
        match self {
            Self::Ansi => &[],
            Self::Utf8 => UTF8_BOM,
            Self::Utf16Le => UTF16LE_BOM,
            Self::Utf16Be => UTF16BE_BOM,
            Self::Utf32Le => UTF32LE_BOM,
            Self::Utf32Be => UTF32BE_BOM,
        }
    }

    /// Whether the encoding needs a byte order mark to be recognised.
    ///
    /// A UTF-8 BOM is optional and NoteCraft writes none for a new document;
    /// a wide encoding without one cannot be told from ANSI, so a file saved in
    /// one always gets its mark.
    pub const fn needs_bom(self) -> bool {
        !matches!(self, Self::Ansi | Self::Utf8)
    }
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// How NoteCraft decided what a file's bytes are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// A UTF-8 byte order mark.
    Utf8Bom,
    /// A UTF-16 little-endian byte order mark.
    Utf16LeBom,
    /// A UTF-16 big-endian byte order mark.
    Utf16BeBom,
    /// A UTF-32 little-endian byte order mark.
    Utf32LeBom,
    /// A UTF-32 big-endian byte order mark.
    Utf32BeBom,
    /// No mark, and every byte is valid UTF-8.
    ValidUtf8,
    /// No mark and not valid UTF-8: read as ANSI.
    NotUtf8,
    /// A mark announced a wide encoding the body does not decode as: read as
    /// ANSI, so that the bytes survive even though the text does not make sense.
    UndecodableBom,
    /// Chosen rather than detected: what a Save As picked, for the file that
    /// was written. Nothing is detected until the file is opened again.
    Chosen,
    /// A new document, which is written as UTF-8 without a mark
    /// (`docs/reference-matrix.md` §4).
    NewDocument,
}

impl Evidence {
    /// Whether the decision came from a byte order mark.
    pub const fn from_bom(self) -> bool {
        matches!(
            self,
            Self::Utf8Bom
                | Self::Utf16LeBom
                | Self::Utf16BeBom
                | Self::Utf32LeBom
                | Self::Utf32BeBom
        )
    }
}

/// What the bytes of a file were taken to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detection {
    /// The encoding the bytes were read as.
    pub encoding: Encoding,
    /// Whether the bytes began with a byte order mark.
    pub bom: bool,
    /// What that decision rested on.
    pub evidence: Evidence,
}

impl Detection {
    /// The encoding a new, unsaved document is written as.
    pub const fn new_document() -> Self {
        Self {
            encoding: Encoding::Utf8,
            bom: false,
            evidence: Evidence::NewDocument,
        }
    }

    /// The encoding a Save As chose, rather than one detected in the bytes.
    ///
    /// The reference's Save As list is an exact-order `TO_CAPTURE` item
    /// (`docs/reference-matrix.md` §4); what is fixed is that the encoding the
    /// user picks is the one the file is written in, and the one the status bar
    /// then names.
    pub const fn chosen(encoding: Encoding, bom: bool) -> Self {
        Self {
            encoding,
            bom,
            evidence: Evidence::Chosen,
        }
    }
}

/// A byte string read as text, and the evidence for reading it that way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpreted {
    /// The encoding the bytes were read as.
    pub detection: Detection,
    /// The text they stand for.
    pub text: String,
}

impl Interpreted {
    /// The encoding the bytes were read as.
    pub fn encoding(&self) -> Encoding {
        self.detection.encoding
    }

    /// Whether the bytes began with a byte order mark.
    pub fn has_bom(&self) -> bool {
        self.detection.bom
    }
}

/// Reads a byte string as text: what the encoding is, and what it says.
///
/// This is the one entry point that decides how a file is opened. It never
/// fails, and it never substitutes characters, so the text it returns always
/// encodes back to the bytes it was given.
pub fn interpret(bytes: &[u8]) -> Interpreted {
    let detection = detect(bytes);
    let text = decode(bytes, detection.encoding).unwrap_or_else(|_| ansi::decode(bytes));
    Interpreted { detection, text }
}

/// Decides the encoding of a byte string without decoding it.
pub fn detect(bytes: &[u8]) -> Detection {
    if let Some(detection) = wide(bytes) {
        return detection;
    }
    if bytes.starts_with(UTF8_BOM) {
        // A mark the body does not live up to is not a UTF-8 file, whatever its
        // first three bytes say.
        return match std::str::from_utf8(&bytes[UTF8_BOM.len()..]) {
            Ok(_) => Detection {
                encoding: Encoding::Utf8,
                bom: true,
                evidence: Evidence::Utf8Bom,
            },
            Err(_) => ansi_fallback(Evidence::UndecodableBom),
        };
    }
    if std::str::from_utf8(bytes).is_ok() {
        return Detection {
            encoding: Encoding::Utf8,
            bom: false,
            evidence: Evidence::ValidUtf8,
        };
    }
    ansi_fallback(Evidence::NotUtf8)
}

/// The detection for a marked wide encoding, or `None` when there is no such
/// mark. A mark whose body does not decode falls back to ANSI.
fn wide(bytes: &[u8]) -> Option<Detection> {
    let marked = [
        (Encoding::Utf32Le, UTF32LE_BOM, Evidence::Utf32LeBom),
        (Encoding::Utf32Be, UTF32BE_BOM, Evidence::Utf32BeBom),
        (Encoding::Utf16Le, UTF16LE_BOM, Evidence::Utf16LeBom),
        (Encoding::Utf16Be, UTF16BE_BOM, Evidence::Utf16BeBom),
    ];
    marked.into_iter().find_map(|(encoding, bom, evidence)| {
        if !bytes.starts_with(bom) {
            return None;
        }
        Some(match decode(bytes, encoding) {
            Ok(_) => Detection {
                encoding,
                bom: true,
                evidence,
            },
            Err(_) => ansi_fallback(Evidence::UndecodableBom),
        })
    })
}

/// The detection for bytes that are not text in any encoding they claim.
fn ansi_fallback(evidence: Evidence) -> Detection {
    Detection {
        encoding: Encoding::Ansi,
        bom: false,
        evidence,
    }
}

/// Reads a byte string as `encoding`, dropping a leading mark of that encoding.
pub fn decode(bytes: &[u8], encoding: Encoding) -> std::result::Result<String, DecodeError> {
    match encoding {
        Encoding::Ansi => Ok(ansi::decode(bytes)),
        Encoding::Utf8 => {
            let body = without_bom(bytes, UTF8_BOM);
            std::str::from_utf8(body)
                .map(str::to_owned)
                .map_err(|_| DecodeError::NotUtf8)
        }
        Encoding::Utf16Le => decode_utf16(without_bom(bytes, UTF16LE_BOM), false),
        Encoding::Utf16Be => decode_utf16(without_bom(bytes, UTF16BE_BOM), true),
        Encoding::Utf32Le => decode_utf32(without_bom(bytes, UTF32LE_BOM), false),
        Encoding::Utf32Be => decode_utf32(without_bom(bytes, UTF32BE_BOM), true),
    }
}

/// Writes `text` as `encoding`, with a byte order mark when `bom` is set.
///
/// Fails when a character has no representation in the encoding, which is the
/// ANSI case. The characters are reported so the UI can ask; nothing is
/// silently replaced. Use [`encode_lossy`] once the user has said to go ahead.
pub fn encode(
    text: &str,
    encoding: Encoding,
    bom: bool,
) -> std::result::Result<Vec<u8>, crate::error::EncodeError> {
    let bytes = encode_units(text, encoding, bom, &mut |_| None);
    let characters = unrepresentable(text, encoding);
    if characters.is_empty() {
        Ok(bytes)
    } else {
        Err(crate::error::EncodeError {
            encoding,
            characters,
        })
    }
}

/// Writes `text` as `encoding`, replacing characters the encoding cannot
/// represent with `?`, and returning the characters that were replaced.
///
/// This is only for the case where the user has been told what will be lost and
/// said to save anyway. What the reference puts in their place is a `TO_CAPTURE`
/// item (`docs/reference-matrix.md` §4).
pub fn encode_lossy(text: &str, encoding: Encoding, bom: bool) -> (Vec<u8>, Vec<char>) {
    let characters = unrepresentable(text, encoding);
    let bytes = encode_units(text, encoding, bom, &mut |_| Some('?'));
    (bytes, characters)
}

/// The characters of `text` that `encoding` cannot represent.
pub fn unrepresentable(text: &str, encoding: Encoding) -> Vec<char> {
    text.chars()
        .filter(|&character| !can_encode(character, encoding))
        .collect()
}

/// Whether `encoding` has a representation for `character`.
pub fn can_encode(character: char, encoding: Encoding) -> bool {
    match encoding {
        Encoding::Ansi => ansi::encode_char(character).is_some(),
        // Every other encoding here covers the whole of Unicode by construction.
        _ => true,
    }
}

/// Writes `text` as a sequence of code units, calling `replacement` for a
/// character the encoding cannot represent and writing nothing when it says
/// `None`.
fn encode_units(
    text: &str,
    encoding: Encoding,
    bom: bool,
    replacement: &mut dyn FnMut(char) -> Option<char>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    if bom {
        bytes.extend_from_slice(encoding.bom_bytes());
    }
    match encoding {
        Encoding::Ansi => {
            for character in text.chars() {
                let byte = ansi::encode_char(character)
                    .or_else(|| replacement(character).and_then(ansi::encode_char));
                if let Some(byte) = byte {
                    bytes.push(byte);
                }
            }
        }
        Encoding::Utf8 => bytes.extend_from_slice(text.as_bytes()),
        Encoding::Utf16Le | Encoding::Utf16Be => {
            let mut units = [0_u16; 2];
            for character in text.chars() {
                for unit in character.encode_utf16(&mut units).iter() {
                    let unit = if encoding == Encoding::Utf16Be {
                        unit.to_be_bytes()
                    } else {
                        unit.to_le_bytes()
                    };
                    bytes.extend_from_slice(&unit);
                }
            }
        }
        Encoding::Utf32Le | Encoding::Utf32Be => {
            for character in text.chars() {
                let scalar = u32::from(character);
                let unit = if encoding == Encoding::Utf32Be {
                    scalar.to_be_bytes()
                } else {
                    scalar.to_le_bytes()
                };
                bytes.extend_from_slice(&unit);
            }
        }
    }
    bytes
}

/// `bytes` without a leading `bom`, if it has one.
fn without_bom<'a>(bytes: &'a [u8], bom: &[u8]) -> &'a [u8] {
    bytes.strip_prefix(bom).unwrap_or(bytes)
}

/// Reads little- or big-endian UTF-16.
fn decode_utf16(bytes: &[u8], big_endian: bool) -> std::result::Result<String, DecodeError> {
    if bytes.len() % 2 != 0 {
        return Err(DecodeError::Truncated);
    }
    let units = bytes.chunks_exact(2).map(|unit| {
        let pair = [unit[0], unit[1]];
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    });
    let mut text = String::with_capacity(bytes.len() / 2);
    for decoded in char::decode_utf16(units) {
        match decoded {
            Ok(character) => text.push(character),
            Err(_) => return Err(DecodeError::UnpairedSurrogate),
        }
    }
    Ok(text)
}

/// Reads little- or big-endian UTF-32.
fn decode_utf32(bytes: &[u8], big_endian: bool) -> std::result::Result<String, DecodeError> {
    if bytes.len() % 4 != 0 {
        return Err(DecodeError::Truncated);
    }
    let mut text = String::with_capacity(bytes.len() / 4);
    for unit in bytes.chunks_exact(4) {
        let quartet = [unit[0], unit[1], unit[2], unit[3]];
        let scalar = if big_endian {
            u32::from_be_bytes(quartet)
        } else {
            u32::from_le_bytes(quartet)
        };
        match char::from_u32(scalar) {
            Some(character) => text.push(character),
            None => return Err(DecodeError::NotAScalarValue),
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect_of(bytes: &[u8]) -> Detection {
        detect(bytes)
    }

    #[test]
    fn an_empty_file_is_utf8_without_a_mark() {
        let detection = detect_of(b"");
        assert_eq!(detection.encoding, Encoding::Utf8);
        assert!(!detection.bom);
        assert_eq!(detection.evidence, Evidence::ValidUtf8);
    }

    #[test]
    fn a_bom_wins_over_the_bytes_after_it() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice(b"hi");
        let detection = detect_of(&bytes);
        assert_eq!(detection.encoding, Encoding::Utf8);
        assert!(detection.bom);
        assert_eq!(detection.evidence, Evidence::Utf8Bom);
    }

    #[test]
    fn utf32_is_not_mistaken_for_utf16() {
        let mut bytes = UTF32LE_BOM.to_vec();
        bytes.extend_from_slice(&[0x41, 0x00, 0x00, 0x00]);
        let detection = detect_of(&bytes);
        assert_eq!(detection.encoding, Encoding::Utf32Le);
        assert_eq!(interpret(&bytes).text, "A");
    }

    #[test]
    fn a_marked_file_that_does_not_decode_is_read_as_ansi() {
        // A UTF-16 LE mark followed by a lone high surrogate and an odd byte.
        let bytes = [0xFF, 0xFE, 0x00, 0xD8, 0x41];
        let detection = detect_of(&bytes);
        assert_eq!(detection.encoding, Encoding::Ansi);
        assert_eq!(detection.evidence, Evidence::UndecodableBom);
        assert_eq!(interpret(&bytes).text.chars().count(), bytes.len());
    }

    #[test]
    fn bytes_that_are_not_utf8_are_ansi() {
        let detection = detect_of(b"\xE9\x97");
        assert_eq!(detection.encoding, Encoding::Ansi);
        assert_eq!(detection.evidence, Evidence::NotUtf8);
    }

    #[test]
    fn a_utf8_file_reads_without_its_mark() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice("naïve".as_bytes());
        assert_eq!(interpret(&bytes).text, "naïve");
        assert_eq!(encode("naïve", Encoding::Utf8, true).unwrap(), bytes);
    }
}
