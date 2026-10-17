//! M6 acceptance suite — tabs, documents, and restart.
//!
//! `ROADMAP.md` M6 sets one exit criterion: *tab operations and restarts preserve
//! the expected documents and state without losing unsaved work.* The rows below
//! are checked against the acceptance text in `docs/feature-matrix.md`, through
//! the same state and the same session file the running shell uses — not against
//! a model of them.
//!
//! What this suite can and cannot settle is worth stating plainly. It drives the
//! shell's state and its session store, so it settles the behaviour criteria:
//! what a tab operation does to the documents, what a prompt asks, and what a
//! restart brings back. It cannot settle the *visual* criteria — that the tab
//! strip's geometry matches the reference (`NC-T001`), that the unsaved dot is
//! actually painted (`NC-T004`), that a second top-level window really appears
//! (`NC-F003`), or that `File ▸ Recent` draws its list (`NC-F010`). Those need a
//! display, and this crate has no display in a test; they stay `IMPLEMENTED`.

use std::path::PathBuf;

use crate::ui::actions;
use crate::ui::session::{self, Session, save_to};
use crate::ui::state::{DocKind, OpenBehavior, Prompt, Shell, Tab};

/// A scratch directory, removed when the test ends. Named after the test, since
/// every test in this binary shares one process id.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("notecraft-m6-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        Self(directory)
    }

    /// A path inside the directory, which need not exist.
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// A path inside the directory that holds `bytes`.
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).expect("a file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A tab carrying `text` as unsaved work.
fn dirty_tab(shell: &mut Shell, text: &str) {
    shell.open_tab(DocKind::Plain);
    append(shell.tabs.last_mut().expect("the new tab"), text);
}

/// Types `text` at the end of a tab's document.
///
/// An opened document keeps its caret at the start, and `insert_str` writes at
/// the caret, so appending takes an explicit move — otherwise the text lands
/// before what was read from the file.
fn append(tab: &mut Tab, text: &str) {
    let end = tab.text().len();
    tab.document_mut().set_caret(end);
    tab.document_mut().insert_str(text);
}

/// Whether the shell has asked the window to close.
///
/// `File ▸ Exit` runs through [`actions::run`], which ends by pumping a frame;
/// the pump spends `exit_pending` the moment it sends the viewport command, so
/// the durable evidence of a close is the command, not the flag.
fn asked_to_close(ctx: &egui::Context) -> bool {
    ctx.viewport(|viewport| {
        viewport
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Close))
    })
}

/// Saves `shell` the way the shell does on exit and reads it back, returning the
/// shell the next start would open with.
fn restart(scratch: &Scratch, shell: &Shell) -> Shell {
    let store = scratch.path("session");
    save_to(&store, &Session::of(shell)).expect("the session is written");
    session::load_from(&store)
        .expect("the session is read")
        .into_shell()
}

// ---- NC-F011 — session restore: "Restores tabs incl. unsaved state" ----------

#[test]
fn a_restart_reopens_every_document_with_its_unsaved_work() {
    let scratch = Scratch::new("restart");
    let kept = scratch.file("kept.txt", b"on disk\r\n");

    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&kept).expect("the file opens");
    dirty_tab(&mut shell, "typed, never saved");
    shell.open_tab(DocKind::Markdown);
    shell.tabs[2]
        .document_mut()
        .insert_str("# Ideas\nsecond paragraph");
    shell.active = 2;

    let restored = restart(&scratch, &shell);

    assert_eq!(restored.tabs.len(), 3, "every tab comes back");
    assert_eq!(restored.active, 2, "and the one that was shown is shown");
    assert_eq!(restored.tabs[0].text(), "on disk\n");
    assert!(
        !restored.tabs[0].is_dirty(),
        "a document that was saved stays saved"
    );
    assert_eq!(restored.tabs[1].text(), "typed, never saved");
    assert!(restored.tabs[1].is_dirty(), "unsaved work is still unsaved");
    assert_eq!(restored.tabs[2].text(), "# Ideas\nsecond paragraph");
    assert!(restored.tabs[2].is_dirty());
    assert_eq!(
        restored.tabs[2].kind,
        DocKind::Markdown,
        "an unsaved Markdown document is still Markdown"
    );

    let _ = scratch;
}

