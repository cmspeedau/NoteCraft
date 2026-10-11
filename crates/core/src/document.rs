//! The document: text, selection, history, and the editing operations.
//!
//! A [`Document`] is what a tab holds and what a command acts on. It owns the
//! buffer, the selection, the undo history, and the line-ending style, and it
//! exposes the editing operations the command registry names — insert, delete,
//! undo, redo, select all, copy, cut, paste.
//!
//! Nothing here talks to a window, a file, or the platform clipboard: the
//! Win32 shell drives these methods and reads the results
//! (`docs/architecture.md` §4).

use std::ops::Range;

use crate::buffer::{LineColumn, TextBuffer};
use crate::caret::{self, CaretMovement, PAGE_LINES};
use crate::clipboard::Clipboard;
use crate::line_ending::{self, LineEnding, LineEndingReport};
use crate::selection::Selection;
use crate::tabs::TabStop;
use crate::undo::{Edit, UndoStack};
use crate::wrap::{WrappedLine, wrap_document};

/// An open document.
#[derive(Debug, Clone)]
pub struct Document {
    buffer: TextBuffer,
    selection: Selection,
    history: UndoStack,
    line_ending: LineEnding,
    line_ending_report: LineEndingReport,
    original: Option<String>,
    dirty: bool,
    word_wrap: bool,
    tab_stop: TabStop,
    desired_column: Option<usize>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// A new, empty document.
    ///
    /// It uses CRLF, which is the style the reference writes a new document
    /// with (`docs/reference-matrix.md` §4), and starts unmodified.
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            selection: Selection::caret(0),
            history: UndoStack::new(),
            line_ending: LineEnding::DEFAULT,
            line_ending_report: LineEndingReport::default(),
            original: None,
            dirty: false,
            word_wrap: false,
            tab_stop: TabStop::DEFAULT,
            desired_column: None,
        }
    }

    /// A document over `text`, keeping its line-ending style.
    ///
    /// The exact bytes handed in are remembered, so saving a document that has
    /// not been edited reproduces them byte for byte — including a file with
    /// mixed line endings, whose save behaviour the reference has not been
    /// captured for.
    pub fn from_text(text: &str) -> Self {
        let report = line_ending::detect(text);
        let mut document = Self::new();
        document.buffer = TextBuffer::from_normalized(&line_ending::to_lf(text));
        document.line_ending = report.style;
        document.line_ending_report = report;
        document.original = Some(text.to_owned());
        document
    }

    // ---- accessors ------------------------------------------------------

    /// The document text, with `\n` terminators.
    pub fn text(&self) -> &str {
        self.buffer.as_str()
    }

    /// The style the document is saved in.
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// What the text read in looked like.
    pub fn line_ending_report(&self) -> LineEndingReport {
        self.line_ending_report
    }

    /// Whether the text read in used more than one line-ending style.
    pub fn has_mixed_line_endings(&self) -> bool {
        self.line_ending_report.mixed
    }

    /// The number of lines, which is at least one.
    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    /// The length of the text in bytes.
    pub fn byte_len(&self) -> usize {
        self.buffer.len()
    }

    /// The buffer.
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// `line`'s text, without its terminator.
    pub fn line(&self, line: usize) -> Option<&str> {
        self.buffer.line(line)
    }

    /// Every line's text, without terminators.
    pub fn lines(&self) -> std::str::Split<'_, char> {
        self.buffer.lines()
    }

    /// The selection.
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Whether anything is selected.
    pub fn has_selection(&self) -> bool {
        !self.selection.is_empty()
    }

    /// The selected text, if the selection is not empty.
    pub fn selected_text(&self) -> Option<&str> {
        if self.selection.is_empty() {
            return None;
        }
        self.buffer.slice(self.selection.range())
    }

    /// The caret's byte offset.
    pub fn caret(&self) -> usize {
        self.selection.head()
    }

    /// The caret's line and column.
    pub fn caret_position(&self) -> LineColumn {
        self.buffer.line_column(self.selection.head())
    }

    /// Whether word wrap is on.
    pub fn word_wrap(&self) -> bool {
        self.word_wrap
    }

    /// Turns word wrap on or off.
    pub fn set_word_wrap(&mut self, enabled: bool) {
        self.word_wrap = enabled;
    }

    /// The tab-stop width in use.
    pub fn tab_stop(&self) -> TabStop {
        self.tab_stop
    }

    /// Sets the tab-stop width.
    pub fn set_tab_stop(&mut self, tab_stop: TabStop) {
        self.tab_stop = tab_stop;
    }

    /// Whether the document has been edited since it was last saved.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Whether there is an edit to undo.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether there is an edit to redo.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    // ---- selection ------------------------------------------------------

    /// Collapses the selection to `byte` and clears the desired column.
    pub fn set_caret(&mut self, byte: usize) {
        self.selection = Selection::caret(self.buffer.clamp(byte));
        self.desired_column = None;
    }

    /// Sets the selection explicitly, as a drag or Shift+Click does.
    pub fn select(&mut self, anchor: usize, head: usize) {
        self.selection = Selection::new(self.buffer.clamp(anchor), self.buffer.clamp(head));
        self.desired_column = None;
    }

    /// Selects the whole document.
    pub fn select_all(&mut self) {
        self.selection = Selection::new(0, self.buffer.len());
        self.desired_column = None;
    }

    /// Moves the caret, collapsing the selection.
    pub fn move_caret(&mut self, movement: CaretMovement) {
        self.move_caret_by(movement, false);
    }

    /// Moves the caret, extending the selection.
    pub fn move_caret_extending(&mut self, movement: CaretMovement) {
        self.move_caret_by(movement, true);
    }

    /// Moves the caret by `movement`, optionally extending the selection.
    pub fn move_caret_by(&mut self, movement: CaretMovement, extend: bool) {
        let (selection, desired) = caret::apply(
            &self.buffer,
            self.selection,
            movement,
            extend,
            self.desired_column,
            PAGE_LINES,
        );
        self.selection = selection;
        self.desired_column = if movement.is_vertical() {
            desired
        } else {
            None
        };
    }

    // ---- editing --------------------------------------------------------

    /// Inserts `text` at the caret, replacing the selection.
    ///
    /// Any CRLF or CR in `text` — a paste, most likely — is normalised to `\n`
    /// so the buffer keeps its single-terminator invariant. Returns whether
    /// anything changed.
    pub fn insert_str(&mut self, text: &str) -> bool {
        self.insert_impl(text, true)
    }

    /// Inserts `text` without letting it merge into the previous edit.
    ///
    /// Paste uses this: one paste is one undo step even when the caret has not
    /// moved between pastes.
    pub fn insert_uncoalesced(&mut self, text: &str) -> bool {
        self.insert_impl(text, false)
    }

    /// Breaks the line at the caret (Enter).
    pub fn insert_newline(&mut self) -> bool {
        self.insert_impl("\n", false)
    }

    fn insert_impl(&mut self, text: &str, coalesce: bool) -> bool {
        let normalized = line_ending::to_lf(text);
        let range = self.selection.range();
        if normalized.is_empty() && range.is_empty() {
            return false;
        }
        let removed = self.buffer.slice(range.clone()).unwrap_or("").to_owned();
        let start = self.selection.start();
        let before = self.selection;
        self.buffer.replace(range, &normalized);
        let after = Selection::caret(start + normalized.len());
        self.selection = after;
        let typing =
            coalesce && !normalized.contains('\n') && removed.is_empty() && before.is_empty();
        self.record(
            Edit {
                start,
                removed,
                inserted: normalized,
                selection_before: before,
                selection_after: after,
            },
            typing,
        );
        true
    }

    /// Deletes the selection, or the grapheme before the caret.
    pub fn backspace(&mut self) -> bool {
        if self.has_selection() {
            return self.delete_selection();
        }
        let head = self.caret();
        if head == 0 {
            return false;
        }
        let from = self.buffer.prev_grapheme(head);
        self.delete_range(from..head)
    }

    /// Deletes the selection, or the grapheme after the caret.
    pub fn delete_forward(&mut self) -> bool {
        if self.has_selection() {
            return self.delete_selection();
        }
        let head = self.caret();
        if head >= self.buffer.len() {
            return false;
        }
        let line = self.buffer.line_of(head);
        let end = self.buffer.line_end(line).unwrap_or(self.buffer.len());
        let to = if head >= end {
            // Sitting before the terminator: delete the terminator and join the
            // lines, which is one byte because the buffer holds `\n` only.
            head + 1
        } else {
            self.buffer.next_grapheme(head)
        };
        self.delete_range(head..to)
    }

    /// Deletes the selection.
    pub fn delete_selection(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let range = self.selection.range();
        self.delete_range(range)
    }

    fn delete_range(&mut self, range: Range<usize>) -> bool {
        let removed = self.buffer.remove(range.clone());
        if removed.is_empty() {
            return false;
        }
        let before = self.selection;
        let after = Selection::caret(range.start);
        self.selection = after;
        self.desired_column = None;
        // Consecutive deletions do not merge: the reference's grouping of a
        // Backspace run is a capture item (NC-E001), so each deletion stays
        // its own step rather than guessing.
        self.record(
            Edit {
                start: range.start,
                removed,
                inserted: String::new(),
                selection_before: before,
                selection_after: after,
            },
            false,
        );
        true
    }

    fn record(&mut self, edit: Edit, coalesce: bool) {
        self.history.record(edit, coalesce);
        self.dirty = true;
    }

    // ---- history --------------------------------------------------------

    /// Undoes the last edit.
    pub fn undo(&mut self) -> bool {
        let ok = self.history.undo(&mut self.buffer, &mut self.selection);
        if ok {
            self.dirty = true;
            self.desired_column = None;
        }
        ok
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) -> bool {
        let ok = self.history.redo(&mut self.buffer, &mut self.selection);
        if ok {
            self.dirty = true;
            self.desired_column = None;
        }
        ok
    }

    /// The number of undoable edits.
    pub fn undo_depth(&self) -> usize {
        self.history.depth()
    }

    // ---- clipboard ------------------------------------------------------

    /// Copies the selection to `clipboard`, as CRLF text.
    ///
    /// Windows puts CRLF on the clipboard whatever the file uses, and paste
    /// normalises back, so a copy/paste round trip inside the document is
    /// faithful either way.
    pub fn copy_to(&self, clipboard: &mut dyn Clipboard) -> bool {
        match self.selected_text() {
            Some(text) if !text.is_empty() => {
                clipboard.set_text(line_ending::from_lf(text, LineEnding::CrLf));
                true
            }
            _ => false,
        }
    }

    /// Copies the selection to `clipboard` and deletes it.
    pub fn cut_to(&mut self, clipboard: &mut dyn Clipboard) -> bool {
        if !self.copy_to(clipboard) {
            return false;
        }
        self.delete_selection()
    }

    /// Replaces the selection with the clipboard's text.
    ///
    /// One paste is one undo step, and any CRLF in the pasted text is
    /// normalised to the buffer's `\n`.
    pub fn paste_from(&mut self, clipboard: &dyn Clipboard) -> bool {
        match clipboard.get_text() {
            Some(text) if !text.is_empty() => self.insert_uncoalesced(&text),
            _ => false,
        }
    }

    // ---- layout and saving ----------------------------------------------

    /// Lays the document out for a viewport `columns` wide.
    pub fn wrapped_lines(&self, columns: usize) -> Vec<WrappedLine> {
        wrap_document(&self.buffer, columns, self.tab_stop)
    }

    /// The bytes to write to disk.
    ///
    /// An unedited document is handed back exactly as it was read in — the
    /// buffered original, not a reconstruction — so ordinary save can never
    /// change a byte (`docs/test-strategy.md` §5). Once the document has been
    /// edited it is written in the document's style.
    pub fn save_text(&self) -> String {
        if !self.dirty {
            if let Some(original) = &self.original {
                return original.clone();
            }
        }
        line_ending::from_lf(self.buffer.as_str(), self.line_ending)
    }

    /// Records that the document has been saved, and what was written.
    pub fn mark_saved(&mut self) {
        self.original = Some(self.save_text());
        self.dirty = false;
    }

    /// Forgets the history and the remembered original, as closing and
    /// reopening a document does.
    pub fn reset(&mut self) {
        self.history.clear();
        self.original = None;
        self.dirty = false;
        self.selection = Selection::caret(0);
        self.desired_column = None;
    }
}
