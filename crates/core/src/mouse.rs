//! Mouse selection: what a click means.
//!
//! Hit-testing a pixel to a byte offset is the viewport's job
//! (`docs/architecture.md` §4); what the click then *means* is engine
//! arithmetic, and it lives here so the window shell never re-implements word
//! or line selection.
//!
//! The reference's exact double- and triple-click rule is a capture item
//! (`docs/feature-matrix.md` NC-E005), so the behaviour below is named an
//! approximation: a double-click takes one UAX #29 word segment and a
//! triple-click takes the whole line *including* its terminator, which is what
//! a Windows edit control shows and what the engine can guarantee without the
//! pinned reference machine.

use std::ops::Range;

use crate::buffer::TextBuffer;
use crate::selection::Selection;

/// How many clicks a run had, folded into the three selections a mouse makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickKind {
    /// One click — place the caret.
    Single,
    /// Two — select the word under the pointer.
    Double,
    /// Three — select the line under the pointer.
    Triple,
}

impl ClickKind {
    /// The kind a run of `count` clicks means.
    ///
    /// A run of one is a caret, two is a word, three is a line; a fourth click
    /// starts a fresh run, as it does in a Windows edit control. Zero is
    /// treated as one so a caller never has to special-case an empty run.
    pub const fn from_count(count: usize) -> Self {
        match count {
            2 => Self::Double,
            3 => Self::Triple,
            _ => Self::Single,
        }
    }
}

/// The selection a click of `kind` at `byte` makes.
///
/// A word or line selection is anchored at its start and headed at its end, so
/// a following Shift+Click or drag grows it the way extending a selection
/// does. Every offset is clamped to a character boundary.
pub fn click_selection(buffer: &TextBuffer, byte: usize, kind: ClickKind) -> Selection {
    match kind {
        ClickKind::Single => Selection::caret(buffer.clamp(byte)),
        ClickKind::Double => {
            let word = buffer.word_range(byte);
            Selection::new(word.start, word.end)
        }
        ClickKind::Triple => {
            let line = buffer.line_range(byte);
            Selection::new(line.start, line.end)
        }
    }
}

/// The selection a drag makes, given the `kind` of the click that started it.
///
/// At byte granularity the anchor stays where the button went down and the head
/// follows the pointer, either way round. After a double- or triple-click the
/// drag works in whole words or whole lines: the selection keeps the far edge
/// of the unit the button went down on and the head takes the far edge of the
/// unit under the pointer, so dragging a double-clicked word across the line
/// selects everything between the two words in either direction.
pub fn drag_selection(buffer: &TextBuffer, anchor: usize, to: usize, kind: ClickKind) -> Selection {
    let anchor = buffer.clamp(anchor);
    let to = buffer.clamp(to);
    if kind == ClickKind::Single {
        return Selection::new(anchor, to);
    }
    let anchored = unit_range(buffer, anchor, kind);
    let under_pointer = unit_range(buffer, to, kind);
    if to >= anchor {
        Selection::new(anchored.start, under_pointer.end)
    } else {
        Selection::new(under_pointer.start, anchored.end)
    }
}

/// The word or line at `byte`; an empty range at the byte for a single click.
fn unit_range(buffer: &TextBuffer, byte: usize, kind: ClickKind) -> Range<usize> {
    match kind {
        ClickKind::Single => buffer.clamp(byte)..buffer.clamp(byte),
        ClickKind::Double => buffer.word_range(byte),
        ClickKind::Triple => buffer.line_range(byte),
    }
}
