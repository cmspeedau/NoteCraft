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
use egui::{Color32, Event, FontId, Galley, Key, Margin, Shape, WidgetType};
use nc_core::Document;

use crate::ui::a11y;
use crate::ui::markdown_view;
use crate::ui::state::{Shell, Tab};

/// The editor widget's id. Fixed so a queued selection can be stored under it
/// and tests can read the caret back.
pub const EDITOR_ID: &str = "notecraft_editor";

/// The colour a misspelled word is underlined in (`NC-P004`). Chosen distinct
/// from the accent so a squiggle is never read as the active-tab or selection
/// colour; the appearance suite looks for exactly this ink.
pub(crate) const SPELL_INK: Color32 = Color32::from_rgb(200, 60, 60);

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
                // A rendering of the text is not a text field, so the document is
                // exposed to a screen reader here instead: without this the body
                // of the window reads as empty (`NC-P007`).
                a11y::document(
                    ui.ctx(),
                    ui.id().with(("formatted_document", index)),
                    ui.max_rect(),
                    tab.title(),
                    tab.text(),
                );
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
    let spell_check = shell.spell_check;
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
    //
    // The height the area is about to fill is how far the eye moves on a Page key; read
    // it now, before the area claims the space, so paging knows how many rows a screenful
    // is here ([`page_navigation`]).
    let viewport = ui.available_height();
    egui::ScrollArea::both()
        .id_salt("editor_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let title = shell.tabs[index].title();
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

            // The document the editor is editing, so a screen reader says which
            // one it has arrived in rather than only that there is a text field
            // (`NC-P007`). The text itself `egui` reports as the field's value.
            a11y::label(&output.response, WidgetType::TextEdit, &title);

            // The caret a queued selection moves to, or the one the user left
            // the editor with.
            let caret = match pending {
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
            };

            // Page Up / Page Down: the widget moves the caret on the arrow keys but
            // does nothing on the page keys, so the shell steps it a screenful here.
            let caret = page_navigation(ui, &output, tab, caret, viewport);

            // The other case that puts the caret outside the window without asking: the
            // document changed size underneath it. A new face or a new zoom reflows every
            // line, so the row the caret is on moves — and the caret itself did not change,
            // so neither this widget nor any command has reason to follow it. When the
            // metrics moved this frame, follow it anyway.
            //
            // The size is not enough to compare, and neither is the `FontId`: both ask for
            // `FontFamily::Monospace` whichever face is behind that name, so the thing that
            // moved is what the context resolves that family to, read back from the context
            // rather than from what the widget was handed.
            let metrics = (resolved_face(ui.ctx()), font.size);
            let key = egui::Id::new("editor-laid-out-in");
            let moved = ui
                .ctx()
                .data(|data| data.get_temp::<(String, f32)>(key))
                .is_none_or(|(face, size)| face != metrics.0 || size != metrics.1);
            if moved {
                scroll_caret_into_view(ui, &output, caret);
            }
            ui.ctx().data_mut(|data| data.insert_temp(key, metrics));

            // The squiggle under a likely misspelling (`NC-P004`). It is painted
            // from the word ranges the tab cached and the galley the text edit has
            // just laid out, so it follows wrapping and scrolling for free — the
            // same coordinate arithmetic the caret uses above.
            if spell_check {
                tab.refresh_spellings();
                paint_misspellings(ui, &output, tab);
            }
            caret
        })
        .inner
}

