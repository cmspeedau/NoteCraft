//! The title bar: app icon, tab strip, new-tab button, and window controls.

use egui::accesskit::Role;
use egui::{
    Align, Align2, Context, CornerRadius, FontId, Layout, Sense, Stroke, ViewportCommand,
    WidgetType,
};

use crate::ui::state::{DocKind, Shell, Tab};
use crate::ui::theme::Palette;
use crate::ui::{a11y, actions, menu_bar, widgets};
use nc_commands::{CommandId, MenuItem, build_tab_context_menu};

/// The height of the title bar, in points.
pub const HEIGHT: f32 = 40.0;

/// What a click on a tab did.
#[derive(Default)]
struct TabHit {
    /// The body of the tab was clicked.
    activate: bool,
    /// The tab's close button was clicked.
    close: bool,
    /// The tab was right-clicked, which raises its context menu.
    menu: bool,
}

/// Draws the title bar and applies any tab changes it caused.
pub fn show(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    egui::Panel::top("title_bar")
        .exact_size(HEIGHT)
        .frame(egui::Frame::NONE.fill(shell.palette.chrome))
        .show(ui, |ui| {
            // Registered before the controls so the empty parts of the bar drag
            // the window while the controls still receive their clicks.
            //
            // `Sense::DRAG` rather than `click_and_drag`: a click that drags nothing
            // is not something this bar does, and the click sense would put the
            // whole title bar in the tab order as an unnamed stop (`NC-P007`).
            let bar = ui.max_rect();
            let drag = ui.interact(bar, ui.id().with("title_drag"), Sense::DRAG);
            if drag.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }

            let mut to_activate: Option<usize> = None;
            let mut to_close: Option<usize> = None;
            // A command chosen from a tab's context menu. Collected during the
            // draw and run after it, the way a tab click is: the items act on
            // the tab the menu was raised from, which right-clicking has already
            // made the active one (`NC-T003`, `docs/ui-reference.md` §11).
            let mut chosen: Option<CommandId> = None;
            let tab_menu = build_tab_context_menu(shell.availability());

            let palette = shell.palette;
            ui.horizontal_centered(|ui| {
                app_icon(ui, palette);
                // Captured, and inert in this build: there is nowhere to go back
                // to. It is named and announced as unavailable rather than left
                // out, so the screen a reader is told about is the one that is
                // drawn (`NC-P007`, `docs/ui-reference.md` §13).
                let back = widgets::icon_button(ui, "‹");
                a11y::label(&back, WidgetType::Button, "Back");
                a11y::inert(ui.ctx(), back.id);
                ui.add_space(2.0);

                egui::ScrollArea::horizontal()
                    .id_salt("tabs")
                    .show(ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            for (index, tab) in shell.tabs.iter().enumerate() {
                                let hit = tab_widget(
                                    ui,
                                    index,
                                    tab,
                                    index == shell.active,
                                    palette,
                                    &tab_menu,
                                    &mut chosen,
                                );
                                if hit.close {
                                    to_close = Some(index);
                                } else if hit.activate || hit.menu {
                                    // A right-click selects the tab it landed on
                                    // before it offers anything, so every item
                                    // then acts on the document the user aimed at
                                    // rather than on whichever tab happened to be
                                    // active — and an item that cannot be honoured
                                    // is greyed rather than wrong.
                                    to_activate = Some(index);
                                }
                            }
                        });
                    });

                let new_tab = widgets::icon_button(ui, "+");
                a11y::label(&new_tab, WidgetType::Button, "New tab");
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
            if let Some(command) = chosen {
                actions::run(shell, ctx, command);
            }
        });
}

/// The three window buttons, right to left.
fn window_controls(ui: &mut egui::Ui, ctx: &Context, shell: &mut Shell) {
    let close = widgets::icon_button(ui, "✕");
    a11y::label(&close, WidgetType::Button, "Close window");
    if close.clicked() {
        // Closing the window prompts for unsaved documents (`NC-D005`).
        actions::close_window(shell);
    }
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let maximize_glyph = if maximized { "❐" } else { "▢" };
    let maximize = widgets::icon_button(ui, maximize_glyph);
    a11y::label(
        &maximize,
        WidgetType::Button,
        if maximized {
            "Restore window"
        } else {
            "Maximize window"
        },
    );
    if maximize.clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    let minimize = widgets::icon_button(ui, "—");
    a11y::label(&minimize, WidgetType::Button, "Minimize window");
    if minimize.clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

/// A small blue app icon.
fn app_icon(ui: &mut egui::Ui, palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(5), palette.accent);
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
///
/// `tab_menu` is the registry's tab context menu, already resolved against the
/// shell's availability; `chosen` collects a command the user picks from it.
fn tab_widget(
    ui: &mut egui::Ui,
    index: usize,
    tab: &Tab,
    active: bool,
    palette: Palette,
    tab_menu: &[MenuItem],
    chosen: &mut Option<CommandId>,
) -> TabHit {
    let height = 34.0;
    let title = tab.title();
    let width = (title.chars().count() as f32 * 7.5 + 52.0).clamp(120.0, 220.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());

    // What a screen reader is told about the tab: its document's name, which one
    // is being shown, and whether anything in it is unsaved — the last of which
    // is a dot on the strip and nothing else (`NC-P007`).
    a11y::label(&response, WidgetType::SelectableLabel, &title);
    a11y::role(ui.ctx(), response.id, Role::Tab);
    a11y::selected(ui.ctx(), response.id, active);
    if tab.is_dirty() {
        a11y::described(ui.ctx(), response.id, "Unsaved changes");
    }

    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 16.0, rect.center().y),
        egui::vec2(22.0, 22.0),
    );
    let close = ui.interact(
        close_rect,
        ui.id().with(("tab_close", index)),
        Sense::click(),
    );
    a11y::label(&close, WidgetType::Button, format!("Close {title}"));

    let painter = ui.painter();
    let fill = if active {
        palette.bg
    } else if response.hovered() {
        palette.row_hover
    } else {
        palette.chrome
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
        painter.rect_filled(underline, CornerRadius::same(1), palette.accent);
    }

    let text_color = if active {
        palette.text
    } else {
        palette.text_dim
    };
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
            palette.accent,
        );
    }

    if active || response.hovered() {
        if close.hovered() {
            painter.rect_filled(close_rect, CornerRadius::same(4), palette.hover);
        }
        painter.text(
            close_rect.center(),
            Align2::CENTER_CENTER,
            "✕",
            FontId::proportional(11.0),
            palette.text_dim,
        );
    }

    // Right-click raises the tab's context menu: the registry's item set, drawn
    // with the same rows as the menu bar, so the strip cannot offer something the
    // menus do not already carry (`NC-T003`, `docs/ui-reference.md` §11).
    let _ = response.context_menu(|ui| {
        for item in tab_menu {
            match item {
                MenuItem::Separator => {
                    let _ = ui.separator();
                }
                MenuItem::Command {
                    spec,
                    accelerator,
                    enabled,
                    checked,
                } => {
                    if menu_bar::command_button(
                        ui,
                        spec.label,
                        *checked,
                        accelerator.as_deref(),
                        *enabled,
                        palette,
                    ) {
                        *chosen = Some(spec.id);
                        ui.close();
                    }
                }
                // The tab menu is a flat list; a submenu would need the menu
                // bar's own renderer, and nothing here is one.
                MenuItem::Submenu { .. } => {}
            }
        }
    });

    TabHit {
        activate: response.clicked() && !close.hovered(),
        close: close.clicked(),
        menu: response.secondary_clicked(),
    }
}
