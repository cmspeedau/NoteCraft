# Feature Matrix — Implementation & Verification Status

Source of truth for **implementation and verification status** (AGENT.md §11).
The UI reference lives in [`ui-reference.md`](ui-reference.md); the command
surface lives in `crates/commands`.

**Status values:** `NOT_STARTED` · `SPECIFIED` · `IN_PROGRESS` · `IMPLEMENTED` ·
`VERIFIED` · `BLOCKED` · `OUT_OF_SCOPE`

`IMPLEMENTED` means the code exists; `VERIFIED` means its acceptance tests pass.

**Current phase: M2 shell + M3–M6 engines, with the accelerators wired.** The
egui/eframe shell draws the window, menus, toolbar, editor, status bar, and
settings page; the `nc-core` and `nc-textproc` engines back editing, file I/O,
and search. Every command this build implements now has a live keyboard
accelerator, dispatched from the registry (`crates/platform/src/ui/input.rs`),
and the Edit commands (undo/redo, select all, delete) and the F5 time/date
insertion are routed through the shell's document rather than the editor widget.
Cut, copy and paste reuse `egui`'s clipboard events — the windowing layer turns
`Ctrl+X`/`Ctrl+C`/`Ctrl+V` into those events before a key press exists — and the
menu entries request the same events, so the menu and the keyboard agree.
`has_selection` is now reported from the document, so Cut/Copy/Delete enable with
a selection. M6 core has begun: `Ctrl+Tab`/`Ctrl+Shift+Tab` switch tabs
cyclically, `File ▸ Save all` writes every dirty path-backed document, a file
whose canonical path is already open focuses its tab instead of opening twice
(NC-F012, including command-line arguments), and `File ▸ New window` opens an
independent window with its own document set (`crates/platform/src/ui/windows.rs`).
The Find/Replace, Go-to, and Open/Save-as dialogs are built and their commands
wired, with the unsaved-changes, overwrite, and unrepresentable prompts. Session
restore (NC-F011) now writes the open documents — including unsaved text — to the
configuration directory on exit and reopens them on the next start when no files
are named on the command line (`crates/platform/src/ui/session.rs`). `File ▸
Recent` lists the documents opened so far, newest first, and choosing one opens
it through the same path as `File ▸ Open`, so a document already open is focused
rather than opened twice (`NC-F010`, `NC-F012`). M7 is closed: `nc-markdown` holds the
formatting model, the shell dispatches every `cmd.md.*` and `cmd.format.*` command
against it, and `View ▸ Markdown` really does switch a document between its source and a
rendering of it — so bold, italic, strikethrough, headings, lists, nesting, links,
tables (writing one and growing one), clear formatting and the view toggle all change
what the user sees, from the toolbar, the menus and the shortcuts alike. The one piece
deliberately left undone is widening a table, which nothing in the capture would fire
(see the verification summary). The page-setup surface is
still ahead. Seven M6 behaviour criteria are
`VERIFIED` by an acceptance suite (`crates/platform/src/ui/lifecycle.rs`) and the
dirty marker by its painted shapes (`crates/platform/src/ui/appearance.rs`); the
remaining visual criteria and the tab context menu are not — see the verification
summary for exactly which rows rest on what. Nothing is `VERIFIED` until its
acceptance tests pass end to end.

Known limitation: `File ▸ Recent` is scoped to what the shell has open — a
document opened and then closed drops off the list, and a restart rebuilds it
from the restored tabs rather than from a history of its own (`NC-F010`). The
session does not keep each tab's undo history or caret either — the shell tracks
neither per tab, so a restored document starts with a fresh history (NC-F011).
The F5 time/date insertion writes **UTC**, because the standard
library exposes no local time zone and the project carries no time-zone database
(NC-S007; to revisit with the M8 localisation work).

---

## A. File & document lifecycle

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-F001 | New empty document (Ctrl+N) | MANDATORY | M4 | SPECIFIED | VERIFIED | UTF-8 (no BOM), CRLF, empty, in a new tab | `crates/platform/src/ui/lifecycle.rs` |
| NC-F002 | New tab (Ctrl+N) | MANDATORY | M6 | SPECIFIED | VERIFIED | Adds a tab without disturbing others | `crates/platform/src/ui/lifecycle.rs` |
| NC-F003 | New window (Ctrl+Shift+N) | MANDATORY | M6 | SPECIFIED | IMPLEMENTED | Independent top-level window | `crates/platform/src/ui/windows.rs` |
| NC-F004 | Open file (Ctrl+O) | MANDATORY | M4 | SPECIFIED | VERIFIED | Open dialog (§7); new tab; encoding/EOL detected | `crates/platform/src/ui/lifecycle.rs`, `crates/platform/src/ui/actions.rs` |
| NC-F005 | Save (Ctrl+S) | MANDATORY | M4 | SPECIFIED | VERIFIED | Byte-correct in-place save; no data loss | `crates/platform/src/ui/lifecycle.rs`, `crates/platform/src/ui/actions.rs`, `crates/textproc/src/textfile.rs` |
| NC-F006 | Save As + encoding (Ctrl+Shift+S) | MANDATORY | M4 | SPECIFIED | VERIFIED | Save-as dialog (§7); overwrite prompt (§9); chosen encoding applied | `crates/platform/src/ui/lifecycle.rs`, `crates/platform/src/ui/actions.rs` |
| NC-F007 | Print / Page setup (Ctrl+P) | OPTIONAL | M8 | SPECIFIED | NOT_STARTED | Page-setup dialog (§8); print pipeline | `tests/file_io/print.rs` |
| NC-F008 | Close tab (Ctrl+W) | MANDATORY | M6 | SPECIFIED | VERIFIED | Closes the active tab; last tab resets to untitled | `crates/platform/src/ui/lifecycle.rs` |
| NC-F009 | Exit (Alt+F4) | MANDATORY | M6 | SPECIFIED | VERIFIED | Closes the window | `crates/platform/src/ui/lifecycle.rs` |
| NC-F010 | Recent files | OPTIONAL | M6 | SPECIFIED | IMPLEMENTED | File ▸ Recent lists prior documents | `crates/platform/src/ui/state.rs` |
| NC-F011 | Session restore | MANDATORY | M6 | SPECIFIED | VERIFIED | Restores tabs incl. unsaved state | `crates/platform/src/ui/lifecycle.rs`, `…/ui/session.rs` |
| NC-F012 | Duplicate-open / document identity | MANDATORY | M6 | SPECIFIED | VERIFIED | New tab vs focus existing | `crates/platform/src/ui/lifecycle.rs` |

