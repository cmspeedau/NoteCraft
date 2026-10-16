//! The top bar: the `File`/`Edit`/`View` menus, the formatting toolbar, and the
//! account / settings controls.

use egui::{Align, Context, Layout, RichText};

use crate::ui::state::{ParagraphStyle, Shell};
use crate::ui::{actions, theme, widgets};
use nc_commands::{CommandId, MenuItem, all_disabled, build_menu_bar, by_id};

/// The height of the top bar, in points.
pub const HEIGHT: f32 = 46.0;

/// Draws the top bar.
pub fn show(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    ui.horizontal_centered(|ui| {
        menus(shell, ui, ctx);
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
        format_toolbar(shell, ui, ctx);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            right_toolbar(shell, ui, ctx);
        });
    });
}

/// The `File` / `Edit` / `View` menu buttons.
fn menus(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    for model in build_menu_bar(shell.availability()) {
        ui.menu_button(model.menu.label(), |ui| {
            menu_items(ui, &model.items, shell, ctx);
        });
    }
}

/// Renders one dropdown's items.
fn menu_items(ui: &mut egui::Ui, items: &[MenuItem], shell: &mut Shell, ctx: &Context) {
    for item in items {
        match item {
            MenuItem::Separator => {
                ui.separator();
            }
            MenuItem::Command {
                spec,
                accelerator,
                enabled,
                checked,
            } => {
                let text = command_label(spec.label, *checked);
                let mut button = egui::Button::new(text).min_size(egui::vec2(210.0, 0.0));
                if let Some(accel) = accelerator {
                    button = button.shortcut_text(RichText::new(accel).color(theme::TEXT_DIM));
                }
                if ui.add_enabled(*enabled, button).clicked() {
                    actions::run(shell, ctx, spec.id);
                    ui.close();
                }
            }
            MenuItem::Submenu { title, items } => {
                if all_disabled(items) {
                    let _ = ui.add_enabled(false, egui::Button::new(*title));
                } else {
                    let _ = ui.menu_button(*title, |ui| menu_items(ui, items, shell, ctx));
                }
            }
        }
    }
}

/// Prefixes a check mark (or the equivalent space) for a toggle item.
fn command_label(label: &str, checked: Option<bool>) -> RichText {
    match checked {
        Some(true) => RichText::new(format!("✓   {label}")),
        Some(false) => RichText::new(format!("     {label}")),
        None => RichText::new(label),
    }
}

/// The formatting controls in the middle of the bar.
fn format_toolbar(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    paragraph_menu(shell, ui);
    list_menu(ui, ctx);

    let bold = widgets::rich_button(ui, RichText::new("B").strong()).on_hover_text("Bold");
    if bold.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.bold"));
    }
    let italic = widgets::rich_button(ui, RichText::new("I").italics()).on_hover_text("Italic");
    if italic.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.italic"));
    }
    let strike =
        widgets::rich_button(ui, RichText::new("S").strikethrough()).on_hover_text("Strikethrough");
    if strike.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.strike"));
    }
    let link = widgets::icon_button(ui, "🔗").on_hover_text("Insert a link");
    if link.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.link"));
    }
    let _ = widgets::icon_button(ui, "▦").on_hover_text("Insert a table");
    let clear = widgets::rich_button(ui, RichText::new("Aa")).on_hover_text("Clear all formatting");
    if clear.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.clear"));
    }
}

/// The `H1` paragraph-style dropdown.
fn paragraph_menu(shell: &mut Shell, ui: &mut egui::Ui) {
    let current = shell
        .active_tab()
        .map(|tab| tab.paragraph)
        .unwrap_or(ParagraphStyle::Body);
    ui.menu_button("H1 ▾", |ui| {
        for style in ParagraphStyle::ALL {
            if ui
                .selectable_label(style == current, style.label())
                .clicked()
            {
                if let Some(tab) = shell.active_tab_mut() {
                    tab.paragraph = style;
                }
                ui.close();
            }
        }
    });
}

/// The bulleted / numbered list dropdown.
fn list_menu(ui: &mut egui::Ui, ctx: &Context) {
    ui.menu_button("≡ ▾", |ui| {
        let _ = ctx;
        if ui.button("Bulleted list").clicked() {
            ui.close();
        }
        if ui.button("Numbered list").clicked() {
            ui.close();
        }
        if ui
            .add_enabled(false, egui::Button::new("Increase indent"))
            .clicked()
        {
            ui.close();
        }
        if ui
            .add_enabled(false, egui::Button::new("Decrease indent"))
            .clicked()
        {
            ui.close();
        }
    });
}

/// The sparkle / feedback / account / settings controls on the right.
fn right_toolbar(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    if widgets::icon_button(ui, "⚙")
        .on_hover_text("Settings")
        .clicked()
    {
        actions::run(shell, ctx, CommandId("cmd.app.settings"));
    }
    avatar(ui);
    let _ = widgets::icon_button(ui, "💬").on_hover_text("Send feedback");

    // The AI ("sparkle") menu. Its entries are service-gated, so they show
    // greyed until the service is configured.
    let context = shell.availability();
    ui.menu_button("✨ ▾", |ui| {
        for id in ["cmd.ai.rewrite", "cmd.ai.summarize", "cmd.ai.write"] {
            if let Some(spec) = by_id(id) {
                let enabled = spec.is_available(context);
                if ui
                    .add_enabled(enabled, egui::Button::new(spec.label))
                    .clicked()
                {
                    ui.close();
                }
            }
        }
    });
}

/// The round account avatar.
fn avatar(ui: &mut egui::Ui) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    ui.painter()
        .circle_filled(rect.center(), 13.0, theme::BORDER);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "👤",
        egui::FontId::proportional(13.0),
        theme::TEXT,
    );
    let _ = response.on_hover_text("Account");
}