#[test]
fn a_restarted_document_keeps_the_encoding_and_line_endings_it_was_written_in() {
    let scratch = Scratch::new("restart-style");
    // UTF-16LE with a byte order mark, terminated with LF.
    let kept = scratch.path("wide.txt");
    let mut bytes = Vec::new();
    for unit in "\u{feff}one\u{a}two".encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&kept, bytes).expect("a file");

    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&kept).expect("the file opens");
    assert_eq!(
        shell.tabs[0].file.encoding(),
        nc_textproc::Encoding::Utf16Le
    );

    let restored = restart(&scratch, &shell);

    assert_eq!(restored.tabs[0].text(), "one\ntwo");
    assert_eq!(
        restored.tabs[0].file.encoding(),
        nc_textproc::Encoding::Utf16Le,
        "the encoding survives the restart"
    );
    assert_eq!(
        restored.tabs[0].file.line_ending(),
        nc_core::LineEnding::Lf,
        "and so do the line endings"
    );

    // And saving it untouched writes the same bytes it was read from.
    let bytes_before = std::fs::read(&kept).expect("the file");
    let mut restored = restored;
    restored.tabs[0].file.save().expect("a save");
    assert_eq!(
        std::fs::read(&kept).expect("the file"),
        bytes_before,
        "a restart must not change how a clean document is written"
    );
}

#[test]
fn a_restarted_unsaved_document_saves_to_its_own_file_without_losing_text() {
    let scratch = Scratch::new("restart-dirty-save");
    let path = scratch.file("note.txt", b"first line\r\n");

    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&path).expect("the file opens");
    append(&mut shell.tabs[0], " and more, unsaved");
    assert!(shell.tabs[0].is_dirty());

    let mut restored = restart(&scratch, &shell);
    assert!(restored.tabs[0].is_dirty(), "the work is still unsaved");
    assert_eq!(restored.tabs[0].text(), "first line\n and more, unsaved");

    restored.tabs[0].file.save().expect("the save");
    assert_eq!(
        std::fs::read(&path).expect("the file"),
        b"first line\r\n and more, unsaved",
        "the restart loses nothing and the file keeps CRLF"
    );
    assert!(!restored.tabs[0].is_dirty(), "and it is saved now");
}

#[test]
fn a_restart_with_nothing_readable_falls_back_to_one_empty_document() {
    let scratch = Scratch::new("restart-empty");
    let mut shell = Shell::new();
    // Named but absent, and clean, so there is nothing to carry across.
    shell.tabs[0] = Tab::open(scratch.path("gone.txt")).unwrap_or_else(|_| Tab::plain());

    let restored = restart(&scratch, &shell);

    assert_eq!(
        restored.tabs.len(),
        1,
        "the window still opens with a document"
    );
    assert!(restored.tabs[0].text().is_empty());
    assert!(!restored.tabs[0].is_dirty());
}

// ---- NC-F002 — new tab: "Adds a tab without disturbing others" ---------------

#[test]
fn a_new_tab_leaves_the_other_documents_untouched() {
    let mut shell = Shell::new();
    dirty_tab(&mut shell, "first");
    shell.open_tab(DocKind::Markdown);
    shell.tabs[2].document_mut().insert_str("# Heading");
    shell.active = 0;

    actions::run(
        &mut shell,
        &egui::Context::default(),
        nc_commands::CommandId("cmd.file.new_tab"),
    );

    assert_eq!(shell.tabs.len(), 4, "one tab is added");
    assert_eq!(shell.active, 3, "and it becomes the active one");
    assert!(shell.tabs[3].text().is_empty(), "and it starts empty");
    assert!(!shell.tabs[3].is_dirty());
    assert_eq!(shell.tabs[1].text(), "first", "the others are as they were");
    assert!(shell.tabs[1].is_dirty());
    assert_eq!(shell.tabs[2].text(), "# Heading");
    assert_eq!(shell.tabs[2].kind, DocKind::Markdown);
}

