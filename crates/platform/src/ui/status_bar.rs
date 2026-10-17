//! The status bar: caret position, character count, document kind, zoom,
//! line-ending style, and encoding.

use egui::{Align, Align2, FontId, Layout, RichText};

use crate::ui::state::Shell;
use crate::ui::theme::Palette;

/// The height of the status bar, in points.
pub const HEIGHT: f32 = 28.0;

/// Draws the status bar.
pub fn show(shell: &Shell, ui: &mut egui::Ui) {
    egui::Panel::bottom("status_bar")
        .exact_size(HEIGHT)
        .frame(egui::Frame::NONE.fill(shell.palette.chrome))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            let tab = shell.active_tab();

            ui.horizontal_centered(|ui| {
                ui.add_space(6.0);
                let palette = shell.palette;
                ui.label(item(
                    format!("Ln {}, Col {}", shell.line, shell.column),
                    palette,
                ));
                ui.separator();
                ui.label(item(format!("{} characters", shell.char_count()), palette));
                if let Some(note) = &shell.notification {
                    ui.separator();
                    ui.label(RichText::new(note).size(12.0).color(shell.palette.text));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(6.0);
                    ui.label(item(
                        tab.map(|t| t.encoding_label()).unwrap_or("UTF-8"),
                        palette,
                    ));
                    ui.separator();
                    ui.label(item(
                        tab.map(|t| t.line_ending_label())
                            .unwrap_or("Windows (CRLF)"),
                        palette,
                    ));
                    ui.separator();
                    ui.label(item(format!("{}%", shell.zoom), palette));
                });
            });

            let label = tab.map(|t| t.kind_label()).unwrap_or("Plain text");
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(12.0),
                shell.palette.text_dim,
            );
        });
}

/// A status-bar text item.
fn item(text: impl Into<String>, palette: Palette) -> RichText {
    RichText::new(text.into())
        .size(12.0)
        .color(palette.text_dim)
}
