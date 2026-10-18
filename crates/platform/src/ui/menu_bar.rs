//! The top bar: the `File`/`Edit`/`View` menus, the formatting toolbar, and the
//! account / settings controls.

use egui::{Align, Context, Layout, RichText, WidgetType};

use crate::ui::state::{ParagraphStyle, Shell};
use crate::ui::theme::Palette;
use crate::ui::{a11y, actions, widgets};
use nc_commands::{CommandId, MenuItem, all_disabled, build_menu_bar, by_id};

/// The height of the top bar, in points.
pub const HEIGHT: f32 = 46.0;

/// Draws the top bar.
pub fn show(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    ui.horizontal_centered(|ui| {
        let palette = shell.palette;
        menus(shell, ui, ctx, palette);
        // `Settings ▸ Formatting` off takes the formatting half of the bar away, which is
        // the whole of what that switch's acceptance row asks for (`NC-P005`). The divider
        // goes with it: it is the seam between the menus and this group, and a divider to
        // nothing is a border around an absence. What the setting does *not* do is recorded
        // in `docs/ui-reference.md` §4 — the accelerators keep formatting, because nothing
        // in the capture shows a formatting key that has stopped.
        if shell.formatting {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            format_toolbar(shell, ui, ctx, palette);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            right_toolbar(shell, ui, ctx, palette);
        });
    });
}

/// The `File` / `Edit` / `View` menu buttons.
fn menus(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context, palette: Palette) {
    for model in build_menu_bar(shell.availability()) {
        ui.menu_button(model.menu.label(), |ui| {
            menu_items(ui, &model.items, shell, ctx, palette);
        });
    }
}

/// Renders one dropdown's items.
fn menu_items(
    ui: &mut egui::Ui,
    items: &[MenuItem],
    shell: &mut Shell,
    ctx: &Context,
    palette: Palette,
) {
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
                if command_button(
                    ui,
                    spec.label,
                    *checked,
                    accelerator.as_deref(),
                    *enabled,
                    palette,
                ) {
                    actions::run(shell, ctx, spec.id);
                    ui.close();
                }
            }
            MenuItem::Submenu { title, items } => {
                if is_recent_submenu(items) && !shell.recent_files.is_empty() {
                    recent_submenu(title, shell, ui);
                } else if all_disabled(items) {
                    let _ = ui.add_enabled(false, egui::Button::new(*title));
                } else {
                    let _ = ui.menu_button(*title, |ui| menu_items(ui, items, shell, ctx, palette));
                }
            }
        }
    }
}

/// Whether this submenu is the one the registry defines for recent documents —
/// the one holding `cmd.file.recent_empty`, whose placeholder text the real list
/// stands in for. Recognized by its command rather than its label, so relabelling
/// the menu cannot leave the list unreachable.
fn is_recent_submenu(items: &[MenuItem]) -> bool {
    items
        .iter()
        .any(|item| item.command().map(|spec| spec.id.as_str()) == Some("cmd.file.recent_empty"))
}

/// `File ▸ Recent`: the documents opened this session, newest first, each of which
/// opens again on click (`NC-F010`). The `title` is the registry's own, so the
/// submenu is labelled whatever the menu model says it should be.
///
/// The list is read out before any of it is acted on, because opening a file
/// rewrites it and an entry may move to the front. An empty list is not drawn
/// here at all: the registry's `cmd.file.recent_empty` says "No recent files",
/// which is what the menu already shows for it.
fn recent_submenu(title: &'static str, shell: &mut Shell, ui: &mut egui::Ui) {
    let paths = shell.recent_files.clone();
    let _ = ui.menu_button(title, |ui| {
        for path in &paths {
            let name = path.file_name().map_or_else(
                || path.to_string_lossy().into_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            let button = egui::Button::new(name).min_size(egui::vec2(210.0, 0.0));
            if ui
                .add(button)
                .on_hover_text(path.display().to_string())
                .clicked()
            {
                actions::open_path(shell, path, None);
                ui.close();
            }
        }
    });
}

/// Prefixes a check mark (or the equivalent space) for a toggle item.
fn command_label(label: &str, checked: Option<bool>) -> RichText {
    match checked {
        Some(true) => RichText::new(format!("✓   {label}")),
        Some(false) => RichText::new(format!("     {label}")),
        None => RichText::new(label),
    }
}

/// One menu row: the registry's label, its accelerator right-aligned, greyed
/// when the command's availability rule says so — and whether it was chosen.
///
/// Shared with the tab context menu (`title_bar`) so the two surfaces cannot
/// drift apart in what a row looks like or when it is clickable.
pub(crate) fn command_button(
    ui: &mut egui::Ui,
    label: &str,
    checked: Option<bool>,
    accelerator: Option<&str>,
    enabled: bool,
    palette: Palette,
) -> bool {
    let mut button =
        egui::Button::new(command_label(label, checked)).min_size(egui::vec2(210.0, 0.0));
    if let Some(accel) = accelerator {
        button = button.shortcut_text(RichText::new(accel).color(palette.text_dim));
    }
    ui.add_enabled(enabled, button).clicked()
}

/// The formatting controls in the middle of the bar.
fn format_toolbar(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context, _palette: Palette) {
    paragraph_menu(shell, ui, ctx);
    list_menu(shell, ui, ctx);

    let bold = widgets::rich_button(ui, RichText::new("B").strong()).on_hover_text("Bold");
    a11y::label(&bold, WidgetType::Button, "Bold");
    if bold.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.bold"));
    }
    let italic = widgets::rich_button(ui, RichText::new("I").italics()).on_hover_text("Italic");
    a11y::label(&italic, WidgetType::Button, "Italic");
    if italic.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.italic"));
    }
    let strike =
        widgets::rich_button(ui, RichText::new("S").strikethrough()).on_hover_text("Strikethrough");
    a11y::label(&strike, WidgetType::Button, "Strikethrough");
    if strike.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.strike"));
    }
    let link = widgets::icon_button(ui, "🔗").on_hover_text("Insert a link");
    a11y::label(&link, WidgetType::Button, "Insert a link");
    if link.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.link"));
    }
    let table = widgets::icon_button(ui, "▦").on_hover_text("Insert a table");
    a11y::label(&table, WidgetType::Button, "Insert a table");
    if table.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.table"));
    }
    let clear = widgets::rich_button(ui, RichText::new("Aa")).on_hover_text("Clear all formatting");
    a11y::label(&clear, WidgetType::Button, "Clear all formatting");
    if clear.clicked() {
        actions::run(shell, ctx, CommandId("cmd.md.clear"));
    }
}

