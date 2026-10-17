//! The editor surface: a monospaced, zoomable text area over the document.
//!
//! The text edit lives inside a scroll area enabled on **both** axes, and that
//! enclosure is doing more than drawing bars. A `TextEdit` whose caret moves inside it
//! — a click, an arrow key, typing — asks `egui` to keep that caret in view by calling
//! `Ui::scroll_to_rect`; with no scroll area enclosing it, as here once, the request
//! went nowhere and a caret moved past the bottom of the window was simply painted off
//! screen, unreachable. A document taller than the window now scrolls, and so does a
//! line too wide for it when [`Shell::word_wrap`] is off (see [`widest_line_width`]).
//!
//! What the widget cannot see for itself is a caret moved *from outside* it. Find, Go to
//! line, Select all and the rest queue a selection which is written into the widget's
//! saved state between frames, so within its own frame the cursor never changed and it
//! does not scroll; [`scroll_caret_into_view`] is the shell making the same request on
//! those commands' behalf.
//!
//! [`Shell::word_wrap`]: crate::ui::state::Shell::word_wrap
//!
//! The text is not a `String` the shell owns: it is the `nc-core` [`Document`]
//! inside the tab's [`TextFile`](nc_textproc::TextFile), edited through a thin
//! [`egui::TextBuffer`] adapter. That keeps one authoritative copy of the text
//! — the one the file saves and the searches read.

use std::ops::Range;

use egui::text::{CCursor, CCursorRange, CharIndex};
use egui::{FontId, Margin};
use nc_core::Document;

use crate::ui::markdown_view;
use crate::ui::state::{Shell, Tab};

/// The editor widget's id. Fixed so a queued selection can be stored under it
/// and tests can read the caret back.
pub const EDITOR_ID: &str = "notecraft_editor";

/// The editor's font size in points at 100% zoom. The formatted view scales from
/// the same number, so a normal line measures the same in either view.
const EDIT_FONT_SIZE: f32 = 15.0;

/// How much room to leave past the end of the longest line.
///
/// The text edit keeps four points of text margin inside its own rect, and the caret
/// sits *after* the last glyph rather than over it, so a rect exactly as wide as the
/// text would break the line it was sized for.
const LINE_HEADROOM: f32 = 16.0;

/// The width `text`'s longest line wants, in points, at `font`.
///
/// Laid out with [`egui::text::LayoutJob::simple`], which is what the text edit's own
/// layouter uses, so the measurement cannot disagree with the layout it is sizing the
/// widget for. Only the longest line is laid out: the rest of the document cannot be
/// wider than it, and a document of any size would otherwise be measured twice per
/// frame in the unwrapped case.
fn widest_line_width(text: &str, font: &FontId, ui: &egui::Ui) -> f32 {
    let longest = text
        .split('\n')
        .max_by_key(|line| line.chars().count())
        .unwrap_or("");
    let job = egui::text::LayoutJob::simple(
        longest.to_owned(),
        font.clone(),
        ui.visuals().strong_text_color(),
        f32::INFINITY,
    );
    ui.fonts_mut(|fonts| fonts.layout_job(job)).rect.width() + LINE_HEADROOM
}

/// Draws the editor for the active tab, updates the caret position, and applies
/// any selection the shell has queued (Find / Go to).
pub fn show(shell: &mut Shell, ui: &mut egui::Ui) {
    // Taken before the document is borrowed: a search or Go to sets this and
    // the editor applies it this frame.
    let pending = shell.pending_selection.take();

    egui::CentralPanel::no_frame()
        .frame(
            egui::Frame::NONE
                .fill(shell.palette.bg)
                .inner_margin(Margin::symmetric(12, 10)),
        )
        .show(ui, |ui| {
            let index = shell.active;
            if index >= shell.tabs.len() {
                return;
            }
            let palette = shell.palette;
            let zoom = shell.zoom as f32 / 100.0;

            // The formatted view is derived from the source and read-only, so it
            // claims no caret: the document keeps the selection it already holds for
            // the whole time it is shown, which is why the switch cannot lose one.
            // All the view has to keep honest is the status bar's line and column.
            let caret = if shell.formatted_view() {
                let tab = &shell.tabs[index];
                markdown_view::show(tab, ui, EDIT_FONT_SIZE * zoom, palette);
                char_index(tab.text(), tab.document().caret())
            } else {
                source_view(shell, index, ui, pending)
            };

            shell.caret = caret;
            let (line, column) = line_column(shell.tabs[index].text(), caret);
            shell.line = line;
            shell.column = column;
        });
}

