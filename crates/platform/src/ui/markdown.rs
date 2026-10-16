//! M7 acceptance suite — Markdown formatting through the shell.
//!
//! `ROADMAP.md` M7 asks that *formatting operations, Markdown serialization, and
//! view transitions pass their acceptance tests*. This module settles the first
//! two of those three, and does it through the shell rather than through
//! `nc-markdown` directly: every assertion below is what
//! [`actions::run`] leaves in the document, which is the same function a menu
//! click and a key chord reach. That is what makes these shell-level acceptance
//! tests rather than the engine's own unit tests one level up — they prove a user
//! can actually cause the bytes, not only that the engine can compute them.
//!
//! What it cannot settle is the third part of that criterion. `cmd.view.md_source`
//! and `cmd.view.md_formatted` set `Shell::formatted_view`, and nothing in the
//! editor reads it: the toggle moves a check mark and renders the same text either
//! way, so `NC-M002` ("No content/selection loss") has nothing to test yet. That
//! is also why `LANDED_MILESTONE` stays at 6 — the wiring guard counts a dispatch
//! arm as wired, and bumping it would have that test certify an inert command.

use std::path::PathBuf;

use crate::ui::actions;
use crate::ui::state::{DocKind, Shell, Tab};
use nc_commands::CommandId;

/// A scratch directory, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("notecraft-m7-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        Self(directory)
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

/// A Markdown tab holding `text`, with the caret at its end.
fn markdown(text: &str) -> Shell {
    let mut shell = Shell::new();
    shell.open_tab(DocKind::Markdown);
    let tab = shell.active_tab_mut().expect("the new tab");
    tab.document_mut().insert_str(text);
    shell
}

/// A Markdown tab holding `text` with `selected` selected.
fn selected(text: &str, chosen: &str) -> Shell {
    let mut shell = markdown(text);
    let start = text.find(chosen).expect("the selection is present");
    let tab = shell.active_tab_mut().expect("the tab");
    tab.document_mut().select(start, start + chosen.len());
    shell
}

/// A Markdown tab holding `text` with the whole document selected.
fn selected_all(text: &str) -> Shell {
    let mut shell = markdown(text);
    let length = text.len();
    let tab = shell.active_tab_mut().expect("the tab");
    tab.document_mut().select(0, length);
    shell
}

/// Fires a command the way a menu item or a key chord does.
fn fire(shell: &mut Shell, id: &'static str) {
    actions::run(shell, &egui::Context::default(), CommandId(id));
}

/// The active document's text.
fn text(shell: &Shell) -> &str {
    shell.active_tab().expect("a tab").text()
}

/// The active selection, as ordered byte offsets.
fn range(shell: &mut Shell) -> (usize, usize) {
    let tab = shell.active_tab_mut().expect("a tab");
    let selection = tab.document_mut().selection();
    (selection.start(), selection.end())
}

/// The byte range `needle` occupies in `haystack` — how these tests name a
/// selection without hard-coding offsets.
fn span(haystack: &str, needle: &str) -> (usize, usize) {
    let start = haystack
        .find(needle)
        .expect("the text is present in the document");
    (start, start + needle.len())
}

/// The undo depth of the active document.
fn depth(shell: &mut Shell) -> usize {
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .undo_depth()
}

/// Whether a path opens as a Markdown document.
fn opens_as_markdown(scratch: &Scratch, name: &str) -> bool {
    let path = scratch.file(name, b"text");
    Tab::open(&path).expect("the file opens").kind == DocKind::Markdown
}

// ---- NC-M003 — Bold / Italic: "Serialization exact" --------------------------

#[test]
fn bold_writes_exactly_two_asterisks_into_the_document() {
    let mut shell = selected("press the button", "button");
    fire(&mut shell, "cmd.md.bold");

    assert_eq!(text(&shell), "press the **button**");
}

#[test]
fn italic_writes_exactly_one_asterisk() {
    let mut shell = selected("make it so", "so");
    fire(&mut shell, "cmd.md.italic");

    assert_eq!(text(&shell), "make it *so*");
}

#[test]
fn the_selection_ends_up_covering_the_markers() {
    // This is what makes one more press the way back: the second press finds its
    // markers inside the selection and takes them off.
    let mut shell = selected("press the button", "button");
    fire(&mut shell, "cmd.md.bold");

    let marked = text(&shell).to_owned();
    assert_eq!(range(&mut shell), span(&marked, "**button**"));

    fire(&mut shell, "cmd.md.bold");
    assert_eq!(text(&shell), "press the button", "one press each way");
    assert_eq!(range(&mut shell), span("press the button", "button"));
}

