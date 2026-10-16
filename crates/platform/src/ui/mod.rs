//! The window shell: layout composition and the eframe application.

mod actions;
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
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let text = std::fs::read(path)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        shell.tabs.push(Tab::from_file(title, text));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(shell: &mut Shell) -> egui::FullOutput {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1180.0, 720.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| draw(shell, ui));
        output.textures_delta.clear();
        output
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
}
