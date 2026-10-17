//! What a command does when it fires.
//!
//! The registry says what commands exist and whether they are available; this
//! module is the one place that turns a fired command into a change in the
//! shell. Commands that belong to a later milestone are present in the menu but
//! have no effect yet, and are simply ignored here.
//!
//! The Find / Replace and Go to operations come from `nc-textproc::search`;
//! this module only moves the caret, applies the replacement, and spells the
//! notices (`docs/ui-reference.md` §5–§6).
//!
//! Markdown formatting is decided in `nc-markdown`; this module supplies the text
//! and the selection, then applies whatever comes back as one document edit — so
//! a format costs exactly one undo step (`NC-M010`).

use std::ops::Range;
use std::path::Path;

use egui::Context;

use crate::clock;
use crate::ui::editor;
use crate::ui::state::{
    DocKind, FileDialog, FileDialogMode, OpenBehavior, ParagraphStyle, PendingReplace, Prompt,
    SaveThen, Shell, SpellDialog, Tab,
};
use nc_commands::CommandId;
use nc_core::Clipboard;
use nc_markdown::{Heading, Inline, ListKind, Outcome};
use nc_textproc::search::find;
use nc_textproc::{Direction, Encoding, SearchResult, TimeDate, find_all, go_to_line};

/// Runs the command with the given id against the shell.
pub fn run(shell: &mut Shell, ctx: &Context, id: CommandId) {
    match id.as_str() {
        "cmd.file.new_tab" => shell.open_tab(DocKind::Plain),
        "cmd.file.new_window" => shell.new_window_request = true,
        "cmd.file.new_markdown" => shell.open_tab(DocKind::Markdown),
        "cmd.file.open" => open_file_dialog(shell),
        "cmd.file.save" => save(shell),
        "cmd.file.save_as" => save_as_dialog(shell),
        "cmd.file.save_all" => save_all(shell),
        "cmd.file.close_tab" => close_tab(shell, shell.active),
        "cmd.file.close_window" | "cmd.file.exit" => close_window(shell),
        "cmd.file.page_setup" => shell.page_setup_dialog.open(&shell.page_setup),
        "cmd.file.print" => print(shell),
        "cmd.tab.next" => next_tab(shell),
        "cmd.tab.previous" => previous_tab(shell),
        "cmd.edit.undo" => undo(shell),
        "cmd.edit.redo" => redo(shell),
        "cmd.edit.select_all" => select_all(shell),
        "cmd.edit.delete" => delete_selection(shell),
        "cmd.edit.cut" => cut(shell, ctx),
        "cmd.edit.copy" => copy(shell, ctx),
        "cmd.edit.paste" => request_paste(ctx),
        "cmd.edit.time_date" => insert_time_date(shell, clock::now_utc()),
        "cmd.view.status_bar" => shell.status_bar_visible = !shell.status_bar_visible,
        "cmd.view.word_wrap" => shell.word_wrap = !shell.word_wrap,
        "cmd.view.zoom_in" => shell.zoom = (shell.zoom + 10).min(500),
        "cmd.view.zoom_out" => shell.zoom = shell.zoom.saturating_sub(10).max(10),
        "cmd.view.zoom_reset" => shell.zoom = 100,
        "cmd.view.md_source" => back_to_source(shell),
        "cmd.view.md_formatted" => {
            // Guarded by kind rather than by availability alone: the flag means
            // "this document is being rendered", and a plain document has no
            // Markdown to render.
            if let Some(tab) = shell
                .active_tab_mut()
                .filter(|tab| tab.kind == DocKind::Markdown)
            {
                tab.formatted = true;
            }
        }
        "cmd.app.settings" => shell.show_settings = !shell.show_settings,
        "cmd.edit.find" => open_find(shell, false),
        "cmd.edit.replace" => open_find(shell, true),
        "cmd.edit.find_next" => search_step(shell, Direction::Forward),
        "cmd.edit.find_prev" => search_step(shell, Direction::Backward),
        "cmd.edit.go_to" => open_go_to(shell),
        "cmd.edit.spell_suggest" => open_spelling(shell),
        // --- Markdown formatting (M7) -----------------------------------
        // The engine decides the bytes; `apply_format` writes them.
        "cmd.md.bold" => format_inline(shell, Inline::Bold),
        "cmd.md.italic" => format_inline(shell, Inline::Italic),
        "cmd.md.strike" => format_inline(shell, Inline::Strike),
        "cmd.md.link" => apply_format(shell, |text, selection| {
            // The URL prompt is not built yet, so the command writes the syntax
            // and leaves the caret between the parentheses to type into.
            nc_markdown::insert_link(text, selection, "")
        }),
        "cmd.md.table" => apply_format(shell, |text, selection| {
            // One command, both halves of NC-M008: inside a table it grows a row,
            // anywhere else it writes a new one. There is no size or direction
            // dialog in the capture and no second command in the registry, so the
            // document decides which job this keystroke is doing.
            nc_markdown::expand_table(text, selection)
                .or_else(|| nc_markdown::insert_table(text, selection, 3, 2))
        }),
        "cmd.md.clear" => apply_format(shell, nc_markdown::clear_formatting),
        "cmd.md.bullets" => format_list(shell, ListKind::Bullet),
        "cmd.md.numbering" => format_list(shell, ListKind::Numbered),
        "cmd.md.indent_increase" => apply_format(shell, nc_markdown::indent_lines),
        "cmd.md.indent_decrease" => apply_format(shell, nc_markdown::outdent_lines),
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

    // A window close asked for by a command is acted on at the end of the frame
    // the same way one asked for by the title bar is.
    pump(shell, ctx);
}

/// Whether this build implements `id`.
///
/// A command can exist in the registry while its effect is scheduled for a
/// later milestone; [`crate::ui::input`] consults this so a key that names an
/// inert command is not consumed for nothing (`AGENT.md` §2). Kept next to the
/// `match` in [`run`] so the two read together; `every_landed_command_is_wired`
/// guards that no landed command is left unimplemented.
pub fn handles(id: CommandId) -> bool {
    matches!(
        id.as_str(),
        "cmd.file.new_tab"
            | "cmd.file.new_window"
            | "cmd.file.new_markdown"
            | "cmd.file.open"
            | "cmd.file.save"
            | "cmd.file.save_as"
            | "cmd.file.save_all"
            | "cmd.file.close_tab"
            | "cmd.file.close_window"
            | "cmd.file.exit"
            | "cmd.file.page_setup"
            | "cmd.file.print"
            | "cmd.tab.next"
            | "cmd.tab.previous"
            | "cmd.edit.undo"
            | "cmd.edit.redo"
            | "cmd.edit.select_all"
            | "cmd.edit.delete"
            | "cmd.edit.cut"
            | "cmd.edit.copy"
            | "cmd.edit.paste"
            | "cmd.edit.time_date"
            | "cmd.edit.find"
            | "cmd.edit.find_next"
            | "cmd.edit.find_prev"
            | "cmd.edit.replace"
            | "cmd.edit.go_to"
            | "cmd.edit.spell_suggest"
            | "cmd.view.status_bar"
            | "cmd.view.word_wrap"
            | "cmd.view.zoom_in"
            | "cmd.view.zoom_out"
            | "cmd.view.zoom_reset"
            | "cmd.view.md_source"
            | "cmd.view.md_formatted"
            | "cmd.app.settings"
            | "cmd.format.title"
            | "cmd.format.subtitle"
            | "cmd.format.heading"
            | "cmd.format.subheading"
            | "cmd.format.section"
            | "cmd.format.subsection"
            | "cmd.format.body"
            | "cmd.md.bold"
            | "cmd.md.italic"
            | "cmd.md.strike"
            | "cmd.md.link"
            | "cmd.md.table"
            | "cmd.md.clear"
            | "cmd.md.bullets"
            | "cmd.md.numbering"
            | "cmd.md.indent_increase"
            | "cmd.md.indent_decrease"
    )
}

/// The highest roadmap milestone whose commands this build implements. Bumped
/// as a milestone lands; `every_landed_command_is_wired` holds the two in step.
///
/// M7 is the reason this constant measures more than dispatch. When every
/// milestone-7 command had an arm in [`run`] but `cmd.view.md_formatted` only set a
/// flag the editor never read, raising it would have had the guard certify a
/// decorative command — a test that counts arms cannot tell a wired command from an
/// inert one. It waits for the effect to exist. Now that [`crate::ui::markdown_view`]
/// renders the document from its own Markdown, the toggle changes what is on screen
/// and 7 is honest.
///
/// M8 raises it to 8. Its commands all have a real effect now, not just an arm:
/// `cmd.file.print` renders the active document to the page §8 describes and hands the
/// PostScript to `lpr`, `cmd.file.page_setup` opens that dialog, and the settings and
/// spell-suggest commands were already live. `cmd.edit.define_bing` (M9) stays `Never`
/// and exempt — there is no spell-*define* effect to raise a test onto yet.
#[cfg(test)]
pub const LANDED_MILESTONE: u8 = 8;

/// Undoes the last edit in the active document (`Ctrl+Z`).
pub fn undo(shell: &mut Shell) {
    if let Some(tab) = shell.active_tab_mut() {
        tab.document_mut().undo();
    }
    sync_caret_to_document(shell);
}

/// Redoes the last undone edit (`Ctrl+Y`).
pub fn redo(shell: &mut Shell) {
    if let Some(tab) = shell.active_tab_mut() {
        tab.document_mut().redo();
    }
    sync_caret_to_document(shell);
}

/// Selects the whole active document (`Ctrl+A`).
pub fn select_all(shell: &mut Shell) {
    let Some(tab) = shell.active_tab_mut() else {
        return;
    };
    tab.document_mut().select_all();
    let count = tab.text().chars().count();
    shell.caret = count;
    // The primary end is where a following search or keystroke travels from, so
    // it starts at the end of the document with the anchor back at the start.
    shell.pending_selection = Some((count, 0));
}

/// Deletes the active document's selection (`Del`/`Delete`).
pub fn delete_selection(shell: &mut Shell) {
    if let Some(tab) = shell.active_tab_mut() {
        tab.document_mut().delete_selection();
    }
    sync_caret_to_document(shell);
}

/// Copies the selection to the clipboard (`Ctrl+C`).
pub fn copy(shell: &mut Shell, ctx: &Context) {
    if let Some(tab) = shell.active_tab() {
        let mut clipboard = EguiClipboard(ctx);
        tab.file.document().copy_to(&mut clipboard);
    }
}

/// Copies the selection and deletes it (`Ctrl+X`).
pub fn cut(shell: &mut Shell, ctx: &Context) {
    let cut = {
        let Some(tab) = shell.active_tab_mut() else {
            return;
        };
        let mut clipboard = EguiClipboard(ctx);
        tab.document_mut().cut_to(&mut clipboard)
    };
    if cut {
        sync_caret_to_document(shell);
    }
}

/// Asks the windowing layer for the clipboard's text (`Ctrl+V`, menu Paste).
///
/// The text does not come back synchronously: the backend answers with an
/// `egui::Event::Paste` on the next frame, which the editor widget inserts
/// through the same document (`crate::ui::input`).
pub fn request_paste(ctx: &Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
}

/// Inserts the time and date F5 writes, as one undoable edit.
pub fn insert_time_date(shell: &mut Shell, moment: TimeDate) {
    if let Some(tab) = shell.active_tab_mut() {
        tab.document_mut()
            .insert_uncoalesced(&moment.time_and_date());
    }
    sync_caret_to_document(shell);
}

/// The sheet body is printed at, in points.
///
/// §8 collects the *sheet* — size, orientation, margins, header — and says nothing
/// about a type size, so the page is set at the classic typewriter ten points. The
/// on-screen zoom is a reading aid and is deliberately not carried onto paper.
const PRINT_POINT_SIZE: f32 = 10.0;

/// `File ▸ Print`, and the Page-setup dialog's `Print`: renders the active document
/// with the page setup as it stands and hands the job to the system print pipeline.
///
/// §8 has `File ▸ Print` print with the current settings *without* opening the dialog,
/// which is what this does. The outcome — sheets sent, or the reason it could not —
/// is reported in the status bar rather than left silent.
pub fn print(shell: &mut Shell) {
    let Some(tab) = shell.active_tab() else {
        return;
    };
    let text = tab.text();
    let moment = clock::now_utc();
    let date = format!("{}-{:02}-{:02}", moment.year, moment.month, moment.day);
    let job = nc_textproc::print_document(text, &shell.page_setup, PRINT_POINT_SIZE, &date);
    let sheets = job.matches("showpage").count();

    match send_to_printer(&job) {
        Ok(()) => {
            shell.notification = Some(format!(
                "Sent {sheets} page{} to the printer",
                if sheets == 1 { "" } else { "s" }
            ));
        }
        Err(error) => {
            shell.notification = Some(format!("Could not print: {error}"));
        }
    }
}

/// Hands a PostScript job to the system print pipeline, feeding it on standard input.
///
/// This is the single step that reaches outside the process, and the one the headless
/// tests deliberately do not drive: a print run needs a queue and a printer, neither of
/// which CI has. With none configured the spawn simply fails, and `print` surfaces that
/// rather than pretending the page went out.
fn send_to_printer(job: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("lpr").stdin(Stdio::piped()).spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(job.as_bytes())?;
    }
    child.wait()?;
    Ok(())
}

