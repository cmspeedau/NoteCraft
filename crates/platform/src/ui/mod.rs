//! The window shell: layout composition and the eframe application.

mod actions;
mod dialogs;
mod editor;
mod menu_bar;
mod settings;
mod state;
mod status_bar;
mod theme;
mod title_bar;
mod widgets;

use std::path::PathBuf;

use crate::ui::state::{Shell, Tab};
use crate::{PlatformError, RunOutcome, StartupOptions};

/// Brings up the window and runs the event loop to completion.
pub(crate) fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    let mut shell = Shell::new();
    seed_documents(&mut shell, &options.open_paths);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(options.title)
            .with_inner_size([1180.0, 720.0])
            .with_min_inner_size([720.0, 420.0])
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "NoteCraft",
        native_options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(ShellApp { shell }))
        }),
    )
    .map_err(|err| PlatformError::Window(err.to_string()))?;

    Ok(RunOutcome { exit_code: 0 })
}

/// Replaces the initial untitled document with the files named on the command
/// line, reading each one best-effort.
fn seed_documents(shell: &mut Shell, paths: &[PathBuf]) {
    if paths.is_empty() {
        return;
    }
    shell.tabs.clear();
    for path in paths {
        // A file that cannot be read leaves an untitled tab in its place;
        // reporting why is a later milestone's job.
        let tab = Tab::open(path).unwrap_or_else(|_| Tab::plain());
        shell.tabs.push(tab);
    }
    shell.active = 0;
}

/// The eframe application. All the drawing lives in [`draw`] so it can be
/// exercised headlessly.
struct ShellApp {
    shell: Shell,
}

impl eframe::App for ShellApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        draw(&mut self.shell, ui);
    }
}

/// Composes the whole window: title bar, top bar, status bar, and content.
pub(crate) fn draw(shell: &mut Shell, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    title_bar::show(shell, ui, &ctx);

    egui::Panel::top("top_bar")
        .exact_size(menu_bar::HEIGHT)
        .frame(egui::Frame::NONE.fill(theme::CHROME))
        .show(ui, |ui| {
            menu_bar::show(shell, ui, &ctx);
        });

    if shell.status_bar_visible && !shell.show_settings {
        status_bar::show(shell, ui);
    }

    if shell.show_settings {
        settings::show(shell, ui);
    } else {
        editor::show(shell, ui);
    }

    // Dialogs sit over either surface; a prompt can be raised from Settings too.
    dialogs::show(shell, &ctx);

    // Act on a window close the window manager asked for, and on one queued by
    // the shell.
    actions::pump(shell, &ctx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers};

    fn run(shell: &mut Shell, events: Vec<egui::Event>) -> (egui::Context, egui::FullOutput) {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1180.0, 720.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| draw(shell, ui));
        output.textures_delta.clear();
        (ctx, output)
    }

    fn frame(shell: &mut Shell) -> egui::FullOutput {
        run(shell, Vec::new()).1
    }

    fn key(key: Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn a_queued_selection_moves_the_editor_caret() {
        let mut shell = Shell::new();
        *shell.tabs[0].document_mut() = nc_core::Document::from_text("one two one");
        shell.pending_selection = Some((3, 0));

        let (ctx, _) = run(&mut shell, Vec::new());

        let state = egui::TextEdit::load_state(&ctx, egui::Id::new(editor::EDITOR_ID))
            .expect("editor state stored");
        let range = state.cursor.char_range().expect("a selection");
        assert_eq!(usize::from(range.primary.index), 3);
        assert_eq!(usize::from(range.secondary.index), 0);
        assert_eq!(shell.caret, 3);
        assert!(shell.pending_selection.is_none());
    }

    #[test]
    fn the_shell_paints_a_frame() {
        let mut shell = Shell::new();
        let output = frame(&mut shell);
        assert!(!output.shapes.is_empty(), "the shell painted nothing");
    }

    #[test]
    fn the_settings_page_paints_a_frame() {
        let mut shell = Shell::new();
        shell.show_settings = true;
        let output = frame(&mut shell);
        assert!(
            !output.shapes.is_empty(),
            "the settings page painted nothing"
        );
    }

    #[test]
    fn the_find_dialog_paints_a_frame() {
        let mut shell = Shell::new();
        shell.find.open = true;
        shell.find.replace = true;
        let output = frame(&mut shell);
        assert!(!output.shapes.is_empty(), "the find dialog painted nothing");
    }

    #[test]
    fn the_go_to_dialog_paints_a_frame() {
        let mut shell = Shell::new();
        shell.go_to.open = true;
        let output = frame(&mut shell);
        assert!(
            !output.shapes.is_empty(),
            "the go to dialog painted nothing"
        );
    }

    #[test]
    fn the_file_dialog_paints_a_frame() {
        let mut shell = Shell::new();
        actions::open_file_dialog(&mut shell);
        let output = frame(&mut shell);
        assert!(!output.shapes.is_empty(), "the file dialog painted nothing");

        shell.file_dialog = Some(state::FileDialog::save_as(
            0,
            "note.txt".to_owned(),
            nc_textproc::Encoding::Utf8,
            false,
        ));
        let output = frame(&mut shell);
        assert!(!output.shapes.is_empty(), "Save As painted nothing");
    }

    #[test]
    fn the_prompt_paints_a_frame() {
        let mut shell = Shell::new();
        shell.prompt = Some(state::Prompt::UnsavedChanges { tab: 0 });
        let output = frame(&mut shell);
        assert!(!output.shapes.is_empty(), "the prompt painted nothing");
    }

    #[test]
    fn escape_closes_the_find_dialog() {
        let mut shell = Shell::new();
        shell.find.open = true;
        run(&mut shell, vec![key(Key::Escape)]);
        assert!(!shell.find.open);
    }

    #[test]
    fn escape_closes_the_go_to_dialog() {
        let mut shell = Shell::new();
        shell.go_to.open = true;
        run(&mut shell, vec![key(Key::Escape)]);
        assert!(!shell.go_to.open);
    }

    #[test]
    fn the_replace_command_opens_the_dialog_in_replace_mode() {
        let mut shell = Shell::new();
        let ctx = egui::Context::default();
        actions::run(&mut shell, &ctx, nc_commands::CommandId("cmd.edit.replace"));
        assert!(shell.find.open);
        assert!(shell.find.replace);

        actions::run(&mut shell, &ctx, nc_commands::CommandId("cmd.edit.find"));
        assert!(shell.find.open);
        assert!(!shell.find.replace);
    }

    #[test]
    fn the_go_to_command_pre_fills_the_current_line() {
        let mut shell = Shell::new();
        shell.line = 7;
        let ctx = egui::Context::default();
        actions::run(&mut shell, &ctx, nc_commands::CommandId("cmd.edit.go_to"));
        assert!(shell.go_to.open);
        assert_eq!(shell.go_to.line, "7");
    }
}