// ---- NC-F008 — close tab: "Closes the active tab; last tab resets to untitled"

#[test]
fn closing_a_clean_tab_removes_it_and_the_active_tab_follows() {
    let mut shell = Shell::new();
    shell.open_tab(DocKind::Plain);
    shell.open_tab(DocKind::Plain);
    shell.active = 2;

    actions::close_tab(&mut shell, 1);

    assert_eq!(shell.tabs.len(), 2);
    assert_eq!(shell.active, 1, "the index follows the removal");
    assert!(
        shell.prompt.is_none(),
        "a clean tab is asked about by nobody"
    );
}

#[test]
fn closing_the_last_tab_resets_to_a_fresh_untitled_document() {
    let mut shell = Shell::new();
    dirty_tab(&mut shell, "the only work");
    shell.active = 1;

    actions::close_tab(&mut shell, 1);
    actions::answer_unsaved_dont_save(&mut shell);

    assert_eq!(shell.tabs.len(), 1, "the window is never left documentless");
    assert_eq!(shell.active, 0);
    assert!(shell.tabs[0].text().is_empty());
    assert!(!shell.tabs[0].is_dirty());
    assert_eq!(shell.tabs[0].title(), "Untitled");
}

#[test]
fn closing_a_dirty_tab_asks_first_and_nothing_is_lost_until_an_answer() {
    let mut shell = Shell::new();
    dirty_tab(&mut shell, "work I care about");

    actions::close_tab(&mut shell, 1);

    assert_eq!(shell.prompt, Some(Prompt::UnsavedChanges { tab: 1 }));
    assert_eq!(shell.tabs.len(), 2, "the tab is still there");
    assert_eq!(shell.tabs[1].text(), "work I care about", "and intact");

    // Cancelling leaves the document exactly as it was.
    actions::answer_cancel(&mut shell);
    assert!(shell.prompt.is_none());
    assert_eq!(shell.tabs.len(), 2);
    assert_eq!(shell.tabs[1].text(), "work I care about");
    assert!(shell.tabs[1].is_dirty());

    // Discarding is what actually removes it.
    actions::close_tab(&mut shell, 1);
    actions::answer_unsaved_dont_save(&mut shell);
    assert_eq!(shell.tabs.len(), 1);
}

// ---- NC-T002 — tab switching: "Cyclic; works with N tabs" -------------------

#[test]
fn tab_switching_cycles_forwards_and_wraps_over_the_end() {
    let mut shell = Shell::new();
    for _ in 0..3 {
        shell.open_tab(DocKind::Plain);
    }
    assert_eq!(shell.tabs.len(), 4);
    shell.active = 0;

    let mut seen = Vec::new();
    for _ in 0..5 {
        actions::next_tab(&mut shell);
        seen.push(shell.active);
    }
    assert_eq!(
        seen,
        vec![1, 2, 3, 0, 1],
        "four tabs step 0,1,2,3 then wrap to 0"
    );
}

#[test]
fn tab_switching_cycles_backwards_and_wraps_under_the_start() {
    let mut shell = Shell::new();
    for _ in 0..3 {
        shell.open_tab(DocKind::Plain);
    }
    shell.active = 0;

    let mut seen = Vec::new();
    for _ in 0..5 {
        actions::previous_tab(&mut shell);
        seen.push(shell.active);
    }
    assert_eq!(seen, vec![3, 2, 1, 0, 3], "and the same in reverse");
}

#[test]
fn a_single_tab_switches_to_itself_and_stops() {
    let mut shell = Shell::new();

    actions::next_tab(&mut shell);
    actions::previous_tab(&mut shell);

    assert_eq!(shell.tabs.len(), 1);
    assert_eq!(shell.active, 0);
}

