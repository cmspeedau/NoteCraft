//! NoteCraft's Markdown formatting model — `docs/architecture.md` §6.
//!
//! Three rules from that section shape the whole crate:
//!
//! 1. **The source is the document.** A formatting command edits the Markdown
//!    text the user can see, never a rich internal tree. There is no parallel
//!    representation that could hold a version of the document the file does not.
//! 2. **Parsing is tolerant and keeps losses.** [`blocks`] and [`spans`] describe
//!    the source without claiming it. Construct it never rewrites a byte, and
//!    anything the parser does not recognise survives as ordinary text rather
//!    than being normalised away (`NC-M011`).
//! 3. **Serialization is deterministic.** The same text and the same command
//!    produce the same bytes, so a formatting test can assert exact output.
//!
//! Everything here works on `&str` plus byte offsets and returns an [`Outcome`]
//! carrying the new text *and* where the selection ended up. That shape is what
//! lets the shell apply a formatting command as one ordinary document edit: one
//! replacement, and therefore one undo step (`NC-M010`), with the selection kept
//! where the user expects it.
//!
//! ```
//! use nc_markdown::{toggle_inline, Inline, Range};
//!
//! let text = "press the button";
//! let selection = Range::new(10, 16).unwrap(); // "button"
//! let edited = toggle_inline(text, selection, Inline::Bold).unwrap();
//!
//! assert_eq!(edited.text, "press the **button**");
//! // The selection now carries the markers, so pressing the shortcut again
//! // unwraps what it just wrapped.
//! assert_eq!(edited.selection, Range::new(10, 20).unwrap());
//! ```

#![deny(missing_docs)]

mod format;
mod parse;
mod patch;

pub use format::{
    Heading, INDENT, Inline, ListKind, clear_formatting, indent_lines, insert_link, insert_table,
    outdent_lines, set_heading, set_list, toggle_inline,
};
pub use parse::{Block, BlockKind, ListMarker, Span, SpanKind, blocks, spans};
pub use patch::{Outcome, Patch, apply};

/// A byte range in a document's text, with `start <= end`.
///
/// Offsets are bytes, not characters or display columns, because that is what the
/// editor's buffer is indexed by. Every entry point rejects a range that would
/// split a `char` rather than guessing at what was meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// The first byte of the range.
    pub start: usize,
    /// One past the last byte of the range.
    pub end: usize,
}

impl Range {
    /// A range between two byte offsets, rejected when they are out of order.
    pub fn new(start: usize, end: usize) -> Option<Self> {
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// A zero-width range: a caret position.
    pub fn caret(at: usize) -> Self {
        Self { start: at, end: at }
    }

    /// Whether the range holds no text.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// How many bytes the range covers.
    pub fn len(self) -> usize {
        self.end - self.start
    }

    /// Whether both ends land on character boundaries of `text`, which is what
    /// makes it safe to slice.
    pub(crate) fn is_within(self, text: &str) -> bool {
        text.is_char_boundary(self.start) && text.is_char_boundary(self.end)
    }

    /// The bytes this range covers in `text`, or nothing if the range does not
    /// belong to it. Commands that are handed a selection from a different buffer
    /// answer "nothing to do" rather than panicking.
    pub fn slice(self, text: &str) -> &str {
        text.get(self.start..self.end).unwrap_or("")
    }
}