## B. Editing, selection, undo

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-E001 | Undo / Redo multilevel | MANDATORY | M3 | SPECIFIED | VERIFIED | Undo depth & grouping | `crates/core/tests/editing/undo.rs`, `crates/platform/src/ui/actions.rs` |
| NC-E002 | Cut / Copy / Paste | MANDATORY | M3 | SPECIFIED | VERIFIED | Clipboard text + EOL conversion | `crates/core/tests/editing/clipboard.rs`, `crates/platform/src/ui/actions.rs` |
| NC-E003 | Select all (Ctrl+A) | MANDATORY | M3 | SPECIFIED | VERIFIED | Whole-document selection | `crates/core/tests/keyboard/select_all.rs`, `crates/platform/src/ui/actions.rs` |
| NC-E004 | Caret navigation | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Word/line/paragraph boundaries | `crates/core/tests/editing/caret.rs` |
| NC-E005 | Mouse selection (click, drag, double, triple) | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Word/line selection rules | `crates/core/tests/editing/mouse.rs` |
| NC-E006 | Word wrap toggle | MANDATORY | M3 | SPECIFIED | VERIFIED | Soft wrap; no bytes changed | `crates/core/tests/editing/word_wrap.rs`, `crates/platform/src/ui/appearance.rs`, `crates/platform/src/ui/settings.rs` |
| NC-E007 | Tabs & indentation behavior | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Tab stop width, insertion | `crates/core/tests/editing/tabs.rs` |
| NC-E008 | Unicode: astral, combining, emoji, surrogate safety | MANDATORY | M3 | SPECIFIED | VERIFIED | No broken surrogate pairs on edit | `crates/core/tests/editing/unicode.rs`, `crates/platform/src/ui/editor.rs` |
| NC-E009 | Drag & drop text | OPTIONAL | M3 | SPECIFIED | IMPLEMENTED | Internal drag; drop target | `crates/core/tests/editing/dnd.rs` |
| NC-E010 | Status bar line/column calculation | MANDATORY | M5 | SPECIFIED | VERIFIED | 1-based line/col | `crates/platform/src/ui/editor.rs` |

## C. Find / replace / navigation

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-S001 | Find (Ctrl+F) | MANDATORY | M5 | SPECIFIED | VERIFIED | Find dialog (§5) | `crates/platform/src/ui/dialogs.rs`, `crates/platform/src/ui/mod.rs` |
| NC-S002 | Find next / previous (F3 / Shift+F3) | MANDATORY | M5 | SPECIFIED | VERIFIED | Direction + wrap semantics (§5) | `crates/textproc/tests/editing/find.rs`, `crates/platform/src/ui/actions.rs` |
| NC-S003 | Replace / Replace All (Ctrl+H) | MANDATORY | M5 | SPECIFIED | VERIFIED | Undo as a single action (§5) | `crates/textproc/tests/editing/replace.rs`, `crates/platform/src/ui/actions.rs` |
| NC-S004 | Search options (match case, whole word) | MANDATORY | M5 | SPECIFIED | VERIFIED | Option set (§5) | `crates/textproc/tests/editing/find_options.rs`, `crates/platform/src/ui/actions.rs` |
| NC-S005 | Wrap-around + boundary notices | MANDATORY | M5 | SPECIFIED | VERIFIED | Wrap option + status notices (§5) | `crates/textproc/tests/editing/find_boundary.rs`, `crates/platform/src/ui/actions.rs` |
| NC-S006 | Go to line (Ctrl+G) | OPTIONAL | M5 | SPECIFIED | VERIFIED | Dialog + clamp behavior (§6) | `crates/textproc/tests/editing/go_to.rs`, `crates/platform/src/ui/actions.rs`, `crates/platform/src/ui/mod.rs` |
| NC-S007 | Insert date/time (F5) | OPTIONAL | M5 | SPECIFIED | VERIFIED | Format string | `crates/textproc/tests/editing/time_date.rs`, `crates/platform/src/ui/mod.rs` |