/// Pages the caret up or down a screenful, for the two keys `egui` leaves dead.
///
/// A `TextEdit` moves the caret on the arrows — and, in this version, on nothing
/// else: `Page Up` and `Page Down` reach it and are ignored, so in a document taller
/// than the window they did nothing at all. The shell answers them by stepping the
/// caret as many rows as fit in the window the document is laid out in, one row at a
/// time with the very primitive the arrows use ([`egui::Galley::cursor_down_one_row`]),
/// so the column the caret sat in is kept and the ends clamp the way an arrow clamps —
/// repeated pages stop at the top and bottom instead of overshooting. The caret is then
/// brought back on screen with the same request every other outside-the-widget move
/// makes. Holding Shift moves the caret and leaves the anchor where it was, extending the
/// selection exactly as Shift+arrow does; a Ctrl or Alt chord is left in the queue for
/// anything that wants it. Only the page movement is the shell's — word and line
/// navigation stay the widget's own (`NC-E004`).
fn page_navigation(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    tab: &mut Tab,
    caret: usize,
    viewport: f32,
) -> usize {
    // The keys belong to the editor and to nothing else: a focused dialog keeps its
    // own Page keys.
    if !output.response.has_focus() {
        return caret;
    }

    // Which page key the frame carries, as `(down, extend)` — at most one, and only a
    // bare or Shift one. Consumed here so it is not seen again further down the frame.
    let mut page: Option<(bool, bool)> = None;
    ui.ctx().input_mut(|input| {
        input.events.retain(|event| {
            let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return true;
            };
            if !matches!(key, Key::PageDown | Key::PageUp) {
                return true;
            }
            if modifiers.ctrl || modifiers.command || modifiers.alt || page.is_some() {
                return true;
            }
            page = Some((matches!(key, Key::PageDown), modifiers.shift));
            false
        });
    });
    let Some((down, extend)) = page else {
        return caret;
    };

    // A page is the window's height in rows. The rows are uniform, so the first one's
    // height stands for all; without a laid-out document (heights not yet known, or a
    // single row that cannot be paged through) there is nowhere to go.
    let galley = &output.galley;
    let Some(row_height) = galley.rows.first().map(|row| row.height()) else {
        return caret;
    };
    if row_height <= 0.0 || galley.rows.len() < 2 {
        return caret;
    }
    let page_rows = ((viewport / row_height).floor() as usize).max(1);

    // Walk the page one row at a time, handing the horizontal position from each step to
    // the next so the caret keeps its column across the whole jump.
    let mut cursor = CCursor::new(caret);
    let mut column: Option<f32> = None;
    for _ in 0..page_rows {
        let (next, kept) = if down {
            galley.cursor_down_one_row(&cursor, column)
        } else {
            galley.cursor_up_one_row(&cursor, column)
        };
        cursor = next;
        column = kept;
    }
    let primary = usize::from(cursor.index);
    if primary == caret {
        return caret;
    }

    // Shift keeps the far end of the selection as the anchor; otherwise the caret
    // collapses onto where it landed. Then the frame's own bring-it-into-view request.
    let secondary = if extend {
        output
            .cursor_range
            .map(|range| usize::from(range.secondary.index))
            .unwrap_or(caret)
    } else {
        primary
    };
    apply_selection(ui.ctx(), output, primary, secondary);
    set_document_selection(tab, primary, secondary);
    scroll_caret_into_view(ui, output, primary);
    primary
}

/// The face the context resolves the document's family slot to right now, which is the
/// face the text edit has just laid the document out in.
fn resolved_face(ctx: &egui::Context) -> String {
    ctx.fonts(|fonts| {
        fonts
            .definitions()
            .families
            .get(&egui::FontFamily::Monospace)
            .and_then(|names| names.first().cloned())
            .unwrap_or_default()
    })
}

/// Paints a red squiggle under every misspelled word the tab cached
/// (`NC-P004`). Each byte range is turned into the character indices the galley
/// speaks and mapped to screen with the same translation the caret uses, so the
/// underline sits where the word does whether or not it wrapped to a second row.
fn paint_misspellings(ui: &egui::Ui, output: &egui::text_edit::TextEditOutput, tab: &Tab) {
    if tab.spellings.is_empty() {
        return;
    }
    let text = tab.text();
    let shift = output.galley_pos.to_vec2() - egui::vec2(output.galley.rect.left(), 0.0);
    let painter = ui.painter();
    for range in &tab.spellings {
        let from = char_index(text, range.start);
        let to = char_index(text, range.end);
        for band in word_bands(&output.galley, from, to) {
            draw_squiggle(painter, band.translate(shift));
        }
    }
}

/// The rectangle to underline for the characters `from..to` of the galley, split
/// per row so a word that wrapped is underlined on each row it occupies. The
/// rectangles are in the galley's own coordinates.
fn word_bands(galley: &Galley, from: usize, to: usize) -> Vec<egui::Rect> {
    let start = galley.layout_from_cursor(CCursor::new(from));
    let end = galley.layout_from_cursor(CCursor::new(to));
    let mut bands = Vec::new();
    for row in start.row..=end.row {
        let Some(placed) = galley.rows.get(row) else {
            break;
        };
        let full = placed.char_count_excluding_newline();
        let first = if row == start.row {
            start.column
        } else {
            CharIndex::ZERO
        };
        let last = if row == end.row { end.column } else { full };
        if !(first < last) {
            continue;
        }
        let x0 = placed.pos.x + placed.x_offset(first);
        let x1 = placed.pos.x + placed.x_offset(last);
        if x1 <= x0 {
            continue;
        }
        // One point above the row's bottom, so the wave sits under the glyphs.
        let y = placed.max_y() - 1.0;
        bands.push(egui::Rect::from_min_max(
            egui::pos2(x0, y),
            egui::pos2(x1, y),
        ));
    }
    bands
}

/// The little wave drawn under one row's span of a misspelled word.
fn draw_squiggle(painter: &egui::Painter, band: egui::Rect) {
    const AMPLITUDE: f32 = 1.2;
    const WAVELENGTH: f32 = 5.0;
    let step = WAVELENGTH / 4.0;
    let x0 = band.min.x;
    let y0 = band.min.y;
    let mut points = vec![egui::pos2(x0, y0)];
    let mut x = x0;
    while x < band.max.x {
        x = (x + step).min(band.max.x);
        let t = (x - x0) / WAVELENGTH * std::f32::consts::TAU;
        points.push(egui::pos2(x, y0 + AMPLITUDE * t.sin()));
    }
    painter.add(Shape::line(points, egui::Stroke::new(1.0, SPELL_INK)));
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