#[test]
fn a_caret_gets_the_markers_and_the_room_to_type_between_them() {
    let mut shell = markdown("press the button");
    let inside = text(&shell).find("button").unwrap() + 3;
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .set_caret(inside);

    fire(&mut shell, "cmd.md.bold");

    assert_eq!(text(&shell), "press the but****ton");
    let (_, caret) = range(&mut shell);
    let marked = text(&shell).to_owned();
    assert_eq!(&marked[caret - 2..caret], "**", "the caret is inside");
    assert_eq!(&marked[caret..caret + 2], "**", "the markers, not before");
}

// ---- NC-M004 — Strikethrough: "`~~` serialization" ---------------------------

#[test]
fn strikethrough_writes_exactly_two_tildes() {
    let mut shell = selected("keep the old value", "old value");
    fire(&mut shell, "cmd.md.strike");

    assert_eq!(text(&shell), "keep the ~~old value~~");
}

// ---- NC-M010 — undo/redo: "Single-step undo" ----------------------------------

#[test]
fn one_formatting_command_costs_exactly_one_undo_step() {
    let mut shell = selected("press the button", "button");
    let before = depth(&mut shell);
    let original = text(&shell).to_owned();

    fire(&mut shell, "cmd.md.bold");

    assert_eq!(
        depth(&mut shell) - before,
        1,
        "the format is one edit, however many markers it inserted"
    );
    assert!(
        shell.active_tab().expect("a tab").is_dirty(),
        "and the tab knows it changed"
    );

    let tab = shell.active_tab_mut().expect("a tab");
    assert!(tab.document_mut().undo(), "one undo takes it back");
    assert_eq!(tab.text(), original);
}

#[test]
fn redo_bring_the_format_back() {
    let mut shell = selected("press the button", "button");
    fire(&mut shell, "cmd.md.bold");
    let marked = text(&shell).to_owned();

    let tab = shell.active_tab_mut().expect("a tab");
    tab.document_mut().undo();
    assert!(tab.document_mut().redo());
    assert_eq!(tab.text(), marked);
}

#[test]
fn a_command_that_changes_nothing_is_not_recorded_as_an_edit() {
    // Outdenting a line that has no indent to take: the engine has nothing to do,
    // and doing "nothing" must not leave an undo step or a dirty flag behind. The
    // document is opened from a file so that "still clean" means what it says.
    let scratch = Scratch::new("no-op");
    let path = scratch.file("plain.md", b"nothing marked here");
    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&path).expect("the file opens");
    shell.active = 0;
    let before = depth(&mut shell);

    fire(&mut shell, "cmd.md.indent_decrease");

    assert_eq!(text(&shell), "nothing marked here");
    assert_eq!(depth(&mut shell), before, "a no-op is not an undo step");
    assert!(
        !shell.active_tab().expect("a tab").is_dirty(),
        "and it does not mark the document unsaved"
    );
}

// ---- NC-M005 — Headings: "ATX output" ----------------------------------------

#[test]
fn the_paragraph_styles_write_atx_in_the_order_the_dropdown_lists_them() {
    // The reference's §3 style list is the only thing that says what these do, and
    // it gives one style per ATX level, in order, with Body as plain text.
    let expected = [
        "# Title",
        "## Title",
        "### Title",
        "#### Title",
        "##### Title",
        "###### Title",
    ];
    for (index, id) in [
        "cmd.format.title",
        "cmd.format.subtitle",
        "cmd.format.heading",
        "cmd.format.subheading",
        "cmd.format.section",
        "cmd.format.subsection",
    ]
    .into_iter()
    .enumerate()
    {
        let mut shell = selected("Title", "Title");
        fire(&mut shell, id);
        assert_eq!(text(&shell), expected[index], "{id}");
    }
}

#[test]
fn asking_for_a_second_style_replaces_the_first_rather_than_stacking() {
    let mut shell = selected("Title", "Title");
    fire(&mut shell, "cmd.format.title");
    fire(&mut shell, "cmd.format.subsection");

    assert_eq!(text(&shell), "###### Title");
}

#[test]
fn asking_for_the_style_the_line_already_carries_makes_it_body_text() {
    let mut shell = selected("Title", "Title");
    fire(&mut shell, "cmd.format.title");
    fire(&mut shell, "cmd.format.title");

    assert_eq!(text(&shell), "Title");
}

#[test]
fn body_takes_a_heading_off() {
    let mut shell = selected("## Chapter", "Chapter");
    fire(&mut shell, "cmd.format.body");

    assert_eq!(text(&shell), "Chapter");
}