## D. Encoding & file I/O data integrity

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-D001 | UTF-8 with & without BOM read/write | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Byte-preserving round-trip | `crates/textproc/tests/file_io/encoding_utf8.rs` |
| NC-D002 | UTF-16 LE/BE (+ UTF-32) read/write | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | BOM-correct round-trip | `crates/textproc/tests/file_io/encoding_utf16.rs` |
| NC-D003 | ANSI (Windows-1252) read/write | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Unrepresentable chars reported; prompt (§9) | `crates/textproc/tests/file_io/encoding_ansi.rs` |
| NC-D004 | EOL detect/preserve CRLF/LF/CR (incl. mixed) | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Bytes on save preserved | `crates/textproc/tests/file_io/eol.rs` |
| NC-D005 | Dirty-state tracking + unsaved prompts | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Unsaved-changes prompt states surfaced (§9) | `crates/textproc/tests/file_io/dirty.rs` |
| NC-D006 | Failed-write / read-only / permission errors | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Original file preserved (atomic write); error prompt (§9) | `crates/textproc/tests/file_io/errors.rs` |
| NC-D007 | Empty / malformed / very large files | MANDATORY | M4/M10 | SPECIFIED | IMPLEMENTED | No crash | `crates/textproc/tests/file_io/stress.rs` |
| NC-D008 | External file-change detection | OPTIONAL | M4 | SPECIFIED | VERIFIED | Detects rewrite/removal; outside-change prompt (§9) | `crates/textproc/tests/file_io/external_change.rs`, `crates/platform/src/ui/lifecycle.rs` |

## E. Tabs, session, lifecycle

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-T001 | Tab strip + add-tab button | MANDATORY | M6 | SPECIFIED | IMPLEMENTED | Geometry matches the reference | `crates/platform/src/ui/appearance.rs` (structural only) |
| NC-T002 | Tab switching Ctrl+Tab / Ctrl+Shift+Tab | MANDATORY | M6 | SPECIFIED | VERIFIED | Cyclic; works with N tabs | `crates/platform/src/ui/lifecycle.rs` |
| NC-T003 | Tab context menu | OPTIONAL | M6 | SPECIFIED | BLOCKED | Reference item set | blocked — see note below |
| NC-T004 | Dirty indicator on tab/title | MANDATORY | M6 | SPECIFIED | VERIFIED | Marker shown when dirty | `crates/platform/src/ui/appearance.rs` |
| NC-T005 | Close all | MANDATORY | M6 | SPECIFIED | VERIFIED | Prompt per dirty doc | `crates/platform/src/ui/lifecycle.rs` |

## F. Markdown / formatting (M7)

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-M001 | `.md`/`.markdown` association & handling | MANDATORY | M7 | SPECIFIED | VERIFIED | Correct path per extension | `crates/platform/src/ui/markdown.rs` |
| NC-M002 | Syntax view vs formatted view toggle | MANDATORY | M7 | SPECIFIED | VERIFIED | No content/selection loss | `crates/platform/src/ui/markdown.rs`, `crates/platform/src/ui/appearance.rs` |
| NC-M003 | Bold / Italic (Ctrl+B / Ctrl+I) | MANDATORY | M7 | SPECIFIED | VERIFIED | Serialization exact | `crates/markdown/tests/markdown/inline.rs`, `crates/platform/src/ui/markdown.rs` |
| NC-M004 | Strikethrough | MANDATORY | M7 | SPECIFIED | VERIFIED | `~~` serialization | `crates/markdown/tests/markdown/inline.rs`, `crates/platform/src/ui/markdown.rs` |
| NC-M005 | Headings | MANDATORY | M7 | SPECIFIED | VERIFIED | ATX output | `crates/markdown/tests/markdown/headings.rs`, `crates/platform/src/ui/markdown.rs` |
| NC-M006 | Ordered/unordered lists + nesting | MANDATORY | M7 | SPECIFIED | VERIFIED | Nested indent correct | `crates/markdown/tests/markdown/lists.rs`, `crates/platform/src/ui/markdown.rs` |
| NC-M007 | Hyperlinks | MANDATORY | M7 | SPECIFIED | VERIFIED | Link syntax exact | `crates/markdown/tests/markdown/links.rs`, `crates/platform/src/ui/markdown.rs` |
| NC-M008 | Tables | MANDATORY | M7 | SPECIFIED | VERIFIED | Insert/expand semantics | `crates/markdown/tests/markdown/tables.rs`, `crates/platform/src/ui/markdown.rs` |
| NC-M009 | Clear formatting | OPTIONAL | M7 | SPECIFIED | VERIFIED | Result | `crates/markdown/tests/markdown/clear.rs`, `crates/platform/src/ui/markdown.rs` |
| NC-M010 | Formatting participates in undo/redo | MANDATORY | M7 | SPECIFIED | VERIFIED | Single-step undo | `crates/platform/src/ui/markdown.rs` |
| NC-M011 | Malformed / unsupported syntax preserved | MANDATORY | M7 | SPECIFIED | VERIFIED | No silent normalization | `crates/markdown/tests/markdown/malformed.rs`, `crates/platform/src/ui/markdown.rs` |

