//! The dialog surfaces: Find / Replace, Go to line (`docs/ui-reference.md`
//! §5–§6), Open / Save As (§7), and the prompts that guard against data loss
//! (§9).
//!
//! The dialogs own no logic of their own: they edit the state the shell already
//! holds, and their buttons call [`actions`], the same place a menu item or a
//! keyboard shortcut lands.

use std::path::PathBuf;

use egui::{Align2, Context, Key, RichText, Vec2, WidgetType};

use crate::ui::a11y;
use crate::ui::actions;
use crate::ui::state::{FileDialogMode, Prompt, Shell};
use nc_textproc::Direction;

/// Draws whichever dialog is open. The file dialog is drawn before the prompt,
/// so a prompt raised over it sits on top.
pub fn show(shell: &mut Shell, ctx: &Context) {
    if shell.find.open {
        find(shell, ctx);
    }
    if shell.go_to.open {
        go_to(shell, ctx);
    }
    if shell.file_dialog.is_some() {
        file_dialog(shell, ctx);
    }
    if shell.prompt.is_some() {
        prompt(shell, ctx);
    }
}

/// The Find and Replace dialog (`docs/ui-reference.md` §5). Modeless, anchored
/// to the top-right of the editor surface.
fn find(shell: &mut Shell, ctx: &Context) {
    let mut open = shell.find.open;
    egui::Window::new("Find and Replace")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::RIGHT_TOP, Vec2::new(-24.0, 70.0))
        .show(ctx, |ui| {
            ui.set_min_width(360.0);

            ui.horizontal(|ui| {
                ui.label("Find what");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut shell.find.query)
                        .id(egui::Id::new("find_query"))
                        .desired_width(240.0),
                );
                a11y::label(&response, WidgetType::TextEdit, "Find what");
                if shell.find.focus {
                    response.request_focus();
                    shell.find.focus = false;
                }
                if response.lost_focus()
                    && ui.input(|input| input.key_pressed(Key::Enter))
                    && !shell.find.query.is_empty()
                {
                    let direction = shell.find.direction;
                    actions::search_step(shell, direction);
                }
            });

            if shell.find.replace {
                ui.horizontal(|ui| {
                    ui.label("Replace with");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut shell.find.replacement)
                            .desired_width(240.0),
                    );
                    a11y::label(&response, WidgetType::TextEdit, "Replace with");
                });
            }

            ui.horizontal(|ui| {
                ui.checkbox(&mut shell.find.options.match_case, "Match case");
                ui.checkbox(&mut shell.find.options.whole_word, "Match whole word only");
            });

            ui.horizontal(|ui| {
                ui.label("Direction");
                ui.radio_value(&mut shell.find.direction, Direction::Forward, "Down");
                ui.radio_value(&mut shell.find.direction, Direction::Backward, "Up");
            });

            ui.checkbox(&mut shell.find.wraps, "Wrap around");
            ui.separator();

            let has_query = !shell.find.query.is_empty();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_query, egui::Button::new("Find Next"))
                    .clicked()
                {
                    let direction = shell.find.direction;
                    actions::search_step(shell, direction);
                }
                if shell.find.replace {
                    if ui
                        .add_enabled(has_query, egui::Button::new("Replace"))
                        .clicked()
                    {
                        actions::replace(shell);
                    }
                    if ui
                        .add_enabled(has_query, egui::Button::new("Replace All"))
                        .clicked()
                    {
                        actions::replace_all(shell);
                    }
                }
                if ui.button("Close").clicked() {
                    shell.find.open = false;
                }
            });
        });

    if !open || ctx.input(|input| input.key_pressed(Key::Escape)) {
        shell.find.open = false;
    }
}

