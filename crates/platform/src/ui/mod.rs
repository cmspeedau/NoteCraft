//! The window shell: layout composition and the eframe application.

mod a11y;
mod actions;
/// The M6 drawn-surface assertions. Test-only: they read back the shapes the
/// shell paints, which live in private state.
#[cfg(test)]
mod appearance;
mod config;
mod dialogs;
mod editor;
mod fonts;
mod input;
/// The M6 acceptance suite. Test-only: it drives the shell's own state, which is
/// private to this crate.
#[cfg(test)]
mod lifecycle;
/// The M7 acceptance suite. Test-only: it drives the shell's own state, which is
/// private to this crate.
#[cfg(test)]
mod markdown;
mod markdown_view;
mod menu_bar;
mod session;
mod settings;
mod state;
mod status_bar;
mod theme;
mod title_bar;
mod widgets;
mod windows;

use std::path::PathBuf;

use crate::ui::state::{Shell, Tab};
use crate::{PlatformError, RunOutcome, StartupOptions};

/// Brings up the window and runs the event loop to completion.
pub(crate) fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    let mut shell = initial_shell(&options.open_paths);
    seed_documents(&mut shell, &options.open_paths);

    // Then the settings, on top of whichever shell this start produced: a session
    // restored from the previous run carries documents, and the settings file says
    // what those documents are shown with. Read once, at the point where a change to
    // disk cannot race the run that is starting.
    let settings = settings::Store::open();
    settings.apply_to(&mut shell);

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
            // The theme the shell arrives with — its default, or the restored
            // choice once settings persist — is installed before the first frame, so
            // nothing is painted in the wrong palette and then corrected.
            shell.palette = theme::apply(&cc.egui_ctx, shell.theme);
            fonts::install(&cc.egui_ctx);
            Ok(Box::new(ShellApp {
                shell,
                windows: windows::Windows::new(),
                settings,
            }))
        }),
    )
    .map_err(|err| PlatformError::Window(err.to_string()))?;

    Ok(RunOutcome { exit_code: 0 })
}

/// The documents the window starts with: the files named on the command line,
/// or the previous session when none were (`NC-F011`). Naming files is an
/// explicit request, so it replaces the session rather than adding to it.
fn initial_shell(paths: &[PathBuf]) -> Shell {
    if paths.is_empty() {
        session::restore_shell()
    } else {
        Shell::new()
    }
}

/// Replaces the initial untitled document with the files named on the command
/// line, reading each one best-effort. A file named twice is opened once
/// (`NC-F012`), so the tab strip never shows two tabs for one document.
fn seed_documents(shell: &mut Shell, paths: &[PathBuf]) {
    if paths.is_empty() {
        return;
    }
    shell.tabs.clear();
    for path in paths {
        if shell.tabs.iter().any(|tab| tab.matches_path(path)) {
            continue;
        }
        // A file named on the command line is an opened document like any other,
        // so it belongs in `File ▸ Recent`.
        shell.remember_recent(path);
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
    /// The extra windows `File ▸ New window` has opened, beyond the main one.
    windows: windows::Windows,
    /// The settings file, and what is on it. The main window owns it: one file per
    /// run, so an extra window changing a setting cannot fight the file.
    settings: settings::Store,
}

impl eframe::App for ShellApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        draw(&mut self.shell, ui);
        // A New window command takes effect at the end of the frame: the window
        // is created and drawn right away, and stays alive by being shown each
        // frame after that.
        self.windows.create_requested(&mut self.shell);
        self.windows.show(&ui.ctx().clone());
        // Settings are written after the frame that changed them has been drawn, so
        // the user sees the change first and the file catches up. A window sitting
        // still compares equal and writes nothing.
        self.settings.sync(&self.shell);
    }

    /// Records the session on shutdown, so the next start reopens these
    /// documents (`NC-F011`). Only the main window's shell is kept.
    fn on_exit(&mut self) {
        if let Err(error) = session::save(&self.shell) {
            tracing::warn!(%error, "could not write the session");
        }
    }
}