// ---- NC-F012 — duplicate open: "New tab vs focus existing" ------------------

#[test]
fn opening_a_path_that_is_already_open_focuses_its_tab_rather_than_opening_again() {
    let scratch = Scratch::new("duplicate");
    let kept = scratch.file("note.txt", b"hello");

    let mut shell = Shell::new();
    actions::open_path(&mut shell, &kept, None);
    let opened = shell.active;
    shell.open_tab(DocKind::Plain);

    actions::open_path(&mut shell, &kept, None);

    assert_eq!(shell.tabs.len(), 3, "the file is not opened a second time");
    assert_eq!(shell.active, opened, "the tab that has it is shown");
}

#[cfg(unix)]
#[test]
fn two_spellings_of_one_file_are_one_document() {
    use std::os::unix::fs::symlink;

    let scratch = Scratch::new("duplicate-symlink");
    let kept = scratch.file("note.txt", b"hello");
    let alias = scratch.path("alias.txt");
    symlink("note.txt", &alias).expect("a symlink to the file");
    assert_ne!(kept, alias, "one file, reachable two ways");

    let mut shell = Shell::new();
    actions::open_path(&mut shell, &kept, None);
    actions::open_path(&mut shell, &alias, None);

    let backed: Vec<_> = shell
        .tabs
        .iter()
        .filter(|tab| tab.matches_path(&kept))
        .collect();
    assert_eq!(backed.len(), 1, "and only one tab shows it");
    assert_eq!(
        shell.tabs.len(),
        2,
        "the untitled tab plus the file, not a tab per spelling"
    );
}

#[test]
fn a_different_file_still_opens_its_own_tab() {
    let scratch = Scratch::new("distinct");
    let one = scratch.file("one.txt", b"one");
    let two = scratch.file("two.txt", b"two");

    let mut shell = Shell::new();
    actions::open_path(&mut shell, &one, None);
    actions::open_path(&mut shell, &two, None);

    assert_eq!(shell.tabs.len(), 3, "distinct files are distinct tabs");
    assert_eq!(shell.active, 2);
    assert_eq!(shell.tabs[2].text(), "two");
}

// ---- NC-P005 — Settings ▸ Opening files: where an opened file goes ----------

#[test]
fn opening_in_a_new_tab_leaves_the_current_document_alone() {
    let scratch = Scratch::new("open-new-tab");
    let file = scratch.file("note.txt", b"hello");

    let mut shell = Shell::new();
    shell.opening = OpenBehavior::NewTab;
    append(&mut shell.tabs[0], "unsaved");

    actions::open_path(&mut shell, &file, None);

    assert_eq!(shell.tabs.len(), 2);
    assert_eq!(shell.active, 1, "the new tab is shown");
    assert_eq!(
        shell.tabs[0].text(),
        "unsaved",
        "and the one that was open kept its text"
    );
}

#[test]
fn replacing_the_current_tab_keeps_the_strip_the_length_it_was() {
    let scratch = Scratch::new("open-replace");
    let file = scratch.file("note.txt", b"hello");

    let mut shell = Shell::new();
    shell.opening = OpenBehavior::ReplaceTab;

    actions::open_path(&mut shell, &file, None);

    assert_eq!(shell.tabs.len(), 1, "the untitled tab is the one replaced");
    assert_eq!(shell.active, 0);
    assert_eq!(shell.tabs[0].text(), "hello");
    assert_eq!(
        shell.pending_selection,
        Some((0, 0)),
        "and the editor is told to start at the new document's own caret"
    );
}