## G. Settings & desktop integration (M8)

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-P001 | App theme (system/light/dark) | MANDATORY | M8 | SPECIFIED | VERIFIED | Dropdown shown; applies the theme | `crates/platform/src/ui/appearance.rs`, `crates/platform/src/ui/settings.rs`, `crates/platform/src/ui/theme.rs` |
| NC-P002 | Font family & size | MANDATORY | M8 | SPECIFIED | VERIFIED | Dropdown shown; applies to the editor | `crates/platform/src/ui/fonts.rs`, `crates/platform/src/ui/settings.rs`, `crates/platform/src/ui/appearance.rs` |
| NC-P003 | Word-wrap preference persistence | MANDATORY | M8 | SPECIFIED | VERIFIED | Survives restart | `crates/platform/src/ui/settings.rs` |
| NC-P004 | Spell check + autocorrect | MANDATORY | M8 | SPECIFIED (invented UI) | IMPLEMENTED | Squiggle, suggestions | `crates/textproc/src/spell.rs`, `crates/platform/src/ui/editor.rs`, `crates/platform/src/ui/appearance.rs`, `crates/platform/src/ui/lifecycle.rs`, `crates/platform/src/ui/settings.rs` |
| NC-P005 | Formatting enable/disable toggle | MANDATORY | M8 | SPECIFIED | VERIFIED | Toolbar hides when off | `crates/platform/src/ui/menu_bar.rs`, `crates/platform/src/ui/settings.rs`, `crates/platform/src/ui/appearance.rs` |
| NC-P006 | High-DPI scaling | MANDATORY | M2/M8 | SPECIFIED | IMPLEMENTED | Layout follows the display scale | egui runtime |
| NC-P007 | Accessibility / screen-reader support | MANDATORY | M8 | SPECIFIED | VERIFIED | Names, focus order | `crates/platform/src/ui/a11y.rs`, `crates/platform/src/ui/title_bar.rs`, `crates/platform/src/ui/menu_bar.rs`, `crates/platform/src/ui/editor.rs`, `crates/platform/src/ui/status_bar.rs`, `crates/platform/src/ui/settings.rs`, `crates/platform/src/ui/dialogs.rs` |
| NC-P008 | File association & shell integration | OPTIONAL | M8 | SPECIFIED | NOT_STARTED | Open-with, default app | `tests/file_io/assoc.rs` |
| NC-P009 | Command-line file args | MANDATORY | M1/M6 | SPECIFIED | IMPLEMENTED | Opens the named files | `crates/app/tests/cli.rs` |
| NC-P010 | Localization / keyboard-layout correctness | OPTIONAL | M8 | SPECIFIED | NOT_STARTED | Layouts correct | `tests/keyboard/layouts.rs` |

## H. AI / service-dependent (M9)

| ID | Feature | Priority | Milestone | Spec | Impl | Note |
| --- | --- | --- | --- | --- | --- | --- |
| NC-X001 | Rewrite (tone/length/format) | SERVICE | M9 | SPECIFIED | NOT_STARTED | Requires an account / service |
| NC-X002 | Summarize | SERVICE | M9 | SPECIFIED | NOT_STARTED | Requires service |
| NC-X003 | Write (generate from instruction) | SERVICE | M9 | SPECIFIED | NOT_STARTED | Requires service |
| NC-X004 | Streaming results | SERVICE | M9 | SPECIFIED | NOT_STARTED | Progressive display |
| NC-X005 | Service error / auth-failure / disabled states | SERVICE | M9 | SPECIFIED | NOT_STARTED | Greyed menu items today |

---

## Verification summary

| Metric | Value |
| --- | --- |
| Total tracked features | 68 |
| IMPLEMENTED | 32 |
| IN_PROGRESS | 0 |
| NOT_STARTED | 9 |
| VERIFIED | 26 |
| BLOCKED | 1 |

**What `VERIFIED` means here.** Forty-two rows are verified, by suites that drive the
real shell rather than a model of it, and by the Markdown engine's own suite — whose
criteria the shell suite then re-checks end to end.

`crates/platform/src/ui/lifecycle.rs` settles M6's **behaviour** criteria: what a
tab operation does to the documents, what each prompt asks and how each answer
behaves, and what a restart brings back — including byte tests that a clean file
is left untouched and an unsaved one comes back and saves in its own encoding and
line endings. That covers `NC-F002`, `NC-F008`, `NC-F009`, `NC-F011`, `NC-F012`,
`NC-T002`, `NC-T005`. `NC-F009` is verified against the observable the shell
actually controls — the `Close` viewport command it emits — not against an
internal flag that `run` consumes before a test could read it.

`crates/platform/src/ui/appearance.rs` settles `NC-T004` by painting the shell
headlessly and reading back the shapes: an accent dot of the marker's radius
appears on exactly the dirty tabs, on a clean document none appears, two dirty
documents make two dots, and answering the unsaved prompt clears the marker. The
assertions were mutation-checked — disabling the dot's paint condition fails four
of them, so they are not reading a shape list that would pass either way.

Three M6 rows stay `IMPLEMENTED`, and the reason is stated rather than glossed.
`NC-T001` reads "Geometry matches the reference", and `docs/ui-reference.md` gives
the tab strip no coordinates — `appearance.rs` now asserts the §1 claims that *are*
structural (one tab per document, laid out side by side, exactly one carrying the
accent underline, tabs within their stated 120..=220 width), but that is the
shell agreeing with itself, not a comparison against the capture, so the row does
not earn `VERIFIED` on that. Whether the shell *looks* like the screenshot still
needs a person looking at a running window. `NC-F003` needs a second real
top-level window, and `NC-F010`'s Recent submenu needs its drawing observed — its
list, ordering and capping are tested, the rendering is not.

