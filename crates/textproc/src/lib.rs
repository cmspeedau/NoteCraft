//! How NoteCraft reads a file's bytes and writes them back.
//!
//! This crate is the rest of the text engine: `nc-core` holds the text and
//! knows nothing about files, and `nc-textproc` holds the files and knows
//! nothing about the viewport. It owns five things
//! (`docs/architecture.md` §4, M4 and M5 in `ROADMAP.md`):
//!
//! * **Encoding.** [`interpret`] reads a byte string as the encoding its bytes
//!   and marks say — UTF-8, with or without a byte order mark, UTF-16 and
//!   UTF-32 in either byte order, and ANSI — and hands back both the text and
//!   the evidence for what it found ([`Detection`]).
//! * **Files.** [`TextFile`] is one open file: its path, the bytes as they are
//!   on disk, and the [`Document`](nc_core::Document) the text was read into.
//!   Saving writes through a sibling temporary file and renames it into place,
//!   so a failed write leaves the old file exactly as it was.
//! * **Questions.** [`prompt`] names the states in which a prompt is raised
//!   before destroying something — unsaved changes, an existing file in Save
//!   As, characters the target encoding cannot hold, a file changed underneath
//!   the editor.
//! * **Searching.** [`search`] finds text — the engine behind Find, Find next
//!   and Replace All — and [`time_date`] writes the time and date F5 inserts.
//!   Both work on a document's own bytes and offsets, so a search can be tested
//!   without a window and a replacement without a dialog.
//! * **Spelling.** [`spell`] marks the words that are a slip away from a word it
//!   knows and offers corrections — behind the squiggle under a misspelled word
//!   and the list of suggestions it opens.
//!
//! # Invariants
//!
//! These are the M4 exit criteria ("no silent data loss is permitted") written
//! as rules the code holds to:
//!
//! * **Reading never substitutes.** A byte string that is not valid in a given
//!   encoding is not repaired character by character; the reader falls back to
//!   ANSI/Windows-1252, which maps every byte to exactly one character and back
//!   (`docs/ui-reference.md`, NC-D001/NC-D002).
//! * **Reading is reversible.** Re-encoding the text of an unedited file with
//!   the encoding and mark that were detected reproduces the input bytes
//!   exactly, whatever they were.
//! * **Saving is reported, not guessed.** Text that the target encoding cannot
//!   represent makes [`save`](TextFile::save) fail and name the characters;
//!   [`save_lossy`](TextFile::save_lossy) writes them as `?` only when asked.
//! * **A failed write changes nothing.** The bytes are written to a temporary
//!   file that is renamed over the target only once it is complete.
//!
//! # Design notes
//!
//! Where the exact behaviour is not fixed by the reference, the approximation is
//! named as such and left `IN_PROGRESS` in `docs/feature-matrix.md` rather than
//! asserted as exact. In this crate that means: ANSI is Windows-1252 (the
//! Western default), so a locale that uses another code page is read with the
//! Western table — wrong for those languages, but never lossy; and the wording
//! of the prompts, the Save As list of encodings, and whether an outside change
//! is watched for at all are design decisions.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod ansi;
pub mod encoding;
pub mod error;
pub mod print;
pub mod prompt;
pub mod search;
pub mod spell;
mod textfile;
pub mod time_date;

pub use encoding::{Detection, Encoding, Evidence, Interpreted, detect, interpret};
pub use error::{DecodeError, EncodeError, Error, Result};
pub use print::{
    Margins, Metrics, Orientation, Page, PageSetup, PaperSize, expand_placeholders, mm_to_pt,
    paginate, print_document, to_postscript,
};
pub use search::{
    Direction, Match, SearchOptions, SearchResult, find_all, find_next, find_previous, go_to_line,
    parse_line_number,
};
pub use spell::{is_known, misspelled_word, misspellings, suggestions};
pub use textfile::{ExternalChange, Saved, TextFile};
pub use time_date::{Meridiem, TimeDate};

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
