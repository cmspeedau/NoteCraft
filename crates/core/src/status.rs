//! The status bar's reading of the caret.
//!
//! The status bar shows where the caret is, and the arithmetic from a byte
//! offset to a 1-based line and column belongs to the engine rather than the
//! window shell, so it can be tested without a window
//! (`docs/feature-matrix.md` NC-E010, `docs/architecture.md` §4).
//!
//! What the reference's status bar *says* — the wording, which of its fields
//! are shown, and whether the column counts tabs or spaces — is a captured gap
//! (`docs/reference-matrix.md` §5: "Line:Column, encoding, zoom %, word-wrap
//! indicator — exact fields `TO_CAPTURE`"), so this module commits to the
//! numbers only: the line counting from 1, and the column counting from 1 in
//! graphemes within that line, measured the way `TextBuffer` measures a column.
//! Nothing here decides how the numbers are spelled.

use crate::buffer::{LineColumn, TextBuffer};

/// The caret's position as the status bar reads it: 1-based line and column.
///
/// ```ignore
/// let position = document.status_position();
/// // "line {position.line}, column {position.column}" is the shell's wording,
/// // not this type's.
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusPosition {
    /// The line number, counting from 1.
    pub line: usize,
    /// The column number within the line, counting from 1 in graphemes.
    pub column: usize,
}

impl StatusPosition {
    /// Reads `byte` in `buffer` as a status-bar position.
    ///
    /// The byte is clamped to the buffer, so the result is always a position
    /// inside the document and an empty document reads as line 1, column 1.
    pub fn of(buffer: &TextBuffer, byte: usize) -> Self {
        let LineColumn { line, column } = buffer.line_column(byte);
        Self {
            line: line + 1,
            column: column + 1,
        }
    }
}