/// The Go to line dialog (`docs/ui-reference.md` §6). A small modal card.
fn go_to(shell: &mut Shell, ctx: &Context) {
    let mut open = shell.go_to.open;
    egui::Window::new("Go to line")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_min_width(240.0);
            ui.horizontal(|ui| {
                ui.label("Line number");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut shell.go_to.line)
                        .id(egui::Id::new("go_to_line"))
                        .desired_width(90.0),
                );
                a11y::label(&response, WidgetType::TextEdit, "Line number");
                if shell.go_to.focus {
                    response.request_focus();
                    shell.go_to.focus = false;
                }
            });

            let valid = nc_textproc::parse_line_number(&shell.go_to.line).is_some();
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(valid, egui::Button::new("Go to")).clicked()
                    || (valid && ui.input(|input| input.key_pressed(Key::Enter)))
                {
                    actions::go_to(shell);
                }
                if ui.button("Cancel").clicked() {
                    shell.go_to.open = false;
                }
            });
        });

    if !open || ctx.input(|input| input.key_pressed(Key::Escape)) {
        shell.go_to.open = false;
    }
}

/// The Open / Save As dialog (`docs/ui-reference.md` §7): one frame in two
/// modes, differing in their title, primary button, and encoding default.
fn file_dialog(shell: &mut Shell, ctx: &Context) {
    let Some(dialog) = shell.file_dialog.as_mut() else {
        return;
    };

    // Copied out first: `shell.file_dialog` is borrowed mutably for the rest of
    // this function, so the palette has to be in hand before that borrow.
    let palette = shell.palette;

    let title = match dialog.mode {
        FileDialogMode::Open => "Open",
        FileDialogMode::SaveAs => "Save as",
    };
    let primary = match dialog.mode {
        FileDialogMode::Open => "Open",
        FileDialogMode::SaveAs => "Save",
    };

    let mut window_open = true;
    let mut confirm = false;
    let mut cancel = false;

    egui::Window::new(title)
        .open(&mut window_open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_min_width(460.0);

            ui.horizontal(|ui| {
                let up = ui.button("‹");
                a11y::label(&up, WidgetType::Button, "Up one level");
                if up.clicked() {
                    dialog.go_up();
                }
                ui.label(dialog.directory.display().to_string());
            });

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut enter: Option<PathBuf> = None;
                    let mut open_file: Option<String> = None;
                    for entry in &dialog.entries {
                        let label = if entry.is_dir {
                            format!("{}/", entry.name)
                        } else {
                            entry.name.clone()
                        };
                        let response = ui.selectable_label(dialog.file_name == entry.name, label);
                        if response.double_clicked() {
                            if entry.is_dir {
                                enter = Some(entry.path.clone());
                            } else {
                                open_file = Some(entry.name.clone());
                            }
                        } else if response.clicked() {
                            if entry.is_dir {
                                enter = Some(entry.path.clone());
                            } else {
                                dialog.file_name = entry.name.clone();
                            }
                        }
                    }
                    if let Some(path) = enter {
                        dialog.enter(&path);
                    }
                    if let Some(name) = open_file {
                        dialog.file_name = name;
                        confirm = true;
                    }
                });

            ui.horizontal(|ui| {
                ui.label("File name");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut dialog.file_name)
                        .id(egui::Id::new("file_name"))
                        .desired_width(300.0),
                );
                a11y::label(&response, WidgetType::TextEdit, "File name");
                if dialog.focus {
                    response.request_focus();
                    dialog.focus = false;
                }
                if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                    confirm = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Encoding");
                let combo = egui::ComboBox::from_id_salt("file_encoding")
                    .selected_text(dialog.encoding.label())
                    .show_ui(ui, |ui| {
                        for choice in dialog.choices() {
                            ui.selectable_value(&mut dialog.encoding, *choice, choice.label());
                        }
                    });
                a11y::label(&combo.response, WidgetType::ComboBox, "Encoding");
            });

            if let Some(error) = &dialog.error {
                ui.label(RichText::new(error).color(palette.danger));
            }

            ui.separator();
            ui.horizontal(|ui| {
                let ready = !dialog.file_name.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new(primary)).clicked() {
                    confirm = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    // `Esc` closes the dialog — unless a prompt is up over it, which takes the
    // key for its own Cancel.
    let escape = shell.prompt.is_none() && ctx.input(|input| input.key_pressed(Key::Escape));
    if cancel || !window_open || escape {
        shell.file_dialog = None;
    } else if confirm {
        actions::confirm_file_dialog(shell);
    }
}

/// What a prompt's buttons do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// Save the document.
    Save,
    /// Discard the edits.
    DontSave,
    /// Replace the existing file.
    Replace,
    /// Write the characters that do not fit as `?`.
    SaveAnyway,
    /// Go back to Save As to pick another encoding.
    ChooseEncoding,
    /// Do nothing.
    Cancel,
}

