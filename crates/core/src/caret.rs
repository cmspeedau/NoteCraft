//! Caret movement.
//!
//! Movement is expressed as a pure function of the buffer, the selection, and
//! the movement, so every key the editor binds can be tested without a window.
//! Vertical movement carries a *desired column*: the column the caret was in
//! before the move, so that walking down through short lines and back out again
//! returns the caret to where it started — the behaviour every Windows text
//! control has, and the one the reference's arrow keys show.

use crate::buffer::{LineColumn, TextBuffer};
use crate::selection::Selection;

/// Lines moved by Page Up / Page Down until the viewport can report its own
/// height (milestone M2). The reference's exact page step is a capture item
/// (`docs/feature-matrix.md` NC-E004).
pub const PAGE_LINES: usize = 10;

/// A caret movement, as bound to a key or a menu command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaretMovement {
    /// One grapheme left.
    Left,
    /// One grapheme right.
    Right,
    /// To the start of the previous word.
    WordLeft,
    /// To the end of the next word.
    WordRight,
    /// To the start of the line.
    LineStart,
    /// To the end of the line.
    LineEnd,
    /// To the start of the document.
    DocumentStart,
    /// To the end of the document.
    DocumentEnd,
    /// One line up, keeping the desired column.
    Up,
    /// One line down, keeping the desired column.
    Down,
    /// A page up, keeping the desired column.
    PageUp,
    /// A page down, keeping the desired column.
    PageDown,
}

impl CaretMovement {
    /// Whether the movement preserves a desired column across lines.
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::PageUp | Self::PageDown)
    }
}

/// Applies `movement` and returns the new selection together with the desired
/// column to remember for the next vertical move.
///
/// When `extend` is set the existing anchor is kept, so Shift+Arrow grows the
/// selection instead of collapsing it.
pub fn apply(
    buffer: &TextBuffer,
    selection: Selection,
    movement: CaretMovement,
    extend: bool,
    desired_column: Option<usize>,
    page_lines: usize,
) -> (Selection, Option<usize>) {
    let head = buffer.clamp(selection.head());
    let position = buffer.line_column(head);
    let last_line = buffer.line_count().saturating_sub(1);

    let (target, desired) = match movement {
        CaretMovement::Left => (move_left(buffer, head), None),
        CaretMovement::Right => (move_right(buffer, head), None),
        CaretMovement::WordLeft => (buffer.prev_word_bound(head), None),
        CaretMovement::WordRight => (buffer.next_word_bound(head), None),
        CaretMovement::LineStart => (buffer.line_start(position.line).unwrap_or(0), None),
        CaretMovement::LineEnd => (buffer.line_end(position.line).unwrap_or(buffer.len()), None),
        CaretMovement::DocumentStart => (0, None),
        CaretMovement::DocumentEnd => (buffer.len(), None),
        CaretMovement::Up => vertical(
            buffer,
            position,
            position.line.saturating_sub(1),
            desired_column,
        ),
        CaretMovement::Down => vertical(
            buffer,
            position,
            (position.line + 1).min(last_line),
            desired_column,
        ),
        CaretMovement::PageUp => vertical(
            buffer,
            position,
            position.line.saturating_sub(page_lines.max(1)),
            desired_column,
        ),
        CaretMovement::PageDown => vertical(
            buffer,
            position,
            (position.line + page_lines.max(1)).min(last_line),
            desired_column,
        ),
    };

    let selection = if extend {
        Selection::new(selection.anchor(), target)
    } else {
        Selection::caret(target)
    };
    (selection, desired)
}

fn vertical(
    buffer: &TextBuffer,
    position: LineColumn,
    line: usize,
    desired_column: Option<usize>,
) -> (usize, Option<usize>) {
    let column = desired_column.unwrap_or(position.column);
    (
        buffer.byte_of_line_column(LineColumn { line, column }),
        Some(column),
    )
}

fn move_left(buffer: &TextBuffer, head: usize) -> usize {
    let position = buffer.line_column(head);
    if position.column == 0 {
        if position.line == 0 {
            return head;
        }
        return buffer.line_end(position.line - 1).unwrap_or(0);
    }
    buffer.prev_grapheme(head)
}

fn move_right(buffer: &TextBuffer, head: usize) -> usize {
    let line = buffer.line_of(head);
    let end = buffer.line_end(line).unwrap_or(buffer.len());
    if head < end {
        return buffer.next_grapheme(head);
    }
    // At the end of the line: step over the terminator onto the next line, but
    // do not step past the end of the last line.
    if head < buffer.len() {
        return head + 1;
    }
    head
}