#[test]
fn replacing_a_dirty_tab_asks_before_letting_the_edits_go() {
    let scratch = Scratch::new("open-replace-dirty");
    let file = scratch.file("note.txt", b"hello");

    let mut shell = Shell::new();
    shell.opening = OpenBehavior::ReplaceTab;
    append(&mut shell.tabs[0], "unsaved");

    actions::open_path(&mut shell, &file, None);

    assert_eq!(shell.prompt, Some(Prompt::UnsavedChanges { tab: 0 }));
    assert!(
        shell.pending_replace.is_some(),
        "the file waits for the answer"
    );
    assert_eq!(shell.tabs[0].text(), "unsaved", "nothing replaced yet");

    actions::answer_unsaved_dont_save(&mut shell);

    assert!(shell.pending_replace.is_none(), "the answer ended the wait");
    assert_eq!(shell.tabs.len(), 1);
    assert_eq!(shell.tabs[0].text(), "hello", "the file took the tab over");
}

#[test]
fn cancelling_the_replace_prompt_keeps_the_edits_and_opens_nothing() {
    let scratch = Scratch::new("open-replace-cancel");
    let file = scratch.file("note.txt", b"hello");

    let mut shell = Shell::new();
    shell.opening = OpenBehavior::ReplaceTab;
    append(&mut shell.tabs[0], "unsaved");
    actions::open_path(&mut shell, &file, None);

    actions::answer_cancel(&mut shell);

    assert!(shell.prompt.is_none());
    assert!(
        shell.pending_replace.is_none(),
        "a cancelled open is not left waiting"
    );
    assert_eq!(shell.tabs[0].text(), "unsaved");
    assert_eq!(shell.tabs.len(), 1);
}

#[test]
fn saving_from_the_replace_prompt_writes_the_edits_and_then_replaces_the_tab() {
    let scratch = Scratch::new("open-replace-save");
    let kept = scratch.file("kept.txt", b"original");
    let other = scratch.file("other.txt", b"other");

    let mut shell = Shell::new();
    actions::open_path(&mut shell, &kept, None);
    shell.opening = OpenBehavior::ReplaceTab;
    append(&mut shell.tabs[shell.active], " edited");
    assert!(shell.tabs[shell.active].is_dirty());

    actions::open_path(&mut shell, &other, None);
    assert_eq!(
        shell.prompt,
        Some(Prompt::UnsavedChanges { tab: 1 }),
        "the dirty tab is asked about"
    );

    actions::answer_unsaved_save(&mut shell);

    assert_eq!(
        std::fs::read_to_string(&kept).expect("the file"),
        "original edited",
        "the edits reached the disk before the tab changed hands"
    );
    assert_eq!(shell.tabs[1].text(), "other");
    assert_eq!(shell.tabs.len(), 2, "a replacement, not a third tab");
}

#[test]
fn a_file_already_open_is_focused_whatever_the_setting_says() {
    // NC-F012 comes first: a file is never open twice, so the setting has nothing
    // to decide when the document is already on screen (docs/ui-reference.md §4).
    let scratch = Scratch::new("open-replace-duplicate");
    let one = scratch.file("one.txt", b"one");
    let two = scratch.file("two.txt", b"two");

    let mut shell = Shell::new();
    actions::open_path(&mut shell, &one, None);
    actions::open_path(&mut shell, &two, None);
    shell.opening = OpenBehavior::ReplaceTab;

    actions::open_path(&mut shell, &one, None);

    assert_eq!(shell.tabs.len(), 3, "nothing was replaced");
    assert_eq!(shell.active, 1, "the tab that has the file is shown");
    assert_eq!(
        shell.tabs[2].text(),
        "two",
        "the active tab kept its document"
    );
}

// ---- NC-T005 / NC-F009 — close all and exit: "Prompt per dirty doc" ---------