**How the File rows are verified.** `NC-F004`, `NC-F005` and `NC-F006` were
`IMPLEMENTED` with the file dialog's *mechanics* settled in `actions.rs` — a name typed
into the dialog becomes a tab, Save As raises the overwrite and unrepresentable prompts
(which need no display, so a test can raise and answer them). What was missing was the
half the acceptance text actually names, proven by a user's own key: that Open
*detects* the encoding and line endings off the bytes rather than assuming UTF-8, that
Ctrl+S rewrites a file *in place* and leaves a clean document's bytes untouched, and that
Save As writes in the *chosen* encoding. `crates/platform/src/ui/lifecycle.rs` now drives
each through [`actions::run`](../crates/platform/src/ui/actions.rs) — the same call a menu
click and an accelerator reach, the accelerator bindings themselves pinned by the
registry's own lookup test — and reads the result off disk: a UTF-16LE-with-mark CRLF file
opens as a new tab carrying `Utf16Le` and `CrLf`, an edit saved with `cmd.file.save`
gains exactly the typed bytes and keeps the file's CRLF, and a document saved as UTF-16
comes back from disk with that encoding. Each was mutation-checked — forcing the open to
stop detecting, blanking the save dispatch, and forcing Save As to ignore the chosen
encoding each fail their test — so they are not asserting a code path that would pass
either way. `NC-F001` moved with them: a brand-new document — Ctrl+N's tab, typed into
and named — had never been driven to disk, so its default format (UTF-8, no byte order
mark, CRLF, empty to begin with) was only asserted in the core's arithmetic, not as an
outcome a user reaches; flipping the new-document line ending to LF fails the test. These
rows need no window and so, unlike the three `IMPLEMENTED` rows above, they can be settled
here: the dialog is a `egui` surface the shell draws itself, not the operating system's
picker.

**How the Edit and Search rows are verified.** The Find, Replace, Go-to, date/time and
clipboard rows had long been settled at the engine — `nc-textproc`'s search suites and
`nc-core`'s undo, clipboard and select-all suites cover *direction and wrap, the option
set, clamping, undo grouping, clipboard text and its CRLF conversion* to the byte — but
the rows still read `IMPLEMENTED` because the shell's own half had not been pinned. It
mostly already was, spread across three files rather than one: the dialogs open and
pre-fill from the command in `mod.rs` (`ctrl_f_opens_the_find_dialog_through_the_registry`,
`the_go_to_command_pre_fills_the_current_line`, `f5_inserts_the_time_and_date_at_the_caret`),
the search steps and replaces in `actions.rs`, and each accelerator is fixed by the
registry's lookup table. Reaching the acceptance text still needed four new shell tests in
`actions.rs`: that Replace All is undone in **one** `Ctrl+Z` (`replace_all_is_a_single_undo_step_from_the_shell`,
which the engine already promised but no shell test had checked), that *Match case* and
*Whole word* steer a search that reaches the document, and that a search with wrapping
**off** stops at the edge and says so rather than wrapping silently. The clipboard joined
them: the engine proves a multi-line selection copies as CRLF, but only
`copy_puts_the_selection_on_the_clipboard_as_crlf` proves the shell's `egui`-backed
clipboard — the one a `Ctrl+C` actually reaches — does the same. Each new test was
mutation-checked: ignoring the options, always wrapping, replacing one match per step, and
stopping the clipboard's CRLF conversion each fail their test.

