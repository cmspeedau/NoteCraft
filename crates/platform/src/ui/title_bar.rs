//! The title bar: app icon, tab strip, new-tab button, and window controls.

use egui::{Align, Align2, Context, CornerRadius, FontId, Layout, Sense, Stroke, ViewportCommand};

use crate::ui::state::{DocKind, Shell, Tab};
use crate::ui::{actions, theme, widgets};

/// The height of the title bar, in points.
pub const HEIGHT: f32 = 40.0;

/// What a click on a tab did.
#[derive(Default)]
struct TabHit {
    /// The body of the tab was clicked.
    activate: bool,
    /// The tab's close button was clicked.
    close: bool,
}

/// Draws the title bar and applies any tab changes it caused.
pub fn show(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    egui::Panel::top("title_bar")
        .exact_size(HEIGHT)
        .frame(egui::Frame::NONE.fill(theme::CHROME))
        .show(ui, |ui| {
            // Registered before the controls so the empty parts of the bar drag
            // the window while the controls still receive their clicks.
            let bar = ui.max_rect();
            let drag = ui.interact(bar, ui.id().with("title_drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }

            let mut to_activate: Option<usize> = None;
            let mut to_close: Option<usize> = None;

            ui.horizontal_centered(|ui| {
                app_icon(ui);
                let _ = widgets::icon_button(ui, "‹");
                ui.add_space(2.0);

                egui::ScrollArea::horizontal()
                    .id_salt("tabs")
                    .show(ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            for (index, tab) in shell.tabs.iter().enumerate() {
                                let hit = tab_widget(ui, index, tab, index == shell.active);
                                if hit.close {
                                    to_close = Some(index);
                                } else if hit.activate {
                                    to_activate = Some(index);
                                }
                            }
                        });
                    });

                let new_tab = widgets::icon_button(ui, "+");
                if new_tab.clicked() {
                    shell.open_tab(DocKind::Plain);
                }
                new_tab.on_hover_text("New tab");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    window_controls(ui, ctx, shell);
                });
            });

            if let Some(index) = to_activate {
                shell.active = index;
            }
            if let Some(index) = to_close {
                actions::close_tab(shell, index);
            }
        });
}

/// The three window buttons, right to left.
fn window_controls(ui: &mut egui::Ui, ctx: &Context, shell: &mut Shell) {
    if widgets::icon_button(ui, "✕").clicked() {
        // Closing the window prompts for unsaved documents (`NC-D005`).
        actions::close_window(shell);
    }
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let maximize_glyph = if maximized { "❐" } else { "▢" };
    if widgets::icon_button(ui, maximize_glyph).clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    if widgets::icon_button(ui, "—").clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

/// A small blue app icon.
fn app_icon(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(5), theme::ACCENT);
    let stroke = Stroke::new(1.4, egui::Color32::WHITE);
    for i in 0..3 {
        let y = rect.top() + 6.0 + i as f32 * 4.0;
        painter.line_segment(
            [
                egui::pos2(rect.left() + 5.0, y),
                egui::pos2(rect.right() - 5.0, y),
            ],
            stroke,
        );
    }
}

/// A single tab, returning what was clicked.
fn tab_widget(ui: &mut egui::Ui, index: usize, tab: &Tab, active: bool) -> TabHit {
    let height = 34.0;
    let title = tab.title();
    let width = (title.chars().count() as f32 * 7.5 + 52.0).clamp(120.0, 220.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());

    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        egui::vec2(22.0, 22.0),
    );
    let close = ui.interact(
        close_rect,
        ui.id().with(("tab_close", index)),
        Sense::click(),
    );

    let painter = ui.painter();
    let fill = if active {
        theme::BG
    } else if response.hovered() {
        theme::ROW_HOVER
    } else {
        theme::CHROME
    };
    painter.rect_filled(
        rect,
        CornerRadius {
            nw: 6,
            ne: 6,
            sw: 0,
            se: 0,
        },
        fill,
    );
    if active {
        let underline = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 6.0, rect.bottom() - 2.0),
            egui::pos2(rect.right() - 6.0, rect.bottom()),
        );
        painter.rect_filled(underline, CornerRadius::same(1), theme::ACCENT);
    }

    let text_color = if active { theme::TEXT } else { theme::TEXT_DIM };
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        &title,
        FontId::proportional(13.0),
        text_color,
    );

    // An unsaved change shows as a dot where the close button sits, the way a
    // tab strip usually reports it (`docs/ui-reference.md` §1).
    if tab.is_dirty() {
        painter.circle_filled(
            egui::pos2(close_rect.center().x, close_rect.center().y),
            3.0,
            theme::ACCENT,
        );
    }

    if active || response.hovered() {
        if close.hovered() {
            painter.rect_filled(close_rect, CornerRadius::same(4), theme::HOVER);
        }
        painter.text(
            close_rect.center(),
            Align2::CENTER_CENTER,
            "✕",
            FontId::proportional(11.0),
            theme::TEXT_DIM,
        );
    }

    TabHit {
        activate: response.clicked() && !close.hovered(),
        close: close.clicked(),
    }
}