#[test]
fn closing_the_window_walks_each_dirty_document_in_turn_and_then_closes() {
    let scratch = Scratch::new("close-all");
    let kept = scratch.file("kept.txt", b"clean\r\n");

    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&kept).expect("the file opens");
    dirty_tab(&mut shell, "first unsaved");
    dirty_tab(&mut shell, "second unsaved");

    actions::close_window(&mut shell);
    assert_eq!(
        shell.prompt,
        Some(Prompt::UnsavedChanges { tab: 1 }),
        "the first dirty document is asked about first"
    );

    actions::answer_unsaved_dont_save(&mut shell);
    assert_eq!(
        shell.prompt,
        Some(Prompt::UnsavedChanges { tab: 1 }),
        "then the next, at its new index"
    );
    assert_eq!(shell.tabs[1].text(), "second unsaved");

    actions::answer_unsaved_dont_save(&mut shell);
    assert!(shell.prompt.is_none(), "nothing is left to ask about");
    assert!(shell.exit_pending, "and the window may close");
    assert!(!shell.closing_window);
}

#[test]
fn cancelling_part_way_stops_the_close_and_keeps_everything() {
    let mut shell = Shell::new();
    dirty_tab(&mut shell, "one");
    dirty_tab(&mut shell, "two");

    actions::close_window(&mut shell);
    assert!(shell.prompt.is_some());

    actions::answer_cancel(&mut shell);

    assert!(shell.prompt.is_none());
    assert!(
        !shell.exit_pending,
        "a cancelled close does not close the window"
    );
    assert!(!shell.closing_window);
    assert_eq!(shell.tabs.len(), 3, "and no document went away");
    assert_eq!(shell.tabs[1].text(), "one");
    assert_eq!(shell.tabs[2].text(), "two");
}

#[test]
fn saving_from_the_close_prompt_writes_the_file_and_continues_the_close() {
    let scratch = Scratch::new("close-save");
    let kept = scratch.file("kept.txt", b"before\r\n");

    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&kept).expect("the file opens");
    append(&mut shell.tabs[0], " after");
    assert!(shell.tabs[0].is_dirty());

    actions::close_window(&mut shell);
    assert_eq!(shell.prompt, Some(Prompt::UnsavedChanges { tab: 0 }));

    actions::answer_unsaved_save(&mut shell);

    assert_eq!(
        std::fs::read(&kept).expect("the file"),
        b"before\r\n after",
        "answering Save writes the document, in its own line endings"
    );
    assert!(
        shell.exit_pending,
        "and the close continues once nothing is dirty"
    );
}

#[test]
fn exit_closes_a_window_with_nothing_dirty_and_prompts_when_something_is() {
    let mut clean = Shell::new();
    let clean_ctx = egui::Context::default();
    actions::run(
        &mut clean,
        &clean_ctx,
        nc_commands::CommandId("cmd.file.exit"),
    );
    assert!(
        clean.prompt.is_none(),
        "a clean window is asked about by nobody"
    );
    assert!(
        asked_to_close(&clean_ctx),
        "and the window is told to close (NC-F009)"
    );
    assert!(
        !clean.exit_pending,
        "the pump spent the request while sending it, rather than leaving it pending"
    );

    let mut dirty = Shell::new();
    dirty_tab(&mut dirty, "unsaved");
    let dirty_ctx = egui::Context::default();
    actions::run(
        &mut dirty,
        &dirty_ctx,
        nc_commands::CommandId("cmd.file.exit"),
    );
    assert_eq!(dirty.prompt, Some(Prompt::UnsavedChanges { tab: 1 }));
    assert!(
        !asked_to_close(&dirty_ctx),
        "a dirty document holds the close until it is answered"
    );
}

// ---- the exit criterion, end to end ------------------------------------------