/// Points the queued editor caret at the active document's own caret, after a
/// shell command moved it.
fn sync_caret_to_document(shell: &mut Shell) {
    let Some(tab) = shell.active_tab() else {
        return;
    };
    let caret = char_index(tab.text(), tab.file.document().caret());
    shell.caret = caret;
    shell.pending_selection = Some((caret, caret));
}

/// The clipboard the shell copies through, backed by `egui`.
///
/// `get_text` is never consulted: paste is driven by the backend's paste event,
/// not read back through this trait.
struct EguiClipboard<'a>(&'a Context);

impl Clipboard for EguiClipboard<'_> {
    fn get_text(&self) -> Option<String> {
        None
    }

    fn set_text(&mut self, text: String) {
        self.0.copy_text(text);
    }
}

/// Opens the Find / Replace dialog, in Find or Replace mode.
pub fn open_find(shell: &mut Shell, replace: bool) {
    shell.find.open = true;
    shell.find.replace = replace;
    shell.find.focus = true;
}

/// Opens the Go to line dialog with the caret's current line pre-filled.
pub fn open_go_to(shell: &mut Shell) {
    shell.go_to.line = shell.line.to_string();
    shell.go_to.open = true;
    shell.go_to.focus = true;
}

/// Opens the spelling dialog for the misspelled word at the caret
/// (`Edit ▸ Spelling suggestions`, F7, `NC-P004`).
///
/// It is a report as often as a correction: with Spell check off, or the caret on
/// a clean word, or a slip the small dictionary has no near neighbour for, the
/// command says so in the status bar rather than opening an empty box.
pub fn open_spelling(shell: &mut Shell) {
    let index = shell.active;
    if !shell.spell_check {
        shell.notification = Some("Turn on Settings ▸ Spell check first".to_owned());
        return;
    }
    let Some(tab) = shell.tabs.get(index) else {
        return;
    };
    let text = tab.text();
    let caret_byte = char_to_byte(text, shell.caret);
    let Some(range) = nc_textproc::spell::misspelled_word(text, caret_byte) else {
        shell.notification = Some("No misspelled word at the caret".to_owned());
        return;
    };
    let word = text[range.clone()].to_owned();
    let suggestions = nc_textproc::spell::suggestions(&word, 8);
    if suggestions.is_empty() {
        shell.notification = Some(format!("No suggestions for \"{word}\""));
        return;
    }
    shell.spelling = SpellDialog {
        open: true,
        tab: index,
        word,
        range,
        suggestions,
    };
}

/// Replaces the misspelled word with the `chosen` suggestion and closes the
/// dialog (`NC-P004`). The correction keeps the original's capitalisation, so
/// fixing `Teh` at a sentence's start does not quietly make it `the`.
pub fn apply_spelling(shell: &mut Shell, chosen: usize) {
    let tab = shell.spelling.tab;
    let Some(replacement) = shell.spelling.suggestions.get(chosen).cloned() else {
        return;
    };
    let original = shell.spelling.word.clone();
    let range = shell.spelling.range.clone();
    // The document may have moved while the dialog was open — it is not modal, so a
    // keystroke can reach it. Replace only while the bytes at the remembered range
    // are still the very word the dialog was about, so a stale range overwrites
    // nothing and the dialog simply steps aside.
    let still_the_same_word = shell
        .tabs
        .get(tab)
        .is_some_and(|tab| tab.text().get(range.clone()) == Some(original.as_str()));
    if !still_the_same_word {
        shell.spelling = SpellDialog::new();
        return;
    }
    let replacement = match_capitalisation(&original, &replacement);
    if let Some(tab) = shell.tabs.get_mut(tab) {
        tab.document_mut().replace_range(range, &replacement);
        if shell.active == shell.spelling.tab {
            sync_caret_to_document(shell);
        }
    }
    shell.spelling = SpellDialog::new();
}

/// Closes the spelling dialog without changing the document (`Esc`, the close box).
pub fn close_spelling(shell: &mut Shell) {
    shell.spelling = SpellDialog::new();
}

/// Gives `replacement` the leading capitalisation of `original`: an all-caps
/// original gets an all-caps replacement, a leading-capital original gets a
/// leading-capital replacement, and anything else is taken as written.
fn match_capitalisation(original: &str, replacement: &str) -> String {
    let mut letters = original.chars().filter(|c| c.is_alphabetic());
    let first_upper = letters.next().is_some_and(char::is_uppercase);
    let all_upper = original
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(char::is_uppercase);
    if all_upper && !replacement.is_empty() {
        replacement.to_uppercase()
    } else if first_upper {
        let mut chars = replacement.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => replacement.to_owned(),
        }
    } else {
        replacement.to_owned()
    }
}

/// Opens the Open dialog (`File ▸ Open`, `Ctrl+O`).
pub fn open_file_dialog(shell: &mut Shell) {
    shell.file_dialog = Some(FileDialog::open());
}

/// Saves the active document (`File ▸ Save`, `Ctrl+S`).
pub fn save(shell: &mut Shell) {
    save_tab(shell, shell.active, SaveThen::Keep);
}

/// Opens Save As for the active document (`File ▸ Save as`, `Ctrl+Shift+S`).
pub fn save_as_dialog(shell: &mut Shell) {
    open_save_as(shell, shell.active);
}

/// Saves every tab with unsaved changes (`File ▸ Save all`, `Ctrl+Alt+S`).
///
/// A tab that has no path yet opens Save As instead (there is nowhere to write),
/// and a prompt or dialog raised by one tab stops the run: the remaining tabs
/// keep their changes and are saved by a later Save all.
pub fn save_all(shell: &mut Shell) {
    let dirty: Vec<usize> = shell
        .tabs
        .iter()
        .enumerate()
        .filter(|(_, tab)| tab.is_dirty())
        .map(|(index, _)| index)
        .collect();

    for tab in dirty {
        // Saving is interrupted while a prompt or a dialog is waiting.
        if shell.prompt.is_some() || shell.file_dialog.is_some() {
            break;
        }
        save_tab(shell, tab, SaveThen::Keep);
    }
}

/// Opens Save As for `tab`, named after the file it already has, or blank.
fn open_save_as(shell: &mut Shell, tab: usize) {
    let Some(document) = shell.tabs.get(tab) else {
        return;
    };
    let name = if document.file.has_path() {
        document.title()
    } else {
        String::new()
    };
    let encoding = document.file.encoding();
    let bom = document.file.has_bom();
    shell.file_dialog = Some(FileDialog::save_as(tab, name, encoding, bom));
}

