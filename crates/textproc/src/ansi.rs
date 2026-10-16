//! ANSI: the Windows code page, which is what a file with no BOM and no valid
//! UTF-8 in it is read as (`docs/ui-reference.md`).
//!
//! The code page implemented here is Windows-1252, the Western one, which is
//! the default on a Windows install that has not been told otherwise. A locale
//! that selects a different code page (1251 Cyrillic, 932 Japanese, …) is
//! therefore read with the Western mapping, which is wrong for those languages
//! but never loses a byte: every byte maps to exactly one character and back.
//! Which code page to use for a given locale is a design decision
//! (`docs/ui-reference.md`); if it changes, only this module's tables change.
//!
//! The five bytes that Windows-1252 leaves undefined (`0x81`, `0x8D`, `0x8F`,
//! `0x90`, `0x9D`) map to the C1 control characters of the same value, so the
//! round trip below holds for every input, undefined bytes included.

/// The characters Windows-1252 gives `0x80`–`0x9F`, in order.
///
/// `0x81`, `0x8D`, `0x8F`, `0x90` and `0x9D` are undefined in the code page and
/// map to the control character of the same value.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// The character byte `byte` stands for.
pub fn decode_byte(byte: u8) -> char {
    match byte {
        0x00..=0x7F => char::from(byte),
        0x80..=0x9F => HIGH[usize::from(byte) - 0x80],
        _ => char::from(byte),
    }
}

/// The byte that stands for `character`, if the code page has one.
pub fn encode_char(character: char) -> Option<u8> {
    let scalar = u32::from(character);
    if scalar < 0xA0 {
        return u8::try_from(scalar).ok();
    }
    if scalar <= 0xFF {
        return Some(u8::try_from(scalar).unwrap_or(b'?'));
    }
    HIGH.iter()
        .position(|&high| high == character)
        .map(|index| 0x80 + u8::try_from(index).unwrap_or(0))
}

/// The text a byte string stands for under the code page.
pub fn decode(bytes: &[u8]) -> String {
    bytes.iter().copied().map(decode_byte).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_round_trips() {
        for byte in 0..=u8::MAX {
            let character = decode_byte(byte);
            assert_eq!(
                encode_char(character),
                Some(byte),
                "{byte:#04X} -> {character:?} did not come back"
            );
        }
    }

    #[test]
    fn the_smart_quotes_and_dashes_are_windows_1252() {
        assert_eq!(decode_byte(0x91), '\u{2018}');
        assert_eq!(decode_byte(0x92), '\u{2019}');
        assert_eq!(decode_byte(0x93), '\u{201C}');
        assert_eq!(decode_byte(0x94), '\u{201D}');
        assert_eq!(decode_byte(0x96), '\u{2013}');
        assert_eq!(decode_byte(0x97), '\u{2014}');
        assert_eq!(decode_byte(0x85), '\u{2026}');
    }

    #[test]
    fn characters_outside_the_code_page_have_no_byte() {
        for character in ['\u{4E00}', '\u{1F600}', '\u{0100}'] {
            assert_eq!(encode_char(character), None, "{character:?}");
        }
    }

    #[test]
    fn the_code_page_byte_example_decodes_to_the_expected_text() {
        // "café — naïve" as a Windows-1252 file.
        let bytes = b"caf\xE9 \x97 na\xEFve";
        assert_eq!(decode(bytes), "café — naïve");
    }
}