#[test]
fn a_work_session_survives_a_restart_without_losing_unsaved_work() {
    let scratch = Scratch::new("exit-criterion");
    let saved = scratch.file("saved.md", b"# Saved\r\n");
    let untouched = scratch.file("untouched.txt", b"untouched\r\n");

    // A working set: one saved document, one untouched file, one document with
    // work that exists only in the shell, and a second window's worth of tabs
    // switched to.
    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&saved).expect("the file opens");
    append(&mut shell.tabs[0], " edit");
    shell.open_tab(DocKind::Plain);
    shell.tabs[1] = Tab::open(&untouched).expect("the file opens");
    dirty_tab(&mut shell, "draft never written anywhere");
    shell.active = 1;

    let mut restored = restart(&scratch, &shell);

    assert_eq!(restored.tabs.len(), 3);
    assert_eq!(restored.active, 1);
    assert_eq!(
        restored.tabs[1].text(),
        "untouched\n",
        "the closing terminator is part of the text, and makes an empty last line"
    );
    assert!(!restored.tabs[1].is_dirty());
    assert_eq!(restored.tabs[0].text(), "# Saved\n edit");
    assert!(restored.tabs[0].is_dirty());
    assert_eq!(restored.tabs[2].text(), "draft never written anywhere");

    // A tab operation after the restart behaves the way it did before it.
    actions::close_tab(&mut restored, 2);
    assert_eq!(
        restored.prompt,
        Some(Prompt::UnsavedChanges { tab: 2 }),
        "the restored document is still known to be unsaved"
    );
    actions::answer_cancel(&mut restored);
    assert_eq!(restored.tabs[2].text(), "draft never written anywhere");

    // And writing the dirty ones back costs nothing to the clean one.
    restored.tabs[0].file.save().expect("a save");
    assert_eq!(
        std::fs::read(&saved).expect("the file"),
        b"# Saved\r\n edit"
    );
    assert_eq!(
        std::fs::read(&untouched).expect("the file"),
        b"untouched\r\n",
        "a document nobody touched is left byte for byte alone"
    );
}

// ---- NC-D008 — outside changes: "Detects rewrite/removal; prompt (§9)" ------

/// Runs one frame with the window focused or not, so a gain of focus can be
/// driven the way the windowing layer reports one.
fn focus_frame(shell: &mut Shell, focused: bool) {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1180.0, 720.0),
        )),
        focused,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| actions::pump(shell, ui.ctx()));
    output.textures_delta.clear();
}

/// A shell showing `path`, already read, with `theirs` written over it.
fn changed_underneath(path: &std::path::Path) -> Shell {
    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(path).expect("the file opens");
    std::fs::write(path, b"theirs").expect("an outside write");
    shell
}

#[test]
fn a_file_changed_on_disk_raises_the_outside_change_question() {
    let scratch = Scratch::new("outside-asks");
    let path = scratch.file("note.txt", b"one");
    let mut shell = changed_underneath(&path);

    actions::check_outside_changes(&mut shell);

    assert_eq!(
        shell.prompt,
        Some(Prompt::OutsideChange {
            tab: 0,
            path: path.clone()
        })
    );
    assert_eq!(shell.tabs[0].text(), "one", "nothing is applied by asking");
}

#[test]
fn the_question_is_asked_when_focus_comes_back_not_every_frame() {
    let scratch = Scratch::new("outside-focus");
    let path = scratch.file("note.txt", b"one");
    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&path).expect("the file opens");

    // A window that keeps the foreground does not re-read its files each frame.
    focus_frame(&mut shell, true);
    std::fs::write(&path, b"theirs").expect("an outside write");
    focus_frame(&mut shell, true);
    assert_eq!(shell.prompt, None, "no focus was gained");

    // Losing focus and coming back is when another program has had its turn.
    focus_frame(&mut shell, false);
    focus_frame(&mut shell, true);
    assert_eq!(
        shell.prompt,
        Some(Prompt::OutsideChange {
            tab: 0,
            path: path.clone()
        })
    );
}

#[test]
fn a_question_or_dialog_already_up_is_not_talked_over() {
    let scratch = Scratch::new("outside-quiet");
    let path = scratch.file("note.txt", b"one");
    let mut shell = changed_underneath(&path);
    shell.prompt = Some(Prompt::UnsavedChanges { tab: 0 });

    actions::check_outside_changes(&mut shell);
    assert_eq!(
        shell.prompt,
        Some(Prompt::UnsavedChanges { tab: 0 }),
        "the question on screen stands"
    );

    shell.prompt = None;
    shell.file_dialog = Some(crate::ui::state::FileDialog::open());
    actions::check_outside_changes(&mut shell);
    assert!(shell.prompt.is_none(), "a dialog in progress is left alone");
}

