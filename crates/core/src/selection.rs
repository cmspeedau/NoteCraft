//! The selection: an anchor and a head.

use std::ops::Range;

/// A selection, or a bare caret when the two ends coincide.
///
/// The *head* is the end the user moved most recently and is where the caret
/// is drawn; the *anchor* is the end that stays put while the selection is
/// extended with Shift. Keeping both means extending and shrinking a selection
/// needs no extra state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    anchor: usize,
    head: usize,
}

impl Selection {
    /// A collapsed selection — a caret — at `at`.
    pub const fn caret(at: usize) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    /// A selection from `anchor` to `head`, either way round.
    pub const fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// The fixed end.
    pub const fn anchor(&self) -> usize {
        self.anchor
    }

    /// The end the user moved.
    pub const fn head(&self) -> usize {
        self.head
    }

    /// Whether the selection is empty.
    pub const fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The selected range, ordered from the earlier offset to the later one.
    pub fn range(&self) -> Range<usize> {
        if self.anchor <= self.head {
            self.anchor..self.head
        } else {
            self.head..self.anchor
        }
    }

    /// The lower offset of the range.
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// The upper offset of the range.
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// The length of the range in bytes.
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::caret(0)
    }
}
