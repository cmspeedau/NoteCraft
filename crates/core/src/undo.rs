//! The undo and redo history.

use crate::buffer::TextBuffer;
use crate::selection::Selection;

/// One reversible edit.
///
/// An edit is stored as "at `start`, `removed` was replaced by `inserted`",
/// together with the selection on either side so undo can restore the caret
/// rather than dumping it at the top of the document. Storing the removed text
/// is what makes the history exact: no edit has to be recomputed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Byte offset the edit started at.
    pub start: usize,
    /// The text that was replaced.
    pub removed: String,
    /// The text that replaced it.
    pub inserted: String,
    /// The selection before the edit.
    pub selection_before: Selection,
    /// The selection after the edit.
    pub selection_after: Selection,
}

impl Edit {
    /// The range this edit currently occupies in the buffer.
    pub fn inserted_range(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.inserted.len()
    }

    /// The range the inverse edit will occupy.
    pub fn removed_range(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.removed.len()
    }
}

/// A linear undo history with a redo stack.
///
/// Two reference behaviours are *not* captured yet, so this type makes them
/// explicit rather than guessing (`AGENT.md` §2):
///
/// * Here, consecutive typing merges into one history entry (as the reference
///   does for a run of typed characters) while a newline, a paste, and a
///   deletion each start their own entry. The reference's exact grouping is a
///   capture item — see `docs/feature-matrix.md` NC-E001.
/// * The history is unbounded. The reference's undo depth is also a capture
///   item; [`UndoStack::truncate`] exists so a depth can be imposed once it is
///   pinned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UndoStack {
    done: Vec<Edit>,
    undone: Vec<Edit>,
}

impl UndoStack {
    /// An empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Discards both stacks.
    pub fn clear(&mut self) {
        self.done.clear();
        self.undone.clear();
    }

    /// Whether there is anything to undo.
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    /// Whether there is anything to redo.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// The number of undoable edits.
    pub fn depth(&self) -> usize {
        self.done.len()
    }

    /// The number of redoable edits.
    pub fn redo_depth(&self) -> usize {
        self.undone.len()
    }

    /// Keeps at most `depth` undoable edits, dropping the oldest.
    pub fn truncate(&mut self, depth: usize) {
        if self.done.len() > depth {
            self.done.drain(..self.done.len() - depth);
        }
    }

    /// Records `edit`.
    ///
    /// When `coalesce_typing` is set and the previous entry is a plain typing
    /// run ending exactly where this one starts, the two merge.
    pub fn record(&mut self, edit: Edit, coalesce_typing: bool) {
        self.undone.clear();
        if coalesce_typing {
            if let Some(last) = self.done.last_mut() {
                if Self::mergeable(last, &edit) {
                    last.inserted.push_str(&edit.inserted);
                    last.selection_after = edit.selection_after;
                    return;
                }
            }
        }
        self.done.push(edit);
    }

    fn mergeable(last: &Edit, next: &Edit) -> bool {
        last.removed.is_empty()
            && next.removed.is_empty()
            && !last.inserted.contains('\n')
            && !next.inserted.contains('\n')
            && last.start + last.inserted.len() == next.start
    }

    /// Reverses the most recent edit, restoring its start selection.
    pub fn undo(&mut self, buffer: &mut TextBuffer, selection: &mut Selection) -> bool {
        let Some(edit) = self.done.pop() else {
            return false;
        };
        buffer.replace(edit.inserted_range(), &edit.removed);
        *selection = edit.selection_before;
        self.undone.push(edit);
        true
    }

    /// Reapplies the most recently undone edit.
    pub fn redo(&mut self, buffer: &mut TextBuffer, selection: &mut Selection) -> bool {
        let Some(edit) = self.undone.pop() else {
            return false;
        };
        buffer.replace(edit.removed_range(), &edit.inserted);
        *selection = edit.selection_after;
        self.done.push(edit);
        true
    }
}