#[test]
fn the_paragraph_dropdown_fires_the_commands_the_menu_uses() {
    // The dropdown used to set the tab's style itself, which meant the toolbar and
    // the menu did different things. Both now go through the registry, and this
    // pins that the style list only names commands that exist.
    for style in crate::ui::state::ParagraphStyle::ALL {
        assert!(
            actions::handles(style.command()),
            "{} has no command behind it",
            style.label()
        );
    }
}

// ---- NC-M006 — Lists, nesting, indentation: "Nested indent correct" ----------

#[test]
fn bullets_mark_every_line_the_selection_touches() {
    let mut shell = selected_all("one\ntwo\nthree");
    fire(&mut shell, "cmd.md.bullets");

    assert_eq!(text(&shell), "- one\n- two\n- three");
}

#[test]
fn a_caret_marks_only_its_own_line() {
    // The counterpart to the test above, since "the lines the selection touches"
    // has to mean something when there is no selection: the caret's line.
    let mut shell = markdown("one\ntwo\nthree");
    fire(&mut shell, "cmd.md.bullets");

    assert_eq!(text(&shell), "one\ntwo\n- three");
}

#[test]
fn numbering_numbers_from_one_across_the_selection() {
    let mut shell = selected_all("one\ntwo\nthree");
    fire(&mut shell, "cmd.md.numbering");

    assert_eq!(text(&shell), "1. one\n2. two\n3. three");
}

#[test]
fn indenting_nests_a_line_by_one_level_and_outdenting_undoes_it() {
    let mut shell = markdown("- one\n- two\n");
    let on_two = text(&shell).find("- two").unwrap();
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .set_caret(on_two);

    fire(&mut shell, "cmd.md.indent_increase");
    assert_eq!(text(&shell), "- one\n  - two\n", "two spaces per level");

    fire(&mut shell, "cmd.md.indent_decrease");
    assert_eq!(text(&shell), "- one\n- two\n");
}

#[test]
fn a_nested_item_keeps_its_indent_when_its_marker_changes() {
    let mut shell = markdown("- one\n  - two\n");
    let on_two = text(&shell).find("- two").unwrap();
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .set_caret(on_two);

    fire(&mut shell, "cmd.md.numbering");

    assert_eq!(
        text(&shell),
        "- one\n  1. two\n",
        "the indent is not the marker"
    );
}

#[test]
fn the_list_dropdown_fires_the_registry_commands() {
    for id in [
        "cmd.md.bullets",
        "cmd.md.numbering",
        "cmd.md.indent_increase",
        "cmd.md.indent_decrease",
    ] {
        assert!(actions::handles(CommandId(id)), "{id} is unreachable");
    }
}

// ---- NC-M007 — Hyperlinks: "Link syntax exact" -------------------------------

#[test]
fn the_hyperlink_command_wraps_the_selection_in_link_syntax() {
    let mut shell = selected("see the docs", "the docs");
    fire(&mut shell, "cmd.md.link");

    assert_eq!(text(&shell), "see [the docs]()");
    let marked = text(&shell).to_owned();
    assert_eq!(
        range(&mut shell),
        span(&marked, "[the docs]()"),
        "the selection covers the syntax"
    );
}

#[test]
fn a_caret_gets_an_empty_link_to_fill_in() {
    let mut shell = markdown("see docs");
    let at = text(&shell).find("docs").unwrap();
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .set_caret(at);

    fire(&mut shell, "cmd.md.link");

    assert_eq!(text(&shell), "see []()docs");
    let (start, _) = range(&mut shell);
    assert_eq!(start, at + 1, "the caret is inside the brackets");
}

// ---- NC-M008 — Tables: "Insert/expand semantics" -----------------------------

#[test]
fn the_table_command_writes_a_header_a_separator_and_the_body() {
    let mut shell = markdown("");
    fire(&mut shell, "cmd.md.table");

    let lines: Vec<&str> = text(&shell).lines().collect();
    assert_eq!(lines.len(), 4, "header, separator, two body rows");
    assert_eq!(
        lines[1], "| --- | --- | --- |",
        "GFM reads row two as the delimiter"
    );
    for line in &lines {
        assert_eq!(
            line.chars().filter(|c| *c == '|').count(),
            4,
            "every row has the same three cells: {line:?}"
        );
    }
}

