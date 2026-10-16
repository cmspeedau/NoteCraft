//! What a command does when it fires.
//!
//! The registry says what commands exist and whether they are available; this
//! module is the one place that turns a fired command into a change in the
//! shell. Commands that belong to a later milestone are present in the menu but
//! have no effect yet, and are simply ignored here.

use egui::Context;

use crate::ui::state::{DocKind, ParagraphStyle, Shell};
use nc_commands::CommandId;

/// Runs the command with the given id against the shell.
pub fn run(shell: &mut Shell, ctx: &Context, id: CommandId) {
    match id.as_str() {
        "cmd.file.new_tab" => shell.open_tab(DocKind::Plain),
        "cmd.file.new_markdown" => shell.open_tab(DocKind::Markdown),
        "cmd.file.close_tab" => shell.close_tab(shell.active),
        "cmd.file.close_window" | "cmd.file.exit" => {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        "cmd.view.status_bar" => shell.status_bar_visible = !shell.status_bar_visible,
        "cmd.view.word_wrap" => shell.word_wrap = !shell.word_wrap,
        "cmd.view.zoom_in" => shell.zoom = (shell.zoom + 10).min(500),
        "cmd.view.zoom_out" => shell.zoom = shell.zoom.saturating_sub(10).max(10),
        "cmd.view.zoom_reset" => shell.zoom = 100,
        "cmd.view.md_source" => {
            if let Some(tab) = shell.active_tab_mut() {
                tab.formatted = false;
            }
            shell.formatted_view = false;
        }
        "cmd.view.md_formatted" => {
            if let Some(tab) = shell.active_tab_mut() {
                tab.formatted = true;
            }
            shell.formatted_view = true;
        }
        "cmd.app.settings" => shell.show_settings = !shell.show_settings,
        "cmd.format.title" => set_paragraph(shell, ParagraphStyle::Title),
        "cmd.format.subtitle" => set_paragraph(shell, ParagraphStyle::Subtitle),
        "cmd.format.heading" => set_paragraph(shell, ParagraphStyle::Heading),
        "cmd.format.subheading" => set_paragraph(shell, ParagraphStyle::Subheading),
        "cmd.format.section" => set_paragraph(shell, ParagraphStyle::Section),
        "cmd.format.subsection" => set_paragraph(shell, ParagraphStyle::Subsection),
        "cmd.format.body" => set_paragraph(shell, ParagraphStyle::Body),
        // Everything else is present in the surface but inert until its
        // milestone lands.
        _ => {}
    }
}

fn set_paragraph(shell: &mut Shell, style: ParagraphStyle) {
    if let Some(tab) = shell.active_tab_mut() {
        tab.paragraph = style;
    }
}