/// Saves `tab`, and carries out `then` once the write succeeds — closing the tab
/// for the unsaved-changes prompt, or giving it up to the file being opened over
/// it (`docs/ui-reference.md` §9, `Settings ▸ Opening files`).
///
/// The continuation is recorded before any detour, because the write does not
/// always happen in this call: a document with no path opens Save As, and a text
/// the encoding cannot hold opens the encoding question. Whichever of them
/// finishes the write is what finishes this, so the intent has to outlive them.
pub fn save_tab(shell: &mut Shell, tab: usize, then: SaveThen) {
    let Some(has_path) = shell.tabs.get(tab).map(|tab| tab.file.has_path()) else {
        return;
    };
    if then != SaveThen::Keep {
        shell.save_then = Some((tab, then));
    }
    if !has_path {
        // There is nowhere to write yet, so Save becomes Save As; what the save
        // was for resumes when that write succeeds.
        open_save_as(shell, tab);
        return;
    }

    // Characters the file's own encoding cannot hold are asked about before
    // anything is written (NC-D003).
    let unrepresentable =
        shell
            .tabs
            .get(tab)
            .and_then(|tab| match nc_textproc::prompt::before_saving(&tab.file) {
                Some(nc_textproc::prompt::Question::Unrepresentable {
                    encoding,
                    characters,
                }) => Some((encoding, characters)),
                _ => None,
            });
    if let Some((encoding, characters)) = unrepresentable {
        shell.prompt = Some(Prompt::Unrepresentable {
            tab,
            encoding,
            characters,
        });
        return;
    }

    let name = shell
        .tabs
        .get(tab)
        .map(|tab| tab.title())
        .unwrap_or_default();
    match shell.tabs[tab].file.save() {
        Ok(_) => finish_save(shell, tab),
        Err(_) => {
            // The write failed, so nothing can follow it: the close or the
            // replacement is dropped rather than waiting for a later Save of this
            // tab, which is a new request with its own intent.
            if shell.save_then.is_some_and(|(saved, _)| saved == tab) {
                shell.save_then = None;
            }
            shell.notification = Some(format!("Could not save \"{name}\""));
        }
    }
}

/// Carries out what a just-written save was for ([`SaveThen`]).
fn finish_save(shell: &mut Shell, tab: usize) {
    let Some((saved, then)) = shell.save_then.take() else {
        return;
    };
    if saved != tab {
        // The step belongs to another tab's save; leave it for that one.
        shell.save_then = Some((saved, then));
        return;
    }
    match then {
        SaveThen::Keep => {}
        SaveThen::Close => {
            shell.close_tab(tab);
            if shell.closing_window {
                advance_close(shell);
            }
        }
        SaveThen::Replace => {
            // The document that was waiting for this write takes the tab over,
            // and the caret starts where a just-opened document's does.
            if let Some(replacement) = shell.pending_replace.take() {
                shell.replace_tab(replacement.tab, replacement.opening);
                sync_caret_to_document(shell);
            }
        }
    }
}

/// Closes a tab, raising the unsaved-changes prompt when it is dirty
/// (`docs/ui-reference.md` §9, NC-D005). The last tab is replaced by a fresh
/// document rather than leaving the window with none.
pub fn close_tab(shell: &mut Shell, index: usize) {
    if shell
        .tabs
        .get(index)
        .map(|tab| tab.is_dirty())
        .unwrap_or(false)
    {
        shell.prompt = Some(Prompt::UnsavedChanges { tab: index });
        return;
    }
    shell.close_tab(index);
}

/// Asks the window to close (`File ▸ Exit`, the title bar's ✕), prompting for
/// each dirty document first.
pub fn close_window(shell: &mut Shell) {
    shell.closing_window = true;
    advance_close(shell);
}

/// Prompts for the next dirty document, or asks the window to close when none
/// is left.
fn advance_close(shell: &mut Shell) {
    match shell.tabs.iter().position(|tab| tab.is_dirty()) {
        Some(tab) => shell.prompt = Some(Prompt::UnsavedChanges { tab }),
        None => {
            shell.closing_window = false;
            shell.exit_pending = true;
        }
    }
}

/// Switches to the next tab, wrapping from the last to the first
/// (`Ctrl+Tab`, NC-T002).
pub fn next_tab(shell: &mut Shell) {
    switch_tab(shell, 1);
}

/// Switches to the previous tab, wrapping from the first to the last
/// (`Ctrl+Shift+Tab`, NC-T002).
pub fn previous_tab(shell: &mut Shell) {
    switch_tab(shell, -1);
}

/// Moves the active tab by `step`, cyclically. The caret is re-pointed at the
/// tab that becomes active, so the editor's own widget state follows the
/// document the way a tab-strip click's does not have to.
fn switch_tab(shell: &mut Shell, step: isize) {
    let count = shell.tabs.len();
    if count < 2 {
        return;
    }
    let offset = step.rem_euclid(count as isize) as usize;
    shell.active = (shell.active + offset) % count;
    sync_caret_to_document(shell);
}