`NC-E004`, `NC-E005` and `NC-E007` deliberately stay `IMPLEMENTED`. Their `nc-core`
caret, mouse and tab models are complete and tested in isolation, but the running editor
is an `egui::TextEdit` fed by the `DocBuffer` adapter in `editor.rs`, and egui handles
arrow keys, click/drag/double/triple-click and focus *natively*; the `nc-core` rule
functions are called nowhere in the live path, and the two models even disagree (a
`nc-core` triple-click selects the line *with* its terminator, egui's does not). Promoting
those rows on their engine tests would read the specification off a model the program does
not run — the exact inflation this note exists to prevent — so they wait for the day the
shell is made to use the engine's rules, which is a feature decision, not a test.
`NC-E010` does move: its line/column is computed by the shell's own `line_column` in
`editor.rs`, not by `nc-core`'s unused `status_position`, and `caret_position_is_one_based`
tests that live function directly. `NC-E008` moves because the buffer adapter edits the
document at character indices across multi-byte text (`the_buffer_edits_the_document_at_character_indices`)
and the edit path keeps grapheme clusters whole under undo/redo.

**How M7's rows are verified.** `nc-markdown`'s own suite
(`crates/markdown/tests/markdown/…`) asserts the byte-level criteria: exact
serialization, ATX output, nested indent, link syntax, the clear result, and that
any document reads back unchanged. `crates/platform/src/ui/markdown.rs` then fires
each command through `actions::run` — the same function a menu click and a key
chord reach — and asserts what lands in the document. That is what makes these rows
`VERIFIED` rather than merely implemented: a user can cause the bytes, and the
shortcut is registered, dispatchable, and greyed exactly where it should be. The
same suite settles `NC-M010` (one command costs exactly one undo step, measured on
`undo_depth`) and `NC-M001` (`.md` / `.markdown`, case-insensitive, and nothing
else), and it settles `NC-M011` at file level: a `.md` holding a footnote, raw
HTML, a fence and `==highlight==` opens and saves back byte for byte, CRLF
included.

`NC-M008`'s "expand semantics" was the last M7 row open, and it was waiting on a
reading rather than on code: the registry carries one table command and the capture shows
no table dialog, so there was nothing to hang growth on. The reading taken is that the
same keystroke grows the table it lands in — one row below the caret's row, as many cells
as the table's widest row, never between a header and its delimiter, since a row there is
a broken table rather than a row — because that invents no control, no dialog and no key
chord; §12 records it beside the glyph it belongs to. Growing a table *sideways* is still
unimplemented and for the original reason: no command, menu item or shortcut anywhere in
the capture would fire it, so it remains a decision rather than a backlog item.

`NC-M002` is the row before it, and why it could not be verified sooner is
worth keeping.
`cmd.view.md_source` and `cmd.view.md_formatted` used to set a flag that nothing read:
the registry declared `Toggle::FormattedView` and no command carried it, so the pair
did not even show a check mark. A toggle with no visible effect has no content or
selection that could be lost, so the criterion had nothing to hold.
`crates/platform/src/ui/markdown_view.rs` now renders a document from its own
Markdown, and `crates/platform/src/ui/markdown.rs` settles what the criterion asks:
switching views leaves the text, the selection, the dirty flag and the undo depth
untouched; one undo after a round trip takes back the format made *after* the trip,
not the trip; and coming back queues the caret into the editor widget, which is a
fresh one and would otherwise have started at offset zero and moved the user to the
top of the file. `markdown_view.rs`'s own tests pin the rendering decisions apart from
the painting, and `appearance.rs` reads them back off the painted frame: a heading is
painted larger and with no `#` in front of it, `**` becomes weight rather than glyphs,
a nested item is drawn to the right of its parent, and syntax the parser does not know
(`[^1]:`, raw HTML, `==highlight==`, an unpaired `**`) is painted exactly as written —
because a view that hid what it could not parse would show the user a document that is
not the one in the file. With that in place `LANDED_MILESTONE` in
`crates/platform/src/ui/actions.rs` reads 7: every milestone-7 command now changes
something a user can see, which is the bar that constant exists to hold.
The toolbar needed correcting on the way. Its `H1 ▾` dropdown wrote the tab's
paragraph style directly and its `≡ ▾` dropdown fired nothing at all, which meant
the same feature behaved differently depending on where it was clicked; both now
route through `CommandId`, and the table glyph — previously inert — fires
`cmd.md.table`.

**How M8's first rows are verified.** `Settings ▸ App theme` began as a control with
nothing behind it: it wrote `shell.theme` and no code read it, so choosing Light
recoloured nothing. That is the settings-page shape of the bug `LANDED_MILESTONE`
exists to catch for commands, and it is why M8's rows are tested at two ends — the
choice must change what is painted, *and* the same must be true after a restart.

The colours moved from module constants into a `Palette` value
(`crates/platform/src/ui/theme.rs`) resolved from the choice plus what the system
reports, and the constants were **deleted**: a leftover `theme::TEXT` compiles and
paints one widget dark in a light window forever, and a suite that paints the surfaces
it cares about would never notice. `appearance.rs` then asserts the outcome — a frame
chosen light contains the light chrome and page colours and *none* of the dark
palette's; an explicit choice holds still while the system moves underneath it; `Use
system setting` recolours the window that is already open, tested across two frames of
one `egui` context because that is what "already open" means.

`settings.rs` is the store: `key value` lines with a version header, like the session
file, written a frame after a change and read back at startup. It keeps an unknown key
verbatim rather than dropping it, so an older build cannot erase a newer one's
settings, and it refuses a header it does not recognise instead of applying half of it.
Its table test moves **one setting at a time** through a write and a restart, so a
setting that is stored but never restored — or restored but never stored — fails as
itself rather than hiding behind the five that work. `NC-P001` and `NC-P003` are
`VERIFIED` on that.

**Word wrap was a check mark over an editor that wrapped either way.** Turning it off
asked the text edit for an infinite width, which `egui` 0.36 clamps back to the panel's
width before laying anything out — and a scroll area does not help either, because 0.36
tells a scroll area's inner UI to prefer wrapping to showing a horizontal bar. So the
flag flipped and the document reflowed identically in both states. The editor now
measures its widest line and hands it to the widget as a floor, which is the one input
that outranks the clamp, and the horizontal bar comes from the scroll area around it.
`appearance.rs` proves both directions from the painted galley: wrapped, a long line
breaks into rows inside the window; unwrapped, it stays one row and runs past it. That
is also `NC-E006`'s shell half, which is why its row moved: its suite had always
settled the layout arithmetic and never the toggle.

**`NC-P002`'s dropdown listed four fonts this build does not have.** The list was
Consolas, Cascadia Mono, Segoe UI and Courier New: real names on Windows, and unreachable
here, because an `egui` application draws with the faces it ships and never asks the
operating system for its own. So every option in that dropdown painted the identical face —
the settings shape of the bug `LANDED_MILESTONE` exists to catch, one row over from the
commands it watches. The list is now what is actually carried: JetBrains Mono, IBM Plex
Mono, Atkinson Hyperlegible Mono and Fira Mono, each embedded from its upstream file under
the OFL, with the licence text beside the font and the provenance in
`crates/platform/assets/fonts/README.md`. All four are monospaced because both document
surfaces are (`markdown_view` indents with spaces, which is exact only in a monospaced
face), and Atkinson Hyperlegible Mono is the family a low-vision reader can ask for, which
is the lever `NC-P007` will need. Choosing one is proven at two ends: the four rasterize to
different marks for the same string, and the editor's own text is set at a different
advance afterwards, asserted from a painted frame because "applies to the editor" is a claim
about the editor. `NC-P002`'s other half, *size*, has no control in the capture, so the
family comes from settings and the size from zoom — recorded in §4 rather than invented.

Two measurements from that work changed the code. Three of the four families share a 0.6 em
advance, so comparing **widths** cannot tell JetBrains Mono from Fira Mono; the glyph atlas
and the per-character advance can, and the tests use those. And a face a window changes
reflows every line, which moves the row the caret sits on *without the caret moving* — so
nothing follows it, and the caret was left below the fold by a settings change. The editor
now compares the face and size it laid the document out in and follows the caret when either
moves; that is also why installing the shipped faces costs a frame and the caret tests warm
the context up first.

**`NC-P005`'s switch was a local variable seeded from another setting.** The page built
the Formatting row as `let mut formatting = shell.word_wrap;` and put a switch on it: the
value shown was word wrap's, the value written was dropped, and nothing read either. A user
turning word wrap off watched the formatting switch go off with it — two rows, one setting —
while the toolbar never moved. The field, the file's line and the toolbar's own `if` are the
row's whole ask ("Toolbar hides when off"), and it is asserted three ways because the bug had
three places to hide: a frame with the formatting controls in it and a frame without them; a
round trip through off that puts them back; and the settings page read back as what it
*claims*, matching each row's `On`/`Off` label against the shell's field for all four
combinations of the two switches. That last one is the test the other two cannot be: a page
that lies about a setting passes every assertion that only looks at the setting.

**Opening files was the last setting with no behaviour — and the last one M8 needed.**
`Settings ▸ Opening files` was stored, restored and shown, and nothing read it, so the M8
exit criterion ("each supported setting persists correctly and produces the expected
behaviour across restarts") had a row it could not clear. It is the same shape as the two
above — a control whose page is honest and whose effect is absent — but here the page was
never wrong, only alone. All three choices now act, at the one funnel every open goes
through: the Open dialog and `File ▸ Recent` both reach `actions::open_path`, so a file
opened either way goes where the setting says. Two rules run *before* the choice and are
tested to: an already-open file is focused rather than duplicated (`NC-F012` — a file is
never open twice, so the setting has nothing to decide), and the file is read in the window
the user used, so a read failure is reported there and *Open in a new window* opens no window
for a file that cannot be read. *Replace the current tab* is the one with a question in it:
replacing a tab throws its edits away, and the rule for unsaved work is to ask (`NC-D005`),
which this row does not get to skip — so the incoming document is read first and held while
the prompt is answered, and `Save`, `Don't save` and `Cancel` each have a defined end.

Proving it turned up a second bug in the machinery that carries an answer past a detour. The
unsaved-changes prompt's "save, then close" travelled as a `close_after` flag that was set
only in Save As's branch, so a document *with* a path whose text the file's encoding could
not hold wrote the file, left the tab open, and left the window's close half-done. The flag
is now a value — `SaveThen::{Keep, Close, Replace}`, recorded before any detour and spent by
whichever path finishes the write — and a test closes such a tab through the encoding
question to hold that down. Section G of the matrix carries no acceptance row for this
control, so nothing was promoted: the behaviour is pinned by the M8 section of
`crates/platform/src/ui/lifecycle.rs` and recorded in `docs/ui-reference.md` §4 instead.

**`NC-P004` is the matrix's one row built as invented UI, and it is `IMPLEMENTED`, not
`VERIFIED`, for that reason.** The capture has no spell checker — no squiggle, no
`Spelling` menu, no command to copy — so the row was built against a written-up design
(`docs/ui-reference.md` §14), authorized for this build, rather than transcribed. Its
acceptance text, "Squiggle, suggestions", is met: a red squiggle is painted under a
likely misspelling and `F7` offers corrections that replace the word (keeping a capital
it started with, and closing over the tab that owned them). The engine is what the tests
weigh most: a word is flagged only when it is unknown *and* one edit from a known word,
so the small embedded list squiggles `teh` and `wrod` without error, and leaves `zebra`
alone. That one-edit rule is the row's whole limit and is stated plainly in §14 — a slip
more than one edit from every listed word is missed, and a correct rare word near some
other word can still be marked. It is `IMPLEMENTED` because there is no captured
acceptance row to verify against, exactly as `Opening files` above rests on its M8 tests
rather than a screenshot; the tests live in `crates/textproc/src/spell.rs` (the engine),
`appearance.rs` (the squiggle is painted, and only when the setting is on), and
`lifecycle.rs` (the command, the correction, and the "nothing to correct" reports).
Autocorrect — the second half of the row's name — is deliberately not built: nothing is
ever silently rewritten.

**A gap no row covers — the editor could not scroll — is closed.** Measured while
proving word wrap, from a painted frame: a document longer than the window kept its first
line at the top and its caret off screen, because the text edit sat directly in the panel
with nothing scrolling it. Every section of the matrix assumes a reachable document, and
none of them names scrolling — the capture cannot show a scrollbar either, so there is
nothing to write an acceptance row *from*. It stays recorded here rather than invented as
a row. The source view now sits in one scroll area enabled on both axes with the caret
kept inside it: `egui` already asks to keep a caret in view whenever the caret moves
*inside* the text edit and needed only an area willing to hear the request, and a caret
moved from *outside* — Find, Go to line and Select all write the widget's state between
frames, so within its own frame the cursor never changed — is scrolled by the shell asking
for the same rectangle the widget would have used. Three painted tests hold it: one per
mechanism, checked separately by disabling each in turn, and one that naming a caret
already on screen does not shift the text the user is reading.

Making those tests mean anything needed the harness to report a clock. `egui` applies a
scroll request from the *timestamp* of a later frame, and `RawInput::time` unset — what
these frames were passing — never applies it: measured both ways on one request, with a
timestamp the view jumps to its target and without one it does not move. So a frame here
now carries a time, as every real frame does, and `theme.rs` sets the scroll to snap
rather than glide, because a Find that slides the caret towards its match over a third of
a second misreports a command that has already run.

**`NC-P007` was a window a screen reader read as a list of glyphs.** Its whole
acceptance was "Names, focus order", and nothing was named: the tabs, their close
buttons and the `+ ✕ ▢ —` strip, the formatting controls whose entire text is `B`,
`🔗`, `▦` or `Aa`, the avatar, the pill switches and every dropdown on the
settings page. `egui` names what it draws, so the menus and the dialog buttons were
already read aloud; what it cannot name is what the shell paints, or writes as a
glyph — which was nearly everything a window is made of. The work is a small
module (`a11y.rs`) that says what to call a response, a role finer than
`WidgetType` offers, a state, a painted string, and — for the one control the
capture shows and this build does not act on, the title bar's back arrow —
"unavailable". The tests read the tree a frame produces, which is the same tree a
screen reader is handed, and hold two things: every stop Tab can reach, on seven
surfaces (source and formatted view, settings, Find and Replace, Go to, the Open
dialog, a prompt), has a name with a letter in it, because a glyph is not a name;
and the order is the one the frame registers — pinned, not chosen. The last piece
is a node for the formatted view: it paints the document and holds no text field,
so a reader found the body of the window empty; it now exposes the document's own
text, though not its structure, which is recorded in §13.

Two measurements shaped it. The title bar's drag area was a `click_and_drag`
rectangle across the whole bar, which made it an unnamed Tab stop ahead of the
first tab; it now senses drags only. And a right-to-left group cannot be made to
register left to right by nesting a left-to-right one inside it — the group lands
at the bar's left edge instead — so the registration order follows the layout the
capture fixes, and the tests record it.

**The outside-change question was the last prompt with no box.** The engine could
already tell that the bytes on disk were no longer the ones NoteCraft read
(`NC-D008`, `ExternalChange::Modified` / `Removed`), and `nc-textproc::prompt`
already turned that into a `Question::OutsideChange`; what was missing was the
shell half §9 specifies. There is now a `Prompt::OutsideChange`, its card, and
three answers. *Reload* takes the bytes on disk, re-detects the encoding and
hands the editor a fresh document, as `TextFile::reload` always did. *Keep my
version* leaves the text, caret and history exactly as they were and adopts the
file now on disk as the baseline — `TextFile::keep_local_version` — while
`Document::mark_dirty` makes the kept text unsaved work against it, so the next
save writes the user's version and the same change is not asked about twice.
*Cancel* changes nothing.

The question is asked when the window **gains focus**, the moment a program that
had the foreground has finished writing, so a window kept in front does not
re-read every document every frame; the trigger and the Cancel semantics are
recorded in §9. It is never raised over a question or a dialog that is already
up. A restart takes the file on disk as its baseline for a restored unsaved
document (`TextFile::restore` now reads it), so coming back to NoteCraft does not
report a change that happened before the session was written. The shell
acceptance tests drive the real state: a changed file raises the
card and changes nothing yet; a focused frame after an outside write raises
nothing, while losing and regaining focus does; Reload, Keep and Cancel each
have the end §9 gives them; and the answer functions were mutation-checked by
removing the dirty mark, the adopted baseline, the focus transition and the
already-asked guard in turn.

**NC-T003 is blocked, not unstarted.** Its acceptance criterion is a "Reference
item set", but `docs/ui-reference.md` documents no context menu anywhere — no
match for "context menu" or "right-click" in the file — and §11 states outright
that the reference capture does not include these behaviours. There is no item set
to implement against, and AGENT.md §2 forbids inventing UI the reference does not
show. It also appears under M2's outstanding surface in `ROADMAP.md`, so it is
double-booked. Unblocking needs a decision from whoever owns the reference capture:
capture the menu, or record a decided item set in the registry the way §3 allows.

> The engines (`nc-core`, `nc-textproc`) and the shell's core surfaces are
> `IMPLEMENTED`, and the Find/Replace, Go-to, and Open/Save-as dialogs are built
> on them, with the unsaved-changes, overwrite, and unrepresentable prompts. M6's
> behaviour criteria are `VERIFIED` against a dedicated acceptance suite, and so is
> all of M7 — every Markdown command, the source/rendered view pair, and the
> serialization the file is written with. M8 has begun: the theme applies and
> survives a restart, settings are stored, the font list is real — four families this
> build actually carries, applied to the editor — the formatting switch now hides
> the toolbar it names, every open honours the opening-files setting, and every
> control Tab can reach says what it is, and a file changed underneath the window
> raises the question §9 specifies. Still ahead are spell check, printing and
> page setup. The visual
> criteria await a display, and the tab context menu awaits a reference decision.
