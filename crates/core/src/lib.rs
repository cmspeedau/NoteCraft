//! The NoteCraft document model.
//!
//! This crate is the editing engine: the text buffer, the selection, the caret
//! arithmetic, the undo history, the clipboard interface, soft wrap, and the
//! document that ties them together. It is deliberately *not* a UI: the editor
//! viewport, the status bar, find/replace, and file I/O hang off these types in
//! later milestones (see `docs/architecture.md` §4).
//!
//! # Invariants
//!
//! * `TextBuffer` always holds valid UTF-8 — so an edit can never leave a
//!   broken surrogate pair behind, whatever was pasted in
//!   (`docs/feature-matrix.md` NC-E008).
//! * `TextBuffer` holds `\n` terminators only. The style read from the file
//!   lives on the `Document` and is written back on save
//!   (`docs/feature-matrix.md` NC-D004).
//! * Positions are byte offsets into the buffer's text; every operation clamps
//!   its arguments to the nearest character boundary rather than panicking.
//!
//! # Evidence discipline
//!
//! Two rules from `AGENT.md` §2 shape this crate:
//!
//! 1. Nothing is invented. Where the reference's exact behaviour is not
//!    captured, the approximation is named as such in a doc comment and left
//!    marked `IN_PROGRESS` in `docs/feature-matrix.md`, not asserted as parity.
//! 2. Behaviour the reference clearly has (a caret, an undo history, an EOL
//!    round-trip) is implemented rather than stubbed, because "not yet
//!    captured" is not a licence to skip it.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod buffer;
mod caret;
mod clipboard;
mod dnd;
mod document;
mod line_ending;
mod mouse;
mod selection;
mod tabs;
mod undo;
mod wrap;

pub use buffer::{LineColumn, TextBuffer};
pub use caret::{CaretMovement, PAGE_LINES};
pub use clipboard::{Clipboard, MemoryClipboard};
pub use dnd::{DragSource, drop_target};
pub use document::Document;
pub use line_ending::{LineEnding, LineEndingReport};
pub use mouse::{ClickKind, click_selection, drag_selection};
pub use selection::Selection;
pub use tabs::{TabStop, display_width};
pub use undo::{Edit, UndoStack};
pub use wrap::{WrappedLine, wrap_document, wrap_line};
