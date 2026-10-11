//! How NoteCraft reads a file's bytes and writes them back.
//!
//! This crate is the second half of the text engine: `nc-core` holds the text
//! and knows nothing about files, and `nc-textproc` holds the files and knows
//! nothing about the viewport. It owns three things
//! (`docs/architecture.md` §4, M4 in `ROADMAP.md`):
//!
//! * **Encoding.** [`interpret`] reads a byte string as the encoding its bytes
//!   and marks say — UTF-8, with or without a byte order mark, UTF-16 and
//!   UTF-32 in either byte order, and ANSI — and hands back both the text and
//!   the evidence for what it found ([`Detection`]).
//! * **Files.** [`TextFile`] is one open file: its path, the bytes as they are
//!   on disk, and the [`Document`](nc_core::Document) the text was read into.
//!   Saving writes through a sibling temporary file and renames it into place,
//!   so a failed write leaves the old file exactly as it was.
//! * **Questions.** [`prompt`] names the states in which the reference asks
//!   before destroying something — unsaved changes, an existing file in Save
//!   As, characters the target encoding cannot hold, a file changed underneath
//!   the editor.
//!
//! # Invariants
//!
//! These are the M4 exit criteria ("no silent data loss is permitted") written
//! as rules the code holds to:
//!
//! * **Reading never substitutes.** A byte string that is not valid in a given
//!   encoding is not repaired character by character; the reader falls back to
//!   ANSI/Windows-1252, which maps every byte to exactly one character and back
//!   (`docs/reference-matrix.md` §4, NC-D001/NC-D002).
//! * **Reading is reversible.** Re-encoding the text of an unedited file with
//!   the encoding and mark that were detected reproduces the input bytes
//!   exactly, whatever they were.
//! * **Saving is reported, not guessed.** Text that the target encoding cannot
//!   represent makes [`save`](TextFile::save) fail and name the characters;
//!   [`save_lossy`](TextFile::save_lossy) writes them as `?` only when asked.
//! * **A failed write changes nothing.** The bytes are written to a temporary
//!   file that is renamed over the target only once it is complete.
//!
//! # Evidence discipline
//!
//! Where the reference's exact behaviour is not captured, the approximation is
//! named as such and left `IN_PROGRESS` in `docs/feature-matrix.md` rather than
//! asserted as parity. In this crate that means: ANSI is Windows-1252 (the
//! Western default), so a locale that uses another code page is read with the
//! Western table — wrong for those languages, but never lossy; and the wording
//! of the prompts, the Save As list of encodings, and whether the reference
//! even notices an outside change are `TO_CAPTURE` items.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod ansi;
pub mod encoding;
pub mod error;
pub mod prompt;
mod textfile;

pub use encoding::{Detection, Encoding, Evidence, Interpreted, detect, interpret};
pub use error::{DecodeError, EncodeError, Error, Result};
pub use textfile::{ExternalChange, Saved, TextFile};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_document_is_utf8_without_a_mark() {
        let detection = Detection::new_document();
        assert_eq!(detection.encoding, Encoding::Utf8);
        assert!(!detection.bom);
        assert_eq!(detection.evidence, Evidence::NewDocument);
        assert_eq!(detect(b"hello").evidence, Evidence::ValidUtf8);
    }
}