/// Draws the source view and returns the caret the editor ends the frame on.
fn source_view(
    shell: &mut Shell,
    index: usize,
    ui: &mut egui::Ui,
    pending: Option<(usize, usize)>,
) -> usize {
    let wrap = shell.word_wrap;
    let zoom = shell.zoom as f32 / 100.0;
    let font = FontId::monospace(EDIT_FONT_SIZE * zoom);
    // Measured before the document is borrowed for the widget below, and only when it
    // is needed: the longest line alone, not a second layout of the whole document.
    let natural = if wrap {
        0.0
    } else {
        widest_line_width(shell.tabs[index].text(), &font, ui)
    };

    // One area, both axes. Vertical, because a document is usually taller than the
    // window and the rows below the fold have to be reachable — and because the text
    // edit's own keep-the-caret-in-view request needs an area to hear it. Horizontal,
    // because with wrapping off a long line is wider than any window and is the text
    // edit's real content width, not an overflow to be clipped away.
    //
    // `auto_shrink` off so the area fills the panel: shrinking to the content would
    // leave the space under a short document outside the widget, so a click there would
    // reach nothing.
    egui::ScrollArea::both()
        .id_salt("editor_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let tab = &mut shell.tabs[index];
            let output = {
                let mut buffer = DocBuffer(tab.document_mut());
                let edit = egui::TextEdit::multiline(&mut buffer)
                    .id(egui::Id::new(EDITOR_ID))
                    .font(font.clone())
                    .frame(egui::Frame::NONE);
                if wrap {
                    edit.desired_width(ui.available_width()).show(ui)
                } else {
                    // Unwrapped, the editor has to be as wide as its widest line, and
                    // `egui` will not work that out for you: a `TextEdit` clamps its
                    // desired width to the space it is given before laying text out, so
                    // `desired_width(f32::INFINITY)` measures the document at the
                    // panel's width and wraps it anyway — and a scroll area does not
                    // rescue it either, because 0.36 tells the area's inner UI to prefer
                    // wrapping to a horizontal bar, on purpose. The measured width is
                    // therefore handed to the widget as `min_size`, the one input that
                    // outranks the clamp, and the area it sits in turns that width into
                    // a bar the user can drag.
                    edit.min_size(egui::vec2(natural, 0.0))
                        .desired_width(natural)
                        .show(ui)
                }
            };

            // The caret a queued selection moves to, or the one the user left
            // the editor with.
            match pending {
                Some((primary, secondary)) => {
                    apply_selection(ui.ctx(), &output, primary, secondary);
                    set_document_selection(tab, primary, secondary);
                    scroll_caret_into_view(ui, &output, primary);
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
            }
        })
        .inner
}

/// Scrolls enough of the frame to bring a caret the shell just moved to back on
/// screen.
///
/// The rectangle is the one the text edit would have used for itself: `egui` positions
/// a caret with [`egui::Galley::pos_from_cursor`], in the galley's own coordinates, and
/// paints that galley shifted left by the galley's left edge — so the translation here
/// matches its `cursor_rect` arithmetic rather than approximating it.
fn scroll_caret_into_view(ui: &egui::Ui, output: &egui::text_edit::TextEditOutput, primary: usize) {
    let rect = output
        .galley
        .pos_from_cursor(CCursor::new(primary))
        .translate(output.galley_pos.to_vec2() - egui::vec2(output.galley.rect.left(), 0.0));
    ui.scroll_to_rect(rect, None);
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

/// The character index of a byte offset, clamped to the text.
///
/// `egui` addresses text by character index and the document by byte offset, so a
/// selection has to travel back across the same boundary it came from — the rule
/// `AGENT.md` §4 sets for the editor.
pub(crate) fn char_index(text: &str, byte: usize) -> usize {
    let byte = byte.min(text.len());
    text[..byte].chars().count()
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