#[test]
fn inserting_a_table_leaves_the_rest_of_the_document_in_place() {
    let mut shell = markdown("before\n\nafter\n");
    let at = text(&shell).find("after").unwrap();
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .set_caret(at);

    fire(&mut shell, "cmd.md.table");

    assert!(text(&shell).starts_with("before\n\n"));
    assert!(text(&shell).ends_with("after\n"));
}

// ---- NC-M009 — Clear formatting: "Result" ------------------------------------

#[test]
fn clear_formatting_takes_the_markers_and_leaves_the_words() {
    let mut shell = selected_all("**important** news");
    fire(&mut shell, "cmd.md.clear");

    assert_eq!(text(&shell), "important news");
}

#[test]
fn clear_formatting_takes_block_markers_with_them() {
    let mut shell = selected_all("## Heading\n- item\n> quoted");
    fire(&mut shell, "cmd.md.clear");

    assert_eq!(text(&shell), "Heading\nitem\nquoted");
}

#[test]
fn clear_formatting_takes_a_carets_own_heading_but_not_its_inline_markers() {
    // The two halves of the command follow the rules their kind allows: a block
    // marker belongs to a line, so the caret's line is enough; an inline marker
    // has to be inside the selection to be removed.
    let mut shell = markdown("## **Heading**");
    fire(&mut shell, "cmd.md.clear");

    assert_eq!(text(&shell), "**Heading**");
}

// ---- NC-M011: "No silent normalization", and NC-M001 -------------------------

#[test]
fn a_plain_document_refuses_markdown_commands() {
    // Availability greys these for a plain document and the keyboard will not
    // dispatch a greyed command, but a handler that wrote `**` into a .txt file
    // would corrupt text nobody asked to have marked up. Opened from a file, so a
    // stray edit would show as a dirty tab as well as changed text.
    let scratch = Scratch::new("plain-refuses");
    let path = scratch.file("notes.txt", b"press the button");
    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&path).expect("the file opens");
    shell.active = 0;
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .select(10, 16);
    let before = depth(&mut shell);

    fire(&mut shell, "cmd.md.bold");

    assert_eq!(text(&shell), "press the button");
    assert_eq!(depth(&mut shell), before, "and no undo step for a non-edit");
    assert!(!shell.active_tab().expect("a tab").is_dirty());
}

#[test]
fn a_markdown_extension_decides_the_document_kind() {
    let scratch = Scratch::new("extensions");

    for name in ["notes.md", "notes.markdown", "NOTES.MD", "Notes.MarkDown"] {
        assert!(
            opens_as_markdown(&scratch, name),
            "{name} should open as Markdown"
        );
    }
    for name in ["notes.txt", "notes.mdown", "notes.md.txt", "readme"] {
        assert!(
            !opens_as_markdown(&scratch, name),
            "{name} should open as plain text"
        );
    }
}

#[test]
fn markdown_commands_are_available_for_a_markdown_document_only() {
    // The other half of the gate above: what the menus grey out.
    let mut shell = markdown("some text");
    assert!(shell.availability().markdown_enabled);

    shell.open_tab(DocKind::Plain);
    assert!(
        !shell.availability().markdown_enabled,
        "a plain tab greys the Markdown menu"
    );
}

#[test]
fn a_markdown_file_survives_being_opened_and_saved_byte_for_byte() {
    let scratch = Scratch::new("roundtrip");
    // Constructs this editor does not implement: a footnote, raw HTML, a fence,
    // and a highlight. CRLF too, since a Markdown file arriving from Windows must
    // not come back with its line endings translated. None of them may be
    // rewritten on the way out.
    let source = b"[^1]: a footnote\r\n<div class=\"note\">\r\n```fenced\r\n==lit==\r\n";
    let path = scratch.file("weird.md", source);

    let mut shell = Shell::new();
    shell.tabs[0] = Tab::open(&path).expect("the file opens");
    shell.active = 0;
    // The same save Ctrl+S reaches for a document that already has a path.
    shell.tabs[0].file.save().expect("the save succeeds");

    assert_eq!(
        std::fs::read(&path).expect("the file reads back"),
        source,
        "saving rewrote Markdown this editor does not understand"
    );
    assert!(
        !shell.tabs[0].is_dirty(),
        "and the tab knows it is saved again"
    );
}

#[test]
fn formatting_a_line_leaves_unsupported_syntax_on_other_lines_alone() {
    let mut shell = markdown("[^1]: a footnote\n\nsee the docs\n");
    let at = text(&shell).find("the docs").unwrap();
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .select(at, at + "the docs".len());

    fire(&mut shell, "cmd.md.bold");

    assert_eq!(text(&shell), "[^1]: a footnote\n\nsee **the docs**\n");
}