/// Acts on a request that outlives the input event, once per frame: a window
/// close asked for by the window manager, and one queued by [`close_window`].
pub fn pump(shell: &mut Shell, ctx: &Context) {
    // A close the window manager asked for is intercepted while a document is
    // dirty, so it can be asked about (NC-D005). A clean window closes normally.
    let closing = ctx.input(|input| input.viewport().close_requested());
    if closing && !shell.closing_window && shell.tabs.iter().any(|tab| tab.is_dirty()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        close_window(shell);
        return;
    }

    // A window that has just been given focus looks for files another program
    // may have written while it was in the background (NC-D008). Only a gain is
    // checked, so a window left in front does no disk reads every frame.
    let focused = ctx.input(|input| input.focused);
    if focused && !shell.window_focused {
        check_outside_changes(shell);
    }
    shell.window_focused = focused;

    if std::mem::take(&mut shell.exit_pending) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

/// Raises the outside-change prompt for the first document whose file changed on
/// disk since NoteCraft read it (`NC-D008`, `docs/ui-reference.md` §9).
///
/// The question is asked when the window gains focus, because that is when a
/// program that had the foreground has finished writing. It never talks over a
/// question or a dialog that is already up: one prompt at a time, and a Save As
/// in progress is not something to interrupt.
pub fn check_outside_changes(shell: &mut Shell) {
    if shell.prompt.is_some() || shell.file_dialog.is_some() || shell.closing_window {
        return;
    }
    let changed = shell.tabs.iter().enumerate().find_map(|(tab, document)| {
        match nc_textproc::prompt::after_outside_change(&document.file) {
            Some(nc_textproc::prompt::Question::OutsideChange { path }) => Some((tab, path)),
            _ => None,
        }
    });
    if let Some((tab, path)) = changed {
        shell.prompt = Some(Prompt::OutsideChange { tab, path });
    }
}

/// Confirms the file dialog: opens the named file, or saves the tab to it.
pub fn confirm_file_dialog(shell: &mut Shell) {
    let Some(dialog) = shell.file_dialog.as_ref() else {
        return;
    };
    if dialog.file_name.trim().is_empty() {
        return;
    }
    let mode = dialog.mode;
    let tab = dialog.tab;
    let target = dialog.target();
    let encoding = dialog.encoding.chosen();

    match mode {
        FileDialogMode::Open => open_path(shell, &target, encoding),
        FileDialogMode::SaveAs => {
            let (encoding, bom) = encoding.unwrap_or((Encoding::Utf8, false));
            if let Some(nc_textproc::prompt::Question::Overwrite { path }) =
                nc_textproc::prompt::before_overwriting(&target)
            {
                shell.prompt = Some(Prompt::Overwrite {
                    path,
                    tab,
                    encoding,
                    bom,
                });
                return;
            }
            write_save_as(shell, tab, &target, encoding, bom);
        }
    }
}

/// Opens `path`, where `Settings ▸ Opening files` says it should go, or reports
/// why it could not be opened.
///
/// A path already open — in any spelling, compared canonically — focuses the
/// tab that shows it rather than opening the file a second time (`NC-F012`).
/// That comes first, whatever the setting says: a file is never open twice, so
/// *Open in a new tab* and the rest have nothing to do when the document is
/// already on screen (`docs/ui-reference.md` §4).
///
/// The file is read here, in the window the user used, so a file that cannot be
/// read is reported here whatever happens next.
///
/// Reachable from the Recent submenu as well as the Open dialog, so both go
/// through one path (`NC-F004`, `NC-F012`).
pub(crate) fn open_path(shell: &mut Shell, path: &Path, encoding: Option<(Encoding, bool)>) {
    // A file is recent as soon as it is asked for, whether or not the read below
    // succeeds: the entry names what the user wanted, and a document that has
    // gone away is still something they opened.
    shell.remember_recent(path);

    if let Some(existing) = shell.tabs.iter().position(|tab| tab.matches_path(path)) {
        shell.active = existing;
        shell.file_dialog = None;
        shell.notification = None;
        return;
    }

    let opened = match encoding {
        Some((encoding, bom)) => Tab::open_as(path, encoding, bom),
        None => Tab::open(path),
    };
    match opened {
        Ok(tab) => {
            shell.file_dialog = None;
            shell.notification = None;
            match shell.opening {
                OpenBehavior::NewTab => {
                    shell.tabs.push(tab);
                    shell.active = shell.tabs.len() - 1;
                }
                OpenBehavior::NewWindow => {
                    // The window itself is opened by the eframe application at the
                    // end of this frame, from a request set here (`NC-F003`); the
                    // document travels with it, so the new window shows the file
                    // rather than the empty untitled one a plain New window gets.
                    shell.new_window_documents.push(tab);
                    shell.new_window_request = true;
                }
                OpenBehavior::ReplaceTab => {
                    let target = shell.active;
                    let dirty = shell.tabs.get(target).is_some_and(Tab::is_dirty);
                    if dirty {
                        // The document waits for the answer: replacing the tab
                        // would throw the edits away, and the rule for that is to
                        // ask (NC-D005).
                        shell.pending_replace = Some(PendingReplace {
                            tab: target,
                            opening: tab,
                        });
                        shell.prompt = Some(Prompt::UnsavedChanges { tab: target });
                    } else {
                        shell.replace_tab(target, tab);
                        sync_caret_to_document(shell);
                    }
                }
            }
        }
        Err(error) => {
            if let Some(dialog) = shell.file_dialog.as_mut() {
                dialog.error = Some(open_error(path, &error));
            }
        }
    }
}

/// Why an Open failed, in the dialog's own words.
fn open_error(path: &Path, error: &nc_textproc::Error) -> String {
    match error {
        nc_textproc::Error::Read { .. } => format!("Cannot find \"{}\"", path.display()),
        nc_textproc::Error::Decode { encoding, .. } => {
            format!("The file is not {encoding} text")
        }
        other => other.to_string(),
    }
}

/// Writes `tab` to `path` as `encoding`, reporting a failure in the dialog.
fn write_save_as(shell: &mut Shell, tab: usize, path: &Path, encoding: Encoding, bom: bool) {
    let result = shell
        .tabs
        .get_mut(tab)
        .map(|document| document.file.save_as(path, encoding, bom));

    match result {
        None => shell.file_dialog = None,
        Some(Ok(_)) => {
            shell.file_dialog = None;
            shell.notification = None;
            finish_save(shell, tab);
        }
        Some(Err(nc_textproc::Error::Encode(nc_textproc::EncodeError { characters, .. }))) => {
            if let Some(dialog) = shell.file_dialog.as_mut() {
                dialog.error = Some(format!(
                    "{} characters cannot be saved in {encoding}",
                    characters.len()
                ));
            }
        }
        Some(Err(error)) => {
            if let Some(dialog) = shell.file_dialog.as_mut() {
                dialog.error = Some(format!("Cannot write \"{}\": {error}", path.display()));
            }
        }
    }
}

/// The answer to the unsaved-changes prompt: save the document (`Save`).
pub fn answer_unsaved_save(shell: &mut Shell) {
    let Some(Prompt::UnsavedChanges { tab }) = shell.prompt.take() else {
        return;
    };
    // The prompt may have been raised to close the tab or to let a file being
    // opened take its place; which of them follows the save is what is pending.
    let then = match &shell.pending_replace {
        Some(replacement) if replacement.tab == tab => SaveThen::Replace,
        _ => SaveThen::Close,
    };
    save_tab(shell, tab, then);
}

/// The answer to the unsaved-changes prompt: discard the edits (`Don't save`).
pub fn answer_unsaved_dont_save(shell: &mut Shell) {
    let Some(Prompt::UnsavedChanges { tab }) = shell.prompt.take() else {
        return;
    };
    if shell
        .pending_replace
        .as_ref()
        .is_some_and(|replacement| replacement.tab == tab)
    {
        let replacement = shell
            .pending_replace
            .take()
            .expect("just checked to be there");
        shell.replace_tab(replacement.tab, replacement.opening);
        sync_caret_to_document(shell);
        return;
    }
    shell.close_tab(tab);
    if shell.closing_window {
        advance_close(shell);
    }
}

/// The answer to the overwrite prompt: replace the file (`Replace`).
pub fn answer_overwrite_replace(shell: &mut Shell) {
    let Some(Prompt::Overwrite {
        path,
        tab,
        encoding,
        bom,
    }) = shell.prompt.take()
    else {
        return;
    };
    write_save_as(shell, tab, &path, encoding, bom);
}

/// The answer to the unrepresentable prompt: write the characters as `?`
/// (`Save anyway`).
pub fn answer_unrepresentable_save(shell: &mut Shell) {
    let Some(Prompt::Unrepresentable { tab, .. }) = shell.prompt.take() else {
        return;
    };
    let name = shell
        .tabs
        .get(tab)
        .map(|document| document.title())
        .unwrap_or_default();
    if shell.tabs[tab].file.save_lossy().is_ok() {
        finish_save(shell, tab);
    } else {
        shell.notification = Some(format!("Could not save \"{name}\""));
    }
}

/// The answer to the unrepresentable prompt: go back to Save As to pick another
/// encoding (`Choose another encoding`).
pub fn answer_unrepresentable_choose_encoding(shell: &mut Shell) {
    let Some(Prompt::Unrepresentable { tab, .. }) = shell.prompt.take() else {
        return;
    };
    open_save_as(shell, tab);
}

/// The answer to the outside-change prompt: read the file again (`Reload`),
/// discarding the editor's text and its history (`NC-D008`).
pub fn answer_outside_change_reload(shell: &mut Shell) {
    let Some(Prompt::OutsideChange { tab, .. }) = shell.prompt.take() else {
        return;
    };
    let Some(document) = shell.tabs.get_mut(tab) else {
        return;
    };
    let name = document.title();
    match document.file.reload() {
        Ok(()) => {
            // The document that was reloaded is a new one, caret at the start.
            if shell.active == tab {
                sync_caret_to_document(shell);
            }
            // Another document may have changed while this one was being read.
            check_outside_changes(shell);
        }
        Err(_) => shell.notification = Some(format!("Could not reload \"{name}\"")),
    }
}

/// The answer to the outside-change prompt: keep the text the editor holds
/// (`Keep my version`), marking it dirty against the file now on disk so the
/// next save writes it (`NC-D008`).
pub fn answer_outside_change_keep(shell: &mut Shell) {
    let Some(Prompt::OutsideChange { tab, .. }) = shell.prompt.take() else {
        return;
    };
    let Some(document) = shell.tabs.get_mut(tab) else {
        return;
    };
    let name = document.title();
    match document.file.keep_local_version() {
        Ok(()) => check_outside_changes(shell),
        Err(_) => shell.notification = Some(format!("Could not read \"{name}\"")),
    }
}

/// The answer to any prompt that has a `Cancel`: do nothing.
///
/// Nothing includes the step that was to follow the answer: a save that has been
/// cancelled cannot close a tab or hand it to the file being opened over it.
pub fn answer_cancel(shell: &mut Shell) {
    shell.prompt = None;
    shell.closing_window = false;
    shell.save_then = None;
    shell.pending_replace = None;
}

/// Steps the search in `direction` from the caret, keeping the options and the
/// wrap choice the dialog holds. This is what `Find Next`, `F3` and
/// `Shift+F3` run.
pub fn search_step(shell: &mut Shell, direction: Direction) {
    let needle = shell.find.query.clone();
    if needle.is_empty() {
        return;
    }
    let options = shell.find.options;
    let wraps = shell.find.wraps;
    let Some(tab) = shell.active_tab() else {
        return;
    };
    let text = tab.text().to_owned();
    let from = char_to_byte(&text, shell.caret);
    let result = find(&text, &needle, from, direction, options, wraps);

    match result {
        SearchResult::Found(found) | SearchResult::Wrapped(found) => {
            let start = char_index(&text, found.start);
            let end = char_index(&text, found.end);
            // `egui`'s primary end is the one a following search travels from,
            // so travel out of the match: past it forwards, before it
            // backwards.
            let (primary, secondary) = match direction {
                Direction::Forward => (end, start),
                Direction::Backward => (start, end),
            };
            shell.caret = primary;
            shell.pending_selection = Some((primary, secondary));
            shell.notification = match result {
                SearchResult::Wrapped(_) => Some(wrapped_notice(direction)),
                _ => None,
            };
        }
        SearchResult::Boundary => shell.notification = Some(boundary_notice(direction)),
        SearchResult::NoMatch => shell.notification = Some(not_found_notice(&needle)),
    }
}

/// Replaces the match at or after the caret and moves the caret past the
/// replacement. This is the dialog's `Replace` button.
pub fn replace(shell: &mut Shell) {
    let needle = shell.find.query.clone();
    if needle.is_empty() {
        return;
    }
    let replacement = shell.find.replacement.clone();
    let options = shell.find.options;
    let Some(tab) = shell.active_tab() else {
        return;
    };
    let text = tab.text().to_owned();
    let from = char_to_byte(&text, shell.caret);
    let result = find(
        &text,
        &needle,
        from,
        Direction::Forward,
        options,
        shell.find.wraps,
    );
    let Some(found) = result.matched() else {
        shell.notification = Some(match result {
            SearchResult::NoMatch => not_found_notice(&needle),
            _ => boundary_notice(Direction::Forward),
        });
        return;
    };

    let caret = char_index(&text, found.start) + replacement.chars().count();
    let tab = shell.active_tab_mut().expect("checked above");
    tab.document_mut()
        .replace_range(found.range(), &replacement);

    shell.caret = caret;
    shell.pending_selection = Some((caret, caret));
    shell.notification = None;
}

/// Replaces every match, left to right, in one pass. This is the dialog's
/// `Replace All` button.
pub fn replace_all(shell: &mut Shell) {
    let needle = shell.find.query.clone();
    if needle.is_empty() {
        return;
    }
    let replacement = shell.find.replacement.clone();
    let options = shell.find.options;
    let Some(tab) = shell.active_tab() else {
        return;
    };
    let text = tab.text().to_owned();
    let matches = find_all(&text, &needle, options);
    if matches.is_empty() {
        shell.notification = Some(not_found_notice(&needle));
        return;
    }

    let replacements: Vec<(Range<usize>, &str)> = matches
        .iter()
        .map(|found| (found.range(), replacement.as_str()))
        .collect();

    let tab = shell.active_tab_mut().expect("checked above");
    // One call, so the whole run is a single undo step (`NC-S003`).
    tab.document_mut().replace_ranges(&replacements);

    shell.caret = 0;
    shell.pending_selection = Some((0, 0));
    shell.notification = None;
}

/// Moves the caret to the start of the line named in the Go to dialog and
/// closes it. The line number clamps (`nc-textproc::go_to_line`).
pub fn go_to(shell: &mut Shell) {
    let Some(line) = nc_textproc::parse_line_number(&shell.go_to.line) else {
        return;
    };
    let Some(tab) = shell.active_tab() else {
        return;
    };
    let text = tab.text().to_owned();
    let caret = char_index(&text, go_to_line(&text, line));
    shell.caret = caret;
    shell.pending_selection = Some((caret, caret));
    shell.go_to.open = false;
    shell.notification = None;
}

/// The notice for a search that ran out of document without wrapping.
fn boundary_notice(direction: Direction) -> String {
    match direction {
        Direction::Forward => "Reached the end of the document".to_owned(),
        Direction::Backward => "Reached the beginning of the document".to_owned(),
    }
}

/// The notice for a search that carried on from the other end.
fn wrapped_notice(direction: Direction) -> String {
    match direction {
        Direction::Forward => "Reached the end of the document and wrapped around".to_owned(),
        Direction::Backward => {
            "Reached the beginning of the document and wrapped around".to_owned()
        }
    }
}

/// The notice for a search that found nothing at all.
fn not_found_notice(needle: &str) -> String {
    format!("Cannot find \"{needle}\"")
}

/// The byte offset of a character index, clamped to the text.
fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len())
}