/// One button on a prompt.
struct PromptButton {
    /// The button's label.
    label: &'static str,
    /// What it means.
    answer: Answer,
}

/// The prompts that guard against data loss (`docs/ui-reference.md` §9). A
/// small centred card, raised before something is destroyed.
fn prompt(shell: &mut Shell, ctx: &Context) {
    let Some(prompt) = shell.prompt.clone() else {
        return;
    };

    let (title, body) = prompt_title_and_body(shell, &prompt);
    let buttons = prompt_buttons(&prompt);

    let mut answer = None;
    let mut window_open = true;

    egui::Window::new(title)
        .open(&mut window_open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_min_width(380.0);
            ui.label(body);
            ui.separator();
            ui.horizontal(|ui| {
                for button in buttons {
                    if ui.button(button.label).clicked() {
                        answer = Some(button.answer);
                    }
                }
            });
        });

    if !window_open || ctx.input(|input| input.key_pressed(Key::Escape)) {
        answer = Some(Answer::Cancel);
    }

    match answer {
        Some(Answer::Save) => actions::answer_unsaved_save(shell),
        Some(Answer::DontSave) => actions::answer_unsaved_dont_save(shell),
        Some(Answer::Replace) => actions::answer_overwrite_replace(shell),
        Some(Answer::SaveAnyway) => actions::answer_unrepresentable_save(shell),
        Some(Answer::ChooseEncoding) => actions::answer_unrepresentable_choose_encoding(shell),
        Some(Answer::Cancel) => actions::answer_cancel(shell),
        None => {}
    }
}

/// A prompt's title and the sentence it asks.
fn prompt_title_and_body(shell: &Shell, prompt: &Prompt) -> (&'static str, String) {
    match prompt {
        Prompt::UnsavedChanges { tab } => {
            let name = shell
                .tabs
                .get(*tab)
                .map(|tab| tab.title())
                .unwrap_or_else(|| "Untitled".to_owned());
            ("Unsaved changes", format!("Save changes to \"{name}\"?"))
        }
        Prompt::Overwrite { path, .. } => {
            let name = file_name(path);
            (
                "Confirm save as",
                format!("\"{name}\" already exists. Do you want to replace it?"),
            )
        }
        Prompt::Unrepresentable {
            encoding,
            characters,
            ..
        } => (
            "Characters cannot be saved",
            format!(
                "{} characters cannot be saved in {encoding}. Saving will replace them with \"?\". Save anyway?",
                characters.len()
            ),
        ),
    }
}

/// A prompt's buttons, left to right.
fn prompt_buttons(prompt: &Prompt) -> &'static [PromptButton] {
    const UNSAVED: &[PromptButton] = &[
        PromptButton {
            label: "Save",
            answer: Answer::Save,
        },
        PromptButton {
            label: "Don't save",
            answer: Answer::DontSave,
        },
        PromptButton {
            label: "Cancel",
            answer: Answer::Cancel,
        },
    ];
    const OVERWRITE: &[PromptButton] = &[
        PromptButton {
            label: "Replace",
            answer: Answer::Replace,
        },
        PromptButton {
            label: "Cancel",
            answer: Answer::Cancel,
        },
    ];
    const UNREPRESENTABLE: &[PromptButton] = &[
        PromptButton {
            label: "Save anyway",
            answer: Answer::SaveAnyway,
        },
        PromptButton {
            label: "Choose another encoding",
            answer: Answer::ChooseEncoding,
        },
        PromptButton {
            label: "Cancel",
            answer: Answer::Cancel,
        },
    ];

    match prompt {
        Prompt::UnsavedChanges { .. } => UNSAVED,
        Prompt::Overwrite { .. } => OVERWRITE,
        Prompt::Unrepresentable { .. } => UNREPRESENTABLE,
    }
}

/// The last component of a path, for a prompt that names a file.
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
