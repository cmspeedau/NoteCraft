//! Drag and drop: where a drop lands.
//!
//! The engine owns the arithmetic a drop needs — what the insertion offset is
//! once the dragged text has been taken out, and whether the drop means
//! anything at all — while the viewport owns the gesture and the platform owns
//! the cross-application half.
//!
//! Whether a modifier turns a drag into a copy is a capture item
//! (`docs/feature-matrix.md` NC-E009), so nothing here guesses at one: an
//! internal drop *moves* the text, and the copy case is left to the viewport,
//! which can reach it through the clipboard commands (`Ctrl+C` / `Ctrl+V`).

use std::ops::Range;

/// Where the text being dropped came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragSource {
    /// A selection in this document, which a drop moves.
    Internal,
    /// Text dragged in from another application, which a drop inserts.
    External,
}

/// The offset a drop of the range `source` at `target` inserts at.
///
/// A drop inside the dragged range, or exactly at either end of it, is a
/// no-op and returns `None` — the document would come back unchanged, and a
/// drop that silently rewrote the file for nothing would be an undo step the
/// user cannot see. A drop after the range is shifted back by the range's
/// length, because the dragged text leaves the buffer before it is put back.
pub fn drop_target(source: Range<usize>, target: usize) -> Option<usize> {
    if source.is_empty() {
        return None;
    }
    if target >= source.start && target <= source.end {
        return None;
    }
    if target > source.end {
        Some(target - source.len())
    } else {
        Some(target)
    }
}