/// The character index of a byte offset. `byte` must be on a character
/// boundary, which a search's match always is.
fn char_index(text: &str, byte: usize) -> usize {
    text[..byte].chars().count()
}

/// The toolbar's paragraph-style dropdown, which is how the reference exposes
/// heading levels (`docs/ui-reference.md` §3).
///
/// *Decision — the reference lists the styles but never says what bytes they
/// write.* They are one-to-one with the six ATX levels, in the order §3 lists them:
/// Title `#` through Subsection `######`, with `Body` ordinary text. Asking for the
/// level the lines already carry takes the marker off, which is what "the current
/// style is highlighted" has to mean to be worth showing.
fn set_paragraph(shell: &mut Shell, style: ParagraphStyle) {
    let heading = heading_for(style);
    apply_format(shell, move |text, selection| {
        nc_markdown::set_heading(text, selection, heading)
    });
    // The dropdown label keeps showing the style the last command chose. Deriving
    // it from the caret's line instead is its own piece of work; what matters here
    // is that a plain document, which cannot be given a style at all, is not made
    // to claim one.
    if let Some(tab) = shell
        .active_tab_mut()
        .filter(|tab| tab.kind == DocKind::Markdown)
    {
        tab.paragraph = style;
    }
}

/// Shows the active document as source again, and puts the caret back.
///
/// The formatted view has no caret of its own — it renders, it does not edit — so
/// the document keeps its selection the whole time it is shown. Coming back is the
/// direction that could lose something: the editor widget is a fresh one, and a new
/// `egui` text area starts at offset zero. Queuing the document's own selection
/// means the first frame it reappears puts the caret where the user left it rather
/// than at the start of the file, which is what `NC-M002`'s "no content/selection
/// loss" turns into in this design.
fn back_to_source(shell: &mut Shell) {
    let Some(tab) = shell.active_tab_mut() else {
        return;
    };
    tab.formatted = false;
    let selection = tab.document_mut().selection();
    let (anchor, head) = {
        let text = tab.text();
        (
            editor::char_index(text, selection.anchor()),
            editor::char_index(text, selection.head()),
        )
    };
    shell.pending_selection = Some((head, anchor));
}

/// The ATX level a paragraph style writes.
fn heading_for(style: ParagraphStyle) -> Heading {
    match style {
        ParagraphStyle::Title => Heading::Level(1),
        ParagraphStyle::Subtitle => Heading::Level(2),
        ParagraphStyle::Heading => Heading::Level(3),
        ParagraphStyle::Subheading => Heading::Level(4),
        ParagraphStyle::Section => Heading::Level(5),
        ParagraphStyle::Subsection => Heading::Level(6),
        ParagraphStyle::Body => Heading::Body,
    }
}

/// Runs a Markdown command against the active document and applies what comes back
/// as one document edit.
///
/// Two gates, both deliberate. `Availability::Markdown` greys these commands for a
/// plain document and the keyboard refuses to dispatch a greyed one, but a handler
/// that wrote `**` into a `.txt` file would corrupt text nobody asked to have
/// marked up — so the kind is checked here too. And a command that changes nothing
/// must not leave a dirty flag or an undo step behind: a no-op is not an edit.
fn apply_format(
    shell: &mut Shell,
    command: impl FnOnce(&str, nc_markdown::Range) -> Option<Outcome>,
) {
    let Some(tab) = shell.active_tab_mut() else {
        return;
    };
    if tab.kind != DocKind::Markdown {
        return;
    }
    let text = tab.text().to_string();
    let selection = tab.document_mut().selection();
    let Some(range) = nc_markdown::Range::new(selection.start(), selection.end()) else {
        return;
    };
    let Some(outcome) = command(&text, range) else {
        return;
    };
    if outcome.text == text {
        return;
    }
    // Rewrite only the bytes that differ, so formatting one line of a long document
    // costs the buffer no more than typing on that line would.
    let (start, before_end, after_end) = changed_span(&text, &outcome.text);
    let document = tab.document_mut();
    if document.replace_range(start..before_end, &outcome.text[start..after_end]) {
        document.select(outcome.selection.start, outcome.selection.end);
    }
}

/// Toggles `kind` around the active document's selection.
fn format_inline(shell: &mut Shell, kind: Inline) {
    apply_format(shell, move |text, selection| {
        nc_markdown::toggle_inline(text, selection, kind)
    });
}

/// Applies `kind` to every line the selection touches.
fn format_list(shell: &mut Shell, kind: ListKind) {
    apply_format(shell, move |text, selection| {
        nc_markdown::set_list(text, selection, kind)
    });
}