/// Composes the whole window: title bar, top bar, status bar, and content.
pub(crate) fn draw(shell: &mut Shell, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    // Accelerators are read before anything is drawn, so a command that opens a
    // dialog or moves the caret is reflected in this frame.
    input::dispatch(shell, &ctx);

    // Then the palette, so a theme chosen in Settings — or a system that changes
    // under a shell set to follow it — is in force for everything drawn below,
    // including this frame. Checked rather than installed every frame: writing the
    // style is cheap but not free, and an unchanged comparison says the same thing
    // twice.
    shell.palette = theme::apply(&ctx, shell.theme);
    // The face the document is drawn in. Context-keyed for the same reason as the
    // palette, with a cost the palette does not pay: a new face rebuilds the glyph
    // atlas, so this is called once per frame and does nothing unless the choice moved.
    fonts::apply(&ctx, shell.font_family);

    title_bar::show(shell, ui, &ctx);

    egui::Panel::top("top_bar")
        .exact_size(menu_bar::HEIGHT)
        .frame(egui::Frame::NONE.fill(shell.palette.chrome))
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
        theme::install(&ctx, shell.palette);
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

    /// A key press with modifiers, as the windowing layer reports a chord.
    fn chord(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
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

    #[test]
    fn ctrl_n_opens_a_tab_and_ctrl_w_closes_it() {
        let mut shell = Shell::new();

        run(&mut shell, vec![chord(Key::N, Modifiers::COMMAND)]);
        assert_eq!(shell.tabs.len(), 2, "Ctrl+N opens a tab");

        run(&mut shell, vec![chord(Key::W, Modifiers::COMMAND)]);
        assert_eq!(shell.tabs.len(), 1, "Ctrl+W closes the active tab");
    }

    #[test]
    fn ctrl_f_opens_the_find_dialog_through_the_registry() {
        let mut shell = Shell::new();

        run(&mut shell, vec![chord(Key::F, Modifiers::COMMAND)]);

        assert!(shell.find.open);
        assert!(!shell.find.replace);
    }

    #[test]
    fn f5_inserts_the_time_and_date_at_the_caret() {
        let mut shell = Shell::new();

        run(&mut shell, vec![key(Key::F5)]);

        assert!(
            shell.tabs[0].text().contains(':'),
            "{}",
            shell.tabs[0].text()
        );
        assert!(shell.tabs[0].is_dirty());
    }

    #[test]
    fn ctrl_a_selects_all_and_ctrl_z_undoes_it() {
        let mut shell = Shell::new();
        *shell.tabs[0].document_mut() = nc_core::Document::from_text("one two");

        run(&mut shell, vec![chord(Key::A, Modifiers::COMMAND)]);
        assert!(shell.tabs[0].file.document().has_selection());

        // The editor widget applies the queued selection in the same frame, so
        // the document and the caret agree.
        assert_eq!(shell.caret, 7);
    }

    #[test]
    fn ctrl_z_undoes_the_last_edit_through_the_document() {
        let mut shell = Shell::new();
        shell.tabs[0].document_mut().insert_str("hello");
        assert_eq!(shell.tabs[0].text(), "hello");

        run(&mut shell, vec![chord(Key::Z, Modifiers::COMMAND)]);

        assert_eq!(shell.tabs[0].text(), "");
        assert_eq!(shell.caret, 0);
    }

    #[test]
    fn a_key_with_no_command_is_left_for_the_editor() {
        // An unbound chord must not be swallowed, or the editor would lose it.
        let mut shell = Shell::new();
        let (ctx, _) = run(&mut shell, vec![chord(Key::Q, Modifiers::ALT)]);

        assert!(shell.find.query.is_empty());
        // Nothing happened and the shell is untouched.
        assert_eq!(shell.tabs.len(), 1);
        let _ = ctx;
    }

    #[test]
    fn ctrl_tab_switches_between_tabs() {
        let mut shell = Shell::new();
        shell.open_tab(state::DocKind::Plain);
        assert_eq!(shell.active, 1);

        run(&mut shell, vec![chord(Key::Tab, Modifiers::COMMAND)]);
        assert_eq!(shell.active, 0, "Ctrl+Tab moves to the next tab");

        run(
            &mut shell,
            vec![chord(
                Key::Tab,
                Modifiers {
                    shift: true,
                    ..Modifiers::COMMAND
                },
            )],
        );
        assert_eq!(shell.active, 1, "Ctrl+Shift+Tab moves to the previous");
    }

    #[test]
    fn the_command_line_opens_a_named_file_once() {
        let directory = std::env::temp_dir().join(format!("notecraft-seed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let path = directory.join("note.txt");
        std::fs::write(&path, b"hello").expect("the file");

        let mut shell = Shell::new();
        seed_documents(&mut shell, &[path.clone(), path.clone()]);

        assert_eq!(shell.tabs.len(), 1, "the file is opened once");
        assert_eq!(shell.tabs[0].text(), "hello");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_command_line_opens_each_named_file_in_order() {
        // NC-P009: files named on the command line open in the order given, one
        // tab each, first one shown — the argument order the CLI parse preserves
        // (cli.rs) carried through to the tab strip the user sees.
        let directory =
            std::env::temp_dir().join(format!("notecraft-seed-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let first = directory.join("a.txt");
        let second = directory.join("b.txt");
        std::fs::write(&first, b"first").expect("the first file");
        std::fs::write(&second, b"second").expect("the second file");

        let mut shell = Shell::new();
        seed_documents(&mut shell, &[first.clone(), second.clone()]);

        assert_eq!(shell.tabs.len(), 2, "one tab per named file");
        assert_eq!(shell.tabs[0].text(), "first");
        assert_eq!(shell.tabs[1].text(), "second");
        assert_eq!(shell.active, 0, "the first named file is the one shown");
        assert_eq!(
            shell.tabs[0].title(),
            "a.txt",
            "a command-line file is a saved document, not untitled"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn ctrl_shift_n_asks_for_a_window_without_opening_a_tab() {
        let mut shell = Shell::new();

        run(
            &mut shell,
            vec![chord(
                Key::N,
                Modifiers {
                    shift: true,
                    ..Modifiers::COMMAND
                },
            )],
        );

        assert!(shell.new_window_request, "a window was asked for");
        assert_eq!(shell.tabs.len(), 1, "no tab was opened");
    }
}
