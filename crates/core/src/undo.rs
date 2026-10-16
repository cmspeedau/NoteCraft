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

/// One step of history: either a single edit or a set that was applied together.
///
/// A drag-and-drop move touches the buffer at two offsets — the text leaves its
/// old place and arrives at the new one — so it cannot be one [`Edit`]. Grouping
/// keeps it *one* undo step all the same, which is what the user pressed the
/// button on: undo takes the text back to where it was dragged from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    /// A single edit.
    Edit(Edit),
    /// Edits applied as one step; undone in reverse order, redone forwards.
    Group(Vec<Edit>),
}

impl Entry {
    /// The selection the entry restores when it is undone.
    fn selection_before(&self) -> Option<Selection> {
        match self {
            Self::Edit(edit) => Some(edit.selection_before),
            Self::Group(edits) => edits.first().map(|edit| edit.selection_before),
        }
    }

    /// The selection the entry restores when it is redone.
    fn selection_after(&self) -> Option<Selection> {
        match self {
            Self::Edit(edit) => Some(edit.selection_after),
            Self::Group(edits) => edits.last().map(|edit| edit.selection_after),
        }
    }
}

/// A linear undo history with a redo stack.
///
/// Two behaviours are *not* fixed by the reference yet, so this type makes them
/// explicit rather than guessing (`AGENT.md` §2):
///
/// * Here, consecutive typing merges into one history entry (as a run of typed
///   characters normally does) while a newline, a paste, and a deletion each
///   start their own entry. The exact grouping is a design decision — see
///   `docs/feature-matrix.md` NC-E001.
/// * The history is unbounded. A bounded undo depth is also a design decision;
///   [`UndoStack::truncate`] exists so a depth can be imposed once one is
///   chosen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UndoStack {
    done: Vec<Entry>,
    undone: Vec<Entry>,
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

    /// The number of undoable steps.
    pub fn depth(&self) -> usize {
        self.done.len()
    }

    /// The number of redoable steps.
    pub fn redo_depth(&self) -> usize {
        self.undone.len()
    }

    /// Keeps at most `depth` undoable steps, dropping the oldest.
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
            if let Some(Entry::Edit(last)) = self.done.last_mut() {
                if Self::mergeable(last, &edit) {
                    last.inserted.push_str(&edit.inserted);
                    last.selection_after = edit.selection_after;
                    return;
                }
            }
        }
        self.done.push(Entry::Edit(edit));
    }

    /// Records several edits as one undoable step.
    ///
    /// The edits must be given in the order they were applied to the buffer, so
    /// that undo can reverse them and redo can replay them. An empty set
    /// records nothing — a step that changes no text would be a button press
    /// the user cannot see.
    pub fn record_group(&mut self, edits: Vec<Edit>) {
        if edits.is_empty() {
            return;
        }
        self.undone.clear();
        self.done.push(Entry::Group(edits));
    }

    fn mergeable(last: &Edit, next: &Edit) -> bool {
        last.removed.is_empty()
            && next.removed.is_empty()
            && !last.inserted.contains('\n')
            && !next.inserted.contains('\n')
            && last.start + last.inserted.len() == next.start
    }

    /// Reverses the most recent step, restoring its start selection.
    pub fn undo(&mut self, buffer: &mut TextBuffer, selection: &mut Selection) -> bool {
        let Some(entry) = self.done.pop() else {
            return false;
        };
        match &entry {
            Entry::Edit(edit) => {
                buffer.replace(edit.inserted_range(), &edit.removed);
            }
            Entry::Group(edits) => {
                for edit in edits.iter().rev() {
                    buffer.replace(edit.inserted_range(), &edit.removed);
                }
            }
        }
        if let Some(before) = entry.selection_before() {
            *selection = before;
        }
        self.undone.push(entry);
        true
    }

    /// Reapplies the most recently undone step.
    pub fn redo(&mut self, buffer: &mut TextBuffer, selection: &mut Selection) -> bool {
        let Some(entry) = self.undone.pop() else {
            return false;
        };
        match &entry {
            Entry::Edit(edit) => {
                buffer.replace(edit.removed_range(), &edit.inserted);
            }
            Entry::Group(edits) => {
                for edit in edits.iter() {
                    buffer.replace(edit.removed_range(), &edit.inserted);
                }
            }
        }
        if let Some(after) = entry.selection_after() {
            *selection = after;
        }
        self.done.push(entry);
        true
    }
}