#[test]
fn reload_takes_the_bytes_on_disk_and_the_editor_follows() {
    let scratch = Scratch::new("outside-reload");
    let path = scratch.file("note.txt", b"one");
    let mut shell = changed_underneath(&path);
    actions::check_outside_changes(&mut shell);

    actions::answer_outside_change_reload(&mut shell);

    assert!(shell.prompt.is_none());
    assert_eq!(shell.tabs[0].text(), "theirs");
    assert!(!shell.tabs[0].is_dirty());
    assert_eq!(
        shell.tabs[0].file.external_change(),
        nc_textproc::ExternalChange::None
    );
}

#[test]
fn keeping_the_version_keeps_the_text_and_makes_it_unsaved() {
    let scratch = Scratch::new("outside-keep");
    let path = scratch.file("note.txt", b"one");
    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&path).expect("the file opens");
    append(&mut shell.tabs[0], " mine");
    shell.tabs[0].file.save().expect("a save");
    std::fs::write(&path, b"theirs").expect("an outside write");
    actions::check_outside_changes(&mut shell);

    actions::answer_outside_change_keep(&mut shell);

    assert!(shell.prompt.is_none());
    assert_eq!(shell.tabs[0].text(), "one mine", "the user's text stands");
    assert!(
        shell.tabs[0].is_dirty(),
        "and is unsaved work against the file"
    );
    assert_eq!(
        shell.tabs[0].file.external_change(),
        nc_textproc::ExternalChange::None,
        "the same change does not ask again"
    );

    shell.tabs[0].file.save().expect("a save");
    assert_eq!(std::fs::read(&path).expect("the file"), b"one mine");
}

#[test]
fn cancelling_the_outside_change_question_changes_nothing() {
    let scratch = Scratch::new("outside-cancel");
    let path = scratch.file("note.txt", b"one");
    let mut shell = changed_underneath(&path);
    actions::check_outside_changes(&mut shell);

    actions::answer_cancel(&mut shell);

    assert!(shell.prompt.is_none());
    assert_eq!(shell.tabs[0].text(), "one");
    assert!(!shell.tabs[0].is_dirty());
    assert_eq!(
        shell.tabs[0].file.external_change(),
        nc_textproc::ExternalChange::Modified,
        "the change has not been resolved, so the next focus gain asks again"
    );

    actions::check_outside_changes(&mut shell);
    assert!(matches!(
        shell.prompt,
        Some(Prompt::OutsideChange { tab: 0, .. })
    ));
}

#[test]
fn a_restart_does_not_report_a_change_that_predates_it() {
    let scratch = Scratch::new("outside-restart");
    let path = scratch.file("note.txt", b"one");
    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&path).expect("the file opens");
    append(&mut shell.tabs[0], " mine");
    assert!(shell.tabs[0].is_dirty());

    let mut restored = restart(&scratch, &shell);
    assert_eq!(restored.tabs[0].text(), "one mine");

    actions::check_outside_changes(&mut restored);
    assert!(
        restored.prompt.is_none(),
        "the file is exactly as the restart found it"
    );

    // A write after the restart is another program's doing, and is asked about.
    std::fs::write(&path, b"theirs").expect("an outside write");
    actions::check_outside_changes(&mut restored);
    assert!(matches!(
        restored.prompt,
        Some(Prompt::OutsideChange { tab: 0, .. })
    ));
}

// One boundary worth naming: this suite drives [`save_to`] and
// [`session::load_from`] with an explicit path rather than the shell's real
// `save`/`restore_shell` pair, because those resolve the user's configuration
// directory and the only override is `NOTECRAFT_CONFIG_DIR` — a process-wide
// variable that tests running in parallel must not set. The serialisation,
// restart, and restore behaviour above is the same code either way; what is left
// unverified here is the directory lookup itself.
