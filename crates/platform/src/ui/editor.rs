//! The editor surface: a monospaced, zoomable text area over the document.
//!
//! The text is not a `String` the shell owns: it is the `nc-core` [`Document`]
//! inside the tab's [`TextFile`](nc_textproc::TextFile), edited through a thin
//! [`egui::TextBuffer`] adapter. That keeps one authoritative copy of the text
//! — the one the file saves and the searches read.

use std::ops::Range;

use egui::text::{CCursor, CCursorRange, CharIndex};
use egui::{FontId, Margin};
use nc_core::Document;

use crate::ui::state::{Shell, Tab};
use crate::ui::theme;

/// The editor widget's id. Fixed so a queued selection can be stored under it
/// and tests can read the caret back.
pub const EDITOR_ID: &str = "notecraft_editor";

/// Draws the editor for the active tab, updates the caret position, and applies
/// any selection the shell has queued (Find / Go to).
pub fn show(shell: &mut Shell, ui: &mut egui::Ui) {
    // Taken before the document is borrowed: a search or Go to sets this and
    // the editor applies it this frame.
    let pending = shell.pending_selection.take();

    egui::CentralPanel::no_frame()
        .frame(
            egui::Frame::NONE
                .fill(theme::BG)
                .inner_margin(Margin::symmetric(12, 10)),
        )
        .show(ui, |ui| {
            let index = shell.active;
            if index >= shell.tabs.len() {
                return;
            }
            let wrap = shell.word_wrap;
            let zoom = shell.zoom as f32 / 100.0;
            let width = if wrap {
                ui.available_width()
            } else {
                f32::INFINITY
            };

            let tab = &mut shell.tabs[index];
            let output = {
                let mut buffer = DocBuffer(tab.document_mut());
                egui::TextEdit::multiline(&mut buffer)
                    .id(egui::Id::new(EDITOR_ID))
                    .font(FontId::monospace(15.0 * zoom))
                    .desired_width(width)
                    .frame(egui::Frame::NONE)
                    .show(ui)
            };

            // The caret a queued selection moves to, or the one the user left
            // the editor with.
            let caret = match pending {
                Some((primary, secondary)) => {
                    apply_selection(ui.ctx(), &output, primary, secondary);
                    set_document_selection(tab, primary, secondary);
                    primary
                }
                None => match output.cursor_range {
                    Some(range) => {
                        let primary = usize::from(range.primary.index);
                        let secondary = usize::from(range.secondary.index);
                        set_document_selection(tab, primary, secondary);
                        primary
                    }
                    None => shell.caret,
                },
            };

            shell.caret = caret;
            let (line, column) = line_column(tab.text(), caret);
            shell.line = line;
            shell.column = column;
        });
}

/// Moves the editor's caret and selection to the given character indices.
///
/// `egui` keeps the selection in per-widget state, so this stores the new
/// cursor back under the widget's own id and focuses the editor.
fn apply_selection(
    ctx: &egui::Context,
    output: &egui::text_edit::TextEditOutput,
    primary: usize,
    secondary: usize,
) {
    let mut state = output.state.clone();
    state.cursor.set_char_range(Some(CCursorRange {
        primary: CCursor::new(primary),
        secondary: CCursor::new(secondary),
        h_pos: None,
    }));
    state.store(ctx, output.response.id);
    output.response.request_focus();
}

/// Points the document's own selection at the range the editor widget holds.
///
/// `egui` keeps the caret and selection in per-widget state; commands routed
/// through the shell (Delete, Select all, Cut) act on the document's selection,
/// so the two are kept in step every frame — the document is the single place
/// those commands read.
fn set_document_selection(tab: &mut Tab, primary: usize, secondary: usize) {
    let (anchor, head) = {
        let text = tab.text();
        (byte_index(text, secondary), byte_index(text, primary))
    };
    tab.document_mut().select(anchor, head);
}

/// An [`egui::TextBuffer`] over an `nc-core` document.
///
/// `egui` addresses text by character index; the document by byte offset, so
/// every call converts at the boundary — the rule `AGENT.md` §4 sets for the
/// editor.
struct DocBuffer<'a>(&'a mut Document);

impl egui::TextBuffer for DocBuffer<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.0.text()
    }

    fn insert_text(&mut self, text: &str, char_index: CharIndex) -> usize {
        if text.is_empty() {
            return 0;
        }
        let byte = byte_index(self.0.text(), usize::from(char_index));
        self.0.set_caret(byte);
        self.0.insert_str(text);
        text.chars().count()
    }

    fn delete_char_range(&mut self, range: Range<CharIndex>) {
        let start = usize::from(range.start);
        let end = usize::from(range.end);
        if start >= end {
            return;
        }
        let text = self.0.text();
        let start_byte = byte_index(text, start);
        let end_byte = byte_index(text, end);
        self.0.replace_range(start_byte..end_byte, "");
    }

    fn type_id(&self) -> core::any::TypeId {
        core::any::TypeId::of::<DocBuffer<'_>>()
    }
}

/// The byte offset of a character index, clamped to the text.
fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len())
}

/// Converts a character index into a 1-based (line, column).
fn line_column(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for (index, ch) in text.chars().enumerate() {
        if index >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::TextBuffer;

    #[test]
    fn caret_position_is_one_based() {
        assert_eq!(line_column("", 0), (1, 1));
        assert_eq!(line_column("abc", 0), (1, 1));
        assert_eq!(line_column("abc", 2), (1, 3));
        assert_eq!(line_column("ab\ncd", 3), (2, 1));
        assert_eq!(line_column("ab\ncd", 5), (2, 3));
    }

    #[test]
    fn the_buffer_edits_the_document_at_character_indices() {
        let mut document = Document::from_text("héllo wörld");
        {
            let mut buffer = DocBuffer(&mut document);
            // Insert after the first 'é' (character index 2).
            buffer.insert_text("!", CharIndex(2));
            assert_eq!(buffer.as_str(), "hé!llo wörld");
            // Delete "!" (character range 2..3).
            buffer.delete_char_range(CharIndex(2)..CharIndex(3));
            assert_eq!(buffer.as_str(), "héllo wörld");
        }
        assert_eq!(document.text(), "héllo wörld");
    }

    #[test]
    fn byte_and_char_indices_agree_across_multibyte_text() {
        let text = "aé€b";
        assert_eq!(byte_index(text, 0), 0);
        assert_eq!(byte_index(text, 1), 1);
        assert_eq!(byte_index(text, 2), 3);
        assert_eq!(byte_index(text, 3), 6);
        assert_eq!(byte_index(text, 4), 7);
        assert_eq!(byte_index(text, 99), 7);
    }
}