/// The `H1` paragraph-style dropdown.
///
/// The item list is the registry's, in §3's order, and choosing one fires its
/// command rather than setting the style here: the dropdown, the menu, and the
/// shortcut have to write the same bytes, and only `nc-markdown` decides what a
/// style is.
fn paragraph_menu(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    let current = shell
        .active_tab()
        .map(|tab| tab.paragraph)
        .unwrap_or(ParagraphStyle::Body);
    let menu = ui.menu_button("H1 ▾", |ui| {
        for style in ParagraphStyle::ALL {
            if ui
                .selectable_label(style == current, style.label())
                .clicked()
            {
                actions::run(shell, ctx, style.command());
                ui.close();
            }
        }
    });
    // The visible label is the current style; what the button is for is the
    // paragraph style as a whole (`NC-P007`).
    a11y::label(&menu.response, WidgetType::ComboBox, "Paragraph style");
}

/// The bulleted / numbered list dropdown (`docs/ui-reference.md` §3: *Bulleted
/// list · Numbered list · Increase indent · Decrease indent, indent items greyed
/// when they cannot apply*).
///
/// All four are the registry's commands, drawn with the registry's labels, so the
/// dropdown cannot offer something the menu does not. What the registry can answer
/// for "cannot apply" today is the document kind — these are Markdown-only
/// commands — and that is what gates them. Whether a particular line has an indent
/// to take away is not a rule `Availability` models, and this does not pretend to
/// know it.
fn list_menu(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context) {
    let context = shell.availability();
    let menu = ui.menu_button("≡ ▾", |ui| {
        for id in [
            "cmd.md.bullets",
            "cmd.md.numbering",
            "cmd.md.indent_increase",
            "cmd.md.indent_decrease",
        ] {
            let Some(spec) = by_id(id) else {
                continue;
            };
            let enabled = spec.is_available(context);
            if ui
                .add_enabled(enabled, egui::Button::new(spec.label))
                .clicked()
            {
                actions::run(shell, ctx, spec.id);
                ui.close();
            }
        }
    });
    a11y::label(&menu.response, WidgetType::ComboBox, "Lists");
}

/// The sparkle / feedback / account / settings controls on the right.
fn right_toolbar(shell: &mut Shell, ui: &mut egui::Ui, ctx: &Context, palette: Palette) {
    let settings = widgets::icon_button(ui, "⚙").on_hover_text("Settings");
    a11y::label(&settings, WidgetType::Button, "Settings");
    if settings.clicked() {
        actions::run(shell, ctx, CommandId("cmd.app.settings"));
    }
    avatar(ui, palette);
    let feedback = widgets::icon_button(ui, "💬").on_hover_text("Send feedback");
    a11y::label(&feedback, WidgetType::Button, "Send feedback");

    // The AI ("sparkle") menu. Its entries are service-gated, so they show
    // greyed until the service is configured.
    let context = shell.availability();
    let ai = ui.menu_button("✨ ▾", |ui| {
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
    a11y::label(&ai.response, WidgetType::ComboBox, "AI");
}

/// The round account avatar.
fn avatar(ui: &mut egui::Ui, palette: Palette) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    a11y::label(&response, WidgetType::Button, "Account");
    ui.painter()
        .circle_filled(rect.center(), 13.0, palette.border);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "👤",
        egui::FontId::proportional(13.0),
        palette.text,
    );
    let _ = response.on_hover_text("Account");
}