/// The narrowest range of `before` that, replaced, yields `after`, as
/// `(start, before_end, after_end)`: `before[start..before_end]` becomes
/// `after[start..after_end]`.
///
/// Both strings are valid UTF-8 and share exactly this byte prefix and suffix, so
/// either edge of either range is a safe character boundary: a common byte prefix
/// of two valid strings cannot stop in the middle of a character.
fn changed_span(before: &str, after: &str) -> (usize, usize, usize) {
    let prefix = before
        .bytes()
        .zip(after.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    let mut suffix = 0usize;
    while suffix < before.len() - prefix
        && suffix < after.len() - prefix
        && before.as_bytes()[before.len() - 1 - suffix]
            == after.as_bytes()[after.len() - 1 - suffix]
    {
        suffix += 1;
    }
    (prefix, before.len() - suffix, after.len() - suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::state::Shell;
    use nc_core::Document;
    use std::path::PathBuf;

    /// A scratch directory of its own, removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let directory = std::env::temp_dir()
                .join(format!("notecraft-actions-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("a scratch directory");
            Self(directory)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn shell_with(text: &str) -> Shell {
        let mut shell = Shell::new();
        *shell.tabs[0].document_mut() = Document::from_text(text);
        shell
    }

    /// A shell whose one document has been typed into, so it is dirty.
    fn shell_edited(text: &str) -> Shell {
        let mut shell = Shell::new();
        shell.tabs[0].document_mut().insert_str(text);
        shell
    }

    #[test]
    fn find_next_selects_the_match_and_reports_nothing() {
        let mut shell = shell_with("one two one");
        shell.find.query = "one".to_owned();
        search_step(&mut shell, Direction::Forward);
        assert_eq!(shell.pending_selection, Some((3, 0)));
        assert_eq!(shell.caret, 3);
        assert!(shell.notification.is_none());

        // A second step travels from the caret, past the first match.
        search_step(&mut shell, Direction::Forward);
        assert_eq!(shell.pending_selection, Some((11, 8)));
    }

    #[test]
    fn a_wrapped_search_says_so() {
        let mut shell = shell_with("one two");
        shell.find.query = "one".to_owned();
        shell.caret = 4;
        search_step(&mut shell, Direction::Forward);
        assert_eq!(shell.pending_selection, Some((3, 0)));
        assert_eq!(
            shell.notification.as_deref(),
            Some("Reached the end of the document and wrapped around")
        );
    }

    #[test]
    fn a_missing_needle_reports_it() {
        let mut shell = shell_with("one two");
        shell.find.query = "three".to_owned();
        search_step(&mut shell, Direction::Forward);
        assert_eq!(shell.notification.as_deref(), Some("Cannot find \"three\""));
        assert!(shell.pending_selection.is_none());
    }

    #[test]
    fn searching_backwards_carries_the_caret_before_the_match() {
        let mut shell = shell_with("one two one");
        shell.find.query = "one".to_owned();
        // From the start of the second match, the previous match is the first.
        shell.caret = 8;
        search_step(&mut shell, Direction::Backward);
        assert_eq!(shell.pending_selection, Some((0, 3)));
        assert_eq!(shell.caret, 0);
    }

    #[test]
    fn replace_swaps_the_match_and_advances() {
        let mut shell = shell_with("one two one");
        shell.find.query = "one".to_owned();
        shell.find.replacement = "1".to_owned();
        replace(&mut shell);
        assert_eq!(shell.tabs[0].text(), "1 two one");
        assert_eq!(shell.pending_selection, Some((1, 1)));
        assert!(shell.tabs[0].is_dirty());
    }

    #[test]
    fn replace_all_rewrites_every_match_at_once() {
        let mut shell = shell_with("one two one");
        shell.find.query = "one".to_owned();
        shell.find.replacement = "1".to_owned();
        replace_all(&mut shell);
        assert_eq!(shell.tabs[0].text(), "1 two 1");
        assert!(shell.notification.is_none());
    }

    #[test]
    fn replace_all_with_no_match_keeps_the_text() {
        let mut shell = shell_with("one two");
        shell.find.query = "three".to_owned();
        shell.find.replacement = "3".to_owned();
        replace_all(&mut shell);
        assert_eq!(shell.tabs[0].text(), "one two");
        assert_eq!(shell.notification.as_deref(), Some("Cannot find \"three\""));
    }

    #[test]
    fn go_to_moves_the_caret_and_clamps() {
        let mut shell = shell_with("ab\ncd\nef");
        shell.go_to.line = "2".to_owned();
        go_to(&mut shell);
        assert_eq!(shell.pending_selection, Some((3, 3)));
        assert!(!shell.go_to.open);

        shell.go_to.line = "99".to_owned();
        shell.go_to.open = true;
        go_to(&mut shell);
        assert_eq!(shell.pending_selection, Some((6, 6)));
    }

    #[test]
    fn go_to_ignores_a_non_number() {
        let mut shell = shell_with("ab\ncd");
        shell.go_to.line = "two".to_owned();
        go_to(&mut shell);
        assert!(shell.pending_selection.is_none());
    }

    // The Find/Replace and clipboard clauses the engine suites settled but no
    // shell test had reached from a user's command: that Replace All is undone in
    // one step, that Match case / Whole word actually steer a search, that a
    // search stops at the edge when wrapping is off, and that the shell's
    // `egui`-backed clipboard carries the selection as CRLF.

    /// The text the context last asked the windowing layer to copy, if any.
    ///
    /// `egui` 0.36 does not hold clipboard text on the output; a copy queues an
    /// `OutputCommand::CopyText` the backend drains next frame — the real path a
    /// `Ctrl+C` takes out of the shell.
    fn copied_text(ctx: &egui::Context) -> Option<String> {
        ctx.output(|output| {
            output.commands.iter().find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
        })
    }

    #[test]
    fn copy_puts_the_selection_on_the_clipboard_as_crlf() {
        // NC-E002: the engine shows a multi-line selection copies as CRLF; nothing
        // yet showed the shell's `egui`-backed clipboard doing the same, which is
        // the copy a user actually gets out of the window.
        let mut shell = shell_with("one\ntwo");
        shell.tabs[0].document_mut().select(0, 7);
        let ctx = egui::Context::default();

        copy(&mut shell, &ctx);

        assert_eq!(
            copied_text(&ctx).as_deref(),
            Some("one\r\ntwo"),
            "the line break reaches the clipboard as CRLF, as Windows expects"
        );
    }

    #[test]
    fn copy_without_a_selection_sends_nothing_to_the_clipboard() {
        let mut shell = shell_with("hello");
        let ctx = egui::Context::default();

        copy(&mut shell, &ctx);

        assert!(
            copied_text(&ctx).is_none(),
            "a copy with nothing selected queues no clipboard write"
        );
    }

    #[test]
    fn cut_puts_the_selection_on_the_clipboard_and_removes_it() {
        let mut shell = shell_with("one\ntwo");
        shell.tabs[0].document_mut().select(0, 3); // "one"
        let ctx = egui::Context::default();

        cut(&mut shell, &ctx);

        assert_eq!(copied_text(&ctx).as_deref(), Some("one"));
        assert_eq!(shell.tabs[0].text(), "\ntwo", "the cut text is gone");
        assert!(shell.tabs[0].is_dirty(), "cutting is an edit");
        assert!(
            !shell.availability().has_selection,
            "nothing is left selected, so Cut/Copy grey back out"
        );
    }

    #[test]
    fn replace_all_is_a_single_undo_step_from_the_shell() {
        // NC-S003: "Undo as a single action". The engine shows `replace_ranges` is
        // one step; this shows the shell's Replace All is too — one Ctrl+Z brings
        // every match back, not one per replacement.
        let mut shell = shell_edited("one two one");
        shell.find.query = "one".to_owned();
        shell.find.replacement = "1".to_owned();
        let before = shell.tabs[0].file.document().undo_depth();

        replace_all(&mut shell);

        assert_eq!(shell.tabs[0].text(), "1 two 1");
        assert_eq!(
            shell.tabs[0].file.document().undo_depth(),
            before + 1,
            "however many matches, they are one step"
        );

        undo(&mut shell);
        assert_eq!(
            shell.tabs[0].text(),
            "one two one",
            "one undo brings them all back"
        );
    }

    #[test]
    fn a_search_with_match_case_only_finds_the_written_case() {
        // NC-S004: the dialog's Match case option, reaching a search through
        // shell.find.options. Off is the default and matches either case; on, only
        // the exact case does, and a needle that then matches nothing says so.
        let mut shell = shell_with("One one ONE");
        shell.find.query = "one".to_owned();
        shell.find.options.match_case = true;

        search_step(&mut shell, Direction::Forward);

        assert_eq!(
            shell.pending_selection,
            Some((7, 4)),
            "case-sensitive, only the lowercase word matches"
        );

        shell.find.query = "onE".to_owned();
        shell.pending_selection = None;
        search_step(&mut shell, Direction::Forward);
        assert!(shell.pending_selection.is_none());
        assert!(shell.notification.is_some(), "it reports finding nothing");
    }

    #[test]
    fn a_search_with_whole_word_skips_matches_inside_a_word() {
        // NC-S004's other option: Whole word only. "one" must skip the "one"
        // buried in "onetwo" and land on the standalone word.
        let mut shell = shell_with("onetwo one");
        shell.find.query = "one".to_owned();
        shell.find.options.whole_word = true;

        search_step(&mut shell, Direction::Forward);

        assert_eq!(shell.pending_selection, Some((10, 7)));
    }

    #[test]
    fn a_search_that_reaches_the_end_without_wrapping_says_so() {
        // NC-S005's untested half: with wrapping turned off, a search that runs
        // past the last match stops at the edge and says so, rather than silently
        // wrapping or claiming the needle is absent.
        let mut shell = shell_with("one two");
        shell.find.query = "one".to_owned();
        shell.find.wraps = false;
        shell.caret = 4; // past the only match

        search_step(&mut shell, Direction::Forward);

        assert_eq!(
            shell.notification.as_deref(),
            Some("Reached the end of the document")
        );
        assert!(
            shell.pending_selection.is_none(),
            "the caret did not wrap back to the match"
        );
    }

    #[test]
    fn byte_and_char_offsets_agree_across_multibyte_text() {
        let text = "aé€b";
        assert_eq!(char_to_byte(text, 0), 0);
        assert_eq!(char_to_byte(text, 1), 1);
        assert_eq!(char_to_byte(text, 2), 3);
        assert_eq!(char_to_byte(text, 3), 6);
        assert_eq!(char_to_byte(text, 4), 7);
        assert_eq!(char_to_byte(text, 99), 7);
        assert_eq!(char_index(text, 0), 0);
        assert_eq!(char_index(text, 3), 2);
        assert_eq!(char_index(text, 6), 3);
    }

    #[test]
    fn find_searches_across_multibyte_text() {
        let mut shell = shell_with("héllo wörld héllo");
        shell.find.query = "héllo".to_owned();
        search_step(&mut shell, Direction::Forward);
        // The second match starts at char 12 ("héllo wörld ").
        search_step(&mut shell, Direction::Forward);
        assert_eq!(shell.pending_selection, Some((17, 12)));
    }

    #[test]
    fn saving_a_document_with_no_path_opens_save_as() {
        let mut shell = shell_edited("hello");
        save(&mut shell);

        let dialog = shell.file_dialog.as_ref().expect("Save As opens");
        assert_eq!(dialog.mode, FileDialogMode::SaveAs);
        assert_eq!(dialog.tab, 0);
        assert!(
            dialog.file_name.is_empty(),
            "a new document has no name yet"
        );
    }

    #[test]
    fn closing_a_clean_tab_just_closes_it() {
        let mut shell = Shell::new();
        shell.open_tab(DocKind::Plain);

        close_tab(&mut shell, 1);

        assert!(shell.prompt.is_none());
        assert_eq!(shell.tabs.len(), 1);
    }

    #[test]
    fn closing_a_dirty_tab_asks_before_discarding() {
        let mut shell = shell_edited("hello");
        close_tab(&mut shell, 0);

        assert_eq!(shell.prompt, Some(Prompt::UnsavedChanges { tab: 0 }));
        assert_eq!(shell.tabs.len(), 1, "nothing has closed yet");

        answer_unsaved_dont_save(&mut shell);
        assert!(shell.prompt.is_none());
        assert!(shell.tabs[0].text().is_empty(), "the last tab is replaced");
    }

    #[test]
    fn cancelling_a_prompt_keeps_the_document() {
        let mut shell = shell_edited("hello");
        close_tab(&mut shell, 0);

        answer_cancel(&mut shell);

        assert!(shell.prompt.is_none());
        assert_eq!(shell.tabs[0].text(), "hello");
    }

    #[test]
    fn saving_from_the_prompt_opens_save_as_and_remembers_the_close() {
        let mut shell = shell_edited("hello");
        close_tab(&mut shell, 0);

        answer_unsaved_save(&mut shell);

        assert!(shell.prompt.is_none());
        let dialog = shell.file_dialog.as_ref().expect("Save As opens");
        assert_eq!(dialog.tab, 0);
        assert_eq!(shell.save_then, Some((0, SaveThen::Close)));
    }

    #[test]
    fn save_as_writes_the_file_and_finishes_a_pending_close() {
        let scratch = Scratch::new("save-as");
        let path = scratch.0.join("note.txt");

        let mut shell = shell_edited("hello");
        close_tab(&mut shell, 0);
        answer_unsaved_save(&mut shell);
        // Fill the File name field the way the dialog's own field would.
        if let Some(dialog) = shell.file_dialog.as_mut() {
            dialog.file_name = path.display().to_string();
        }

        confirm_file_dialog(&mut shell);

        assert!(shell.file_dialog.is_none());
        assert_eq!(shell.save_then, None);
        assert_eq!(std::fs::read_to_string(&path).expect("the file"), "hello");
        assert!(shell.tabs[0].text().is_empty(), "the close resumed");
        assert!(!shell.tabs[0].is_dirty());
    }

    #[test]
    fn save_as_over_an_existing_file_asks_first() {
        let scratch = Scratch::new("overwrite");
        let path = scratch.0.join("note.txt");
        std::fs::write(&path, b"old").expect("the existing file");

        let mut shell = shell_edited("new");
        save_as_dialog(&mut shell);
        if let Some(dialog) = shell.file_dialog.as_mut() {
            dialog.file_name = path.display().to_string();
        }

        confirm_file_dialog(&mut shell);
        assert!(matches!(shell.prompt, Some(Prompt::Overwrite { .. })));
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            b"old",
            "nothing written"
        );
        assert!(shell.file_dialog.is_some(), "the dialog stays open");

        answer_overwrite_replace(&mut shell);
        assert!(shell.prompt.is_none());
        assert!(shell.file_dialog.is_none());
        assert_eq!(std::fs::read(&path).expect("the file"), b"new");
    }

    #[test]
    fn confirming_open_reads_the_file_into_a_new_tab() {
        let scratch = Scratch::new("open");
        let path = scratch.0.join("note.txt");
        std::fs::write(&path, b"one\r\ntwo\r\n").expect("the file");

        let mut shell = Shell::new();
        open_file_dialog(&mut shell);
        if let Some(dialog) = shell.file_dialog.as_mut() {
            dialog.file_name = path.display().to_string();
        }

        confirm_file_dialog(&mut shell);

        assert!(shell.file_dialog.is_none());
        assert_eq!(shell.tabs.len(), 2);
        assert_eq!(shell.active, 1);
        assert_eq!(shell.tabs[1].text(), "one\ntwo\n");
        assert_eq!(shell.tabs[1].title(), "note.txt");
        assert!(!shell.tabs[1].is_dirty(), "opening is not an edit");
    }

    #[test]
    fn opening_a_missing_file_reports_it_and_keeps_the_dialog() {
        let scratch = Scratch::new("open-missing");
        let mut shell = Shell::new();
        open_file_dialog(&mut shell);
        if let Some(dialog) = shell.file_dialog.as_mut() {
            dialog.file_name = scratch.0.join("nowhere.txt").display().to_string();
        }

        confirm_file_dialog(&mut shell);

        assert_eq!(shell.tabs.len(), 1, "no tab was opened");
        let dialog = shell.file_dialog.as_ref().expect("the dialog stays open");
        assert!(dialog.error.is_some(), "the failure is reported");
    }

    #[test]
    fn saving_ansi_text_that_does_not_fit_asks_before_replacing() {
        let scratch = Scratch::new("unrepresentable");
        let path = scratch.0.join("ansi.txt");
        std::fs::write(&path, b"caf\xE9").expect("an ANSI file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&path).expect("the file opens");
        assert_eq!(shell.tabs[0].file.encoding(), Encoding::Ansi);

        // A Cyrillic letter has no ANSI byte, so Save must ask first.
        shell.tabs[0].file.insert_str("Ж");
        save(&mut shell);
        assert!(matches!(shell.prompt, Some(Prompt::Unrepresentable { .. })));
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            b"caf\xE9",
            "nothing written yet"
        );

        answer_unrepresentable_save(&mut shell);
        assert!(shell.prompt.is_none());
        assert_eq!(std::fs::read(&path).expect("the file"), b"?caf\xE9");
    }

    #[test]
    fn choosing_another_encoding_reopens_save_as() {
        let scratch = Scratch::new("choose-encoding");
        let path = scratch.0.join("ansi.txt");
        std::fs::write(&path, b"caf\xE9").expect("an ANSI file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&path).expect("the file opens");
        shell.tabs[0].file.insert_str("Ж");
        save(&mut shell);

        answer_unrepresentable_choose_encoding(&mut shell);

        assert!(shell.prompt.is_none());
        let dialog = shell.file_dialog.as_ref().expect("Save As opens");
        assert_eq!(dialog.mode, FileDialogMode::SaveAs);
        assert_eq!(dialog.tab, 0);
    }

    #[test]
    fn a_save_that_detours_still_closes_the_tab_it_was_answering_for() {
        // The write does not always happen in the call that asked for it: a text
        // the file's encoding cannot hold opens the encoding question first, and
        // the close the user asked for has to survive that detour — otherwise
        // answering "Save anyway" writes the file and leaves the tab open.
        let scratch = Scratch::new("unrepresentable-close");
        let path = scratch.0.join("ansi.txt");
        std::fs::write(&path, b"caf\xE9").expect("an ANSI file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&path).expect("the file opens");
        // A Cyrillic letter has no ANSI byte, and the caret is at the start of a
        // freshly opened document, so this writes "Жcaf\xE9".
        shell.tabs[0].file.insert_str("Ж");

        close_tab(&mut shell, 0);
        answer_unsaved_save(&mut shell);

        assert!(matches!(shell.prompt, Some(Prompt::Unrepresentable { .. })));
        assert_eq!(
            shell.save_then,
            Some((0, SaveThen::Close)),
            "the close is still waiting behind the question"
        );

        answer_unrepresentable_save(&mut shell);

        assert!(shell.prompt.is_none());
        assert_eq!(shell.save_then, None, "the close resumed");
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            b"?caf\xE9",
            "and the lossy write happened"
        );
        assert_eq!(shell.tabs.len(), 1, "the tab closed");
        assert!(shell.tabs[0].text().is_empty(), "the fresh last tab");
    }

    #[test]
    fn a_save_as_that_cannot_be_written_is_reported_and_the_original_survives() {
        // NC-D006's shell half: a write that cannot happen must not touch the file
        // the document came from, and the shell has to say so. A Save As whose path
        // sits under a regular file cannot create its sibling temporary, so the
        // atomic write is refused before anything moves. errors.rs proves the byte
        // guarantee; this proves the user is told and the tab keeps its edits.
        let scratch = Scratch::new("save-as-fails");
        let note = scratch.0.join("note.txt");
        std::fs::write(&note, b"one").expect("a file");
        let blocker = scratch.0.join("blocker.txt");
        std::fs::write(&blocker, b"not a directory").expect("a regular file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&note).expect("the file opens");
        shell.tabs[0].file.insert_str(" more");
        save_as_dialog(&mut shell);
        if let Some(dialog) = shell.file_dialog.as_mut() {
            // "blocker.txt" is a file, so this path has no directory to write into.
            dialog.file_name = blocker.join("nested.txt").display().to_string();
        }

        confirm_file_dialog(&mut shell);

        let dialog = shell
            .file_dialog
            .as_ref()
            .expect("the dialog stays open on failure");
        assert!(dialog.error.is_some(), "the write failure is reported");
        assert!(shell.tabs[0].is_dirty(), "the document is still unsaved");
        assert_eq!(
            std::fs::read(&note).expect("the file is still there"),
            b"one",
            "the atomic write never started, so the original is untouched"
        );
    }

    #[test]
    fn the_shell_opens_empty_malformed_and_large_files_without_failing() {
        // NC-D007's shell half: the reading rules hold at the edges, through the
        // same open the Open dialog and Recent use. An empty file is an empty
        // document; a byte string that is not UTF-8 is read as ANSI rather than
        // crashing or substituting; a large file comes in whole. stress.rs and the
        // encoding suites prove the engine; this proves nothing falls over on the
        // way in.
        let scratch = Scratch::new("stress-open");

        let empty = scratch.0.join("empty.txt");
        std::fs::write(&empty, b"").expect("an empty file");

        let malformed = scratch.0.join("bytes.txt");
        // Not valid UTF-8 and carrying no UTF-16 mark, so it is read as ANSI.
        std::fs::write(&malformed, [0xE9, 0x80, 0xFF]).expect("a non-UTF-8 file");

        let large = scratch.0.join("large.txt");
        std::fs::write(&large, "line\n".repeat(5_000).as_bytes()).expect("a large file");

        let mut shell = Shell::new();

        open_path(&mut shell, &empty, None);
        assert_eq!(shell.tabs.len(), 2, "a tab opened for the empty file");
        assert_eq!(
            shell.tabs[1].text(),
            "",
            "an empty file is an empty document"
        );
        assert!(!shell.tabs[1].is_dirty());

        open_path(&mut shell, &malformed, None);
        assert_eq!(shell.tabs.len(), 3, "a tab opened for the malformed file");
        assert_eq!(
            shell.tabs[2].file.encoding(),
            Encoding::Ansi,
            "not UTF-8 and no wide mark, so the ANSI fallback reads it"
        );
        assert_eq!(
            shell.tabs[2].text().chars().count(),
            3,
            "one character per byte"
        );

        open_path(&mut shell, &large, None);
        assert_eq!(shell.tabs.len(), 4, "a tab opened for the large file");
        assert_eq!(
            shell.tabs[3].text().lines().count(),
            5_000,
            "every line came in"
        );
        assert!(!shell.tabs[3].is_dirty());
    }

    #[test]
    fn closing_the_window_walks_the_dirty_documents() {
        let mut shell = shell_edited("one");
        shell.open_tab(DocKind::Plain);

        close_window(&mut shell);
        assert_eq!(shell.prompt, Some(Prompt::UnsavedChanges { tab: 0 }));

        answer_unsaved_dont_save(&mut shell);

        assert!(shell.prompt.is_none());
        assert!(
            shell.exit_pending,
            "the window closes once nothing is dirty"
        );
        assert!(!shell.closing_window);
    }

    #[test]
    fn every_landed_command_is_wired() {
        for spec in nc_commands::REGISTRY {
            // A never-available entry is a placeholder for a surface a later
            // milestone fills in (the empty Recent list), not a command with an
            // effect to wire (`NC-F010`).
            if spec.availability == nc_commands::Availability::Never {
                continue;
            }
            if spec.milestone.number() <= LANDED_MILESTONE {
                assert!(
                    handles(spec.id),
                    "{} (M{}) is not wired, so its accelerator would be inert",
                    spec.id,
                    spec.milestone.number(),
                );
            }
        }
    }

    #[test]
    fn an_unwired_command_that_shows_is_disabled() {
        // The other half of `every_landed_command_is_wired`. That one says a landed
        // command must be wired; this enforces `AGENT.md` §2 from the other side: a
        // command the user can see (a menu placement) or press (an accelerator) that
        // this build does not yet act on must be greyed, not offered live and then do
        // nothing. Only `Never` (a placeholder a later milestone fills in) and
        // `Service` (no AI service is configured in this build) leave such a command
        // disabled in every state the running app can actually reach.
        for spec in nc_commands::REGISTRY {
            let shown = spec.placement.is_some() || spec.shortcut.is_some();
            if shown && !handles(spec.id) {
                assert!(
                    matches!(
                        spec.availability,
                        nc_commands::Availability::Never | nc_commands::Availability::Service
                    ),
                    "{} is offered to the user but not wired, so §2 says it must be \
                     disabled — yet its availability is {:?}",
                    spec.id,
                    spec.availability,
                );
            }
        }
    }

    #[test]
    fn undo_and_redo_route_through_the_document() {
        let mut shell = shell_edited("hello");

        undo(&mut shell);
        assert_eq!(shell.tabs[0].text(), "");
        assert_eq!(shell.pending_selection, Some((0, 0)));

        redo(&mut shell);
        assert_eq!(shell.tabs[0].text(), "hello");
        assert_eq!(shell.caret, 5);
    }

    #[test]
    fn select_all_selects_the_whole_document() {
        let mut shell = shell_with("one two");

        select_all(&mut shell);

        assert_eq!(shell.pending_selection, Some((7, 0)));
        assert_eq!(shell.caret, 7);
        assert!(shell.tabs[0].file.document().has_selection());
        assert!(
            shell.availability().has_selection,
            "Cut/Copy become available"
        );
    }

    #[test]
    fn delete_removes_the_selection_through_the_document() {
        let mut shell = shell_with("one two");
        shell.tabs[0].document_mut().select(0, 3);

        delete_selection(&mut shell);

        assert_eq!(shell.tabs[0].text(), " two");
        assert_eq!(shell.caret, 0);
    }

    #[test]
    fn copy_leaves_the_text_alone_and_cut_removes_it() {
        let ctx = egui::Context::default();

        let mut shell = shell_with("one two");
        shell.tabs[0].document_mut().select(0, 3);
        copy(&mut shell, &ctx);
        assert_eq!(shell.tabs[0].text(), "one two");

        let mut shell = shell_with("one two");
        shell.tabs[0].document_mut().select(0, 3);
        cut(&mut shell, &ctx);
        assert_eq!(shell.tabs[0].text(), " two");
        assert_eq!(shell.caret, 0);
    }

    #[test]
    fn f5_inserts_the_time_and_date_as_one_edit() {
        let mut shell = Shell::new();

        insert_time_date(&mut shell, TimeDate::new(2026, 12, 10, 15, 47, 6));

        assert_eq!(shell.tabs[0].text(), "3:47 PM 12/10/2026");
        assert!(shell.tabs[0].is_dirty());
        assert_eq!(shell.caret, 18);

        // The whole insertion comes back out with one undo.
        undo(&mut shell);
        assert_eq!(shell.tabs[0].text(), "");
    }

    #[test]
    fn switching_tabs_cycles_and_wraps() {
        let mut shell = Shell::new();
        shell.open_tab(DocKind::Plain);
        shell.open_tab(DocKind::Plain);
        assert_eq!(shell.active, 2);

        next_tab(&mut shell);
        assert_eq!(shell.active, 0, "wraps from the last to the first");

        previous_tab(&mut shell);
        assert_eq!(shell.active, 2, "wraps from the first to the last");

        previous_tab(&mut shell);
        assert_eq!(shell.active, 1);
    }

    #[test]
    fn switching_tabs_does_nothing_with_a_single_tab() {
        let mut shell = Shell::new();

        next_tab(&mut shell);
        assert_eq!(shell.active, 0);
        previous_tab(&mut shell);
        assert_eq!(shell.active, 0);
    }

    #[test]
    fn save_all_writes_every_dirty_document() {
        let scratch = Scratch::new("save-all");
        let first = scratch.0.join("one.txt");
        let second = scratch.0.join("two.txt");
        std::fs::write(&first, b"one").expect("the first file");
        std::fs::write(&second, b"two").expect("the second file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&first).expect("the first opens");
        shell.open_tab(DocKind::Plain);
        shell.tabs[1] = Tab::open(&second).expect("the second opens");
        shell.tabs[0].file.insert_str("!");
        shell.tabs[1].file.insert_str("!");

        save_all(&mut shell);

        assert_eq!(std::fs::read(&first).expect("the first file"), b"!one");
        assert_eq!(std::fs::read(&second).expect("the second file"), b"!two");
        assert!(!shell.tabs[0].is_dirty());
        assert!(!shell.tabs[1].is_dirty());
    }

    #[test]
    fn save_all_stops_at_a_document_that_has_no_path() {
        let scratch = Scratch::new("save-all-stop");
        let path = scratch.0.join("one.txt");
        std::fs::write(&path, b"one").expect("the file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&path).expect("the file opens");
        shell.tabs[0].file.insert_str("!");
        shell.open_tab(DocKind::Plain);
        shell.tabs[1].file.insert_str("untitled");

        save_all(&mut shell);

        // The path-backed tab is written before the untitled one stops the run
        // by opening Save As, which is where the answer resumes.
        assert_eq!(std::fs::read(&path).expect("the file"), b"!one");
        let dialog = shell.file_dialog.as_ref().expect("Save As opens");
        assert_eq!(dialog.tab, 1);
    }

    #[test]
    fn opening_a_file_that_is_already_open_focuses_its_tab() {
        let scratch = Scratch::new("duplicate-open");
        let path = scratch.0.join("note.txt");
        std::fs::write(&path, b"hello").expect("the file");

        let mut shell = Shell::new();
        shell.open_tab(DocKind::Plain);
        shell.tabs[1] = Tab::open(&path).expect("the file opens");
        shell.active = 0;

        open_file_dialog(&mut shell);
        if let Some(dialog) = shell.file_dialog.as_mut() {
            dialog.file_name = path.display().to_string();
        }
        confirm_file_dialog(&mut shell);

        assert_eq!(shell.tabs.len(), 2, "the file is not opened a second time");
        assert_eq!(shell.active, 1, "the existing tab is focused");
        assert!(shell.file_dialog.is_none());
    }

    #[test]
    fn the_zoom_commands_step_by_ten_and_stop_at_the_ends() {
        // The View ▸ Zoom items and their accelerators change the editor's size;
        // nothing but the accelerator *mapping* was pinned, so the running value and
        // the reason it stops climbing and falling — so a document can never be
        // zoomed to nothing or past the readable ceiling — go here. Driven through
        // `run`, the same path a menu click and a key chord take.
        let ctx = egui::Context::default();
        let mut shell = Shell::new();
        assert_eq!(shell.zoom, 100, "a document opens at 100%");

        run(&mut shell, &ctx, CommandId("cmd.view.zoom_in"));
        assert_eq!(shell.zoom, 110, "Zoom In steps up by ten");

        // Pressing on does not run away: it settles at the ceiling and stays there.
        for _ in 0..100 {
            run(&mut shell, &ctx, CommandId("cmd.view.zoom_in"));
        }
        assert_eq!(shell.zoom, 500, "Zoom In stops at 500%");

        // And down settles at the floor rather than at zero, which would be no text.
        for _ in 0..200 {
            run(&mut shell, &ctx, CommandId("cmd.view.zoom_out"));
        }
        assert_eq!(shell.zoom, 10, "Zoom Out stops at 10%");

        // Reset returns to the plain size from wherever it ended up.
        run(&mut shell, &ctx, CommandId("cmd.view.zoom_reset"));
        assert_eq!(shell.zoom, 100);
    }

    #[test]
    fn page_setup_opens_the_dialog_seeded_from_the_current_page() {
        let ctx = egui::Context::default();
        let mut shell = Shell::new();
        shell.page_setup.paper = nc_textproc::PaperSize::Legal;

        run(&mut shell, &ctx, CommandId("cmd.file.page_setup"));

        assert!(
            shell.page_setup_dialog.open,
            "File ▸ Page setup opens the §8 dialog"
        );
        assert_eq!(
            shell.page_setup_dialog.draft.paper,
            nc_textproc::PaperSize::Legal,
            "the dialog opens editing the page the shell is already set to"
        );
    }

    #[test]
    fn new_window_sets_the_request() {
        let mut shell = Shell::new();
        let ctx = egui::Context::default();

        run(&mut shell, &ctx, CommandId("cmd.file.new_window"));

        assert!(
            shell.new_window_request,
            "the application will open a window"
        );
    }

    #[test]
    fn a_changed_span_describes_the_edit_it_names() {
        // Every pair is one a formatting command produces. The contract is that
        // replacing `before[start..before_end]` with `after[start..after_end]`
        // yields `after` — which is what `apply_format` relies on. Getting the
        // after-side edge wrong (writing the whole tail instead) duplicated text
        // wherever a marker landed with shared bytes on both sides of it.
        for (before, after) in [
            ("press the button", "press the but****ton"),
            ("press the button", "press the **button**"),
            ("## Chapter", "Chapter"),
            ("**important** news", "important news"),
            ("- one\n- two\n", "- one\n  - two\n"),
            ("one\ntwo\n", "- one\n- two\n"),
            ("", "|   |\n| --- |"),
            ("see docs", "see []()docs"),
            ("a", "a"),
        ] {
            let (start, before_end, after_end) = changed_span(before, after);
            let rebuilt = format!(
                "{}{}{}",
                &before[..start],
                &after[start..after_end],
                &before[before_end..]
            );
            assert_eq!(
                rebuilt, after,
                "span ({start}, {before_end}, {after_end}) misdescribes {before:?} -> {after:?}"
            );
        }
    }

    #[test]
    fn a_changed_span_lands_on_character_boundaries() {
        // Both edges are used as slice indices into the buffer, so a span that cut
        // a multi-byte character would panic in `replace_range`.
        for (before, after) in [
            ("naïve résumé", "naïve ré****sumé"),
            ("cafés and naïve", "cafés and **naïve**"),
            ("日本語のテキスト", "日本語の****テキスト"),
        ] {
            let (start, before_end, after_end) = changed_span(before, after);
            assert!(
                before.is_char_boundary(start)
                    && before.is_char_boundary(before_end)
                    && after.is_char_boundary(after_end),
                "{before:?} -> {after:?} split a character"
            );
        }
    }
}
