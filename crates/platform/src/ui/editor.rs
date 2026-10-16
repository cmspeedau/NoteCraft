//! The editor surface: a monospaced, zoomable text area.

use egui::{FontId, Margin};

use crate::ui::state::Shell;
use crate::ui::theme;

/// Draws the editor for the active tab and updates the caret position.
pub fn show(shell: &mut Shell, ui: &mut egui::Ui) {
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

            let text = &mut shell.tabs[index].text;
            let output = egui::TextEdit::multiline(text)
                .font(FontId::monospace(15.0 * zoom))
                .desired_width(width)
                .frame(egui::Frame::NONE)
                .show(ui);

            if let Some(range) = output.cursor_range {
                let (line, column) = line_column(text, usize::from(range.primary.index));
                shell.line = line;
                shell.column = column;
            }
        });
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

    #[test]
    fn caret_position_is_one_based() {
        assert_eq!(line_column("", 0), (1, 1));
        assert_eq!(line_column("abc", 0), (1, 1));
        assert_eq!(line_column("abc", 2), (1, 3));
        assert_eq!(line_column("ab\ncd", 3), (2, 1));
        assert_eq!(line_column("ab\ncd", 5), (2, 3));
    }
}
