# Feature Matrix — Implementation & Verification Status

Source of truth for **implementation and verification status** (AGENT.md §11).
The UI reference lives in [`ui-reference.md`](ui-reference.md); the command
surface lives in `crates/commands`.

**Status values:** `NOT_STARTED` · `SPECIFIED` · `IN_PROGRESS` · `IMPLEMENTED` ·
`VERIFIED` · `BLOCKED` · `OUT_OF_SCOPE`

`IMPLEMENTED` means the code exists; `VERIFIED` means its acceptance tests pass.

**Current phase: M2 shell + M3–M5 engines.** The egui/eframe shell draws the
window, menus, toolbar, editor, status bar, and settings page; the `nc-core` and
`nc-textproc` engines back editing, file I/O, and search. Dialog surfaces and
the Markdown, session, and settings-persistence work are still ahead.

---

## A. File & document lifecycle

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-F001 | New empty document (Ctrl+N) | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | UTF-8 (no BOM), CRLF, empty, in a new tab | `tests/editing/new_document.rs` |
| NC-F002 | New tab (Ctrl+N) | MANDATORY | M6 | SPECIFIED | IMPLEMENTED | Adds a tab without disturbing others | `crates/platform/src/ui/state.rs` |
| NC-F003 | New window (Ctrl+Shift+N) | MANDATORY | M6 | SPECIFIED | NOT_STARTED | Independent top-level window | `tests/session/windows.rs` |
| NC-F004 | Open file (Ctrl+O) | MANDATORY | M4 | SPECIFIED | IN_PROGRESS | Open dialog; new tab; encoding/EOL detected | `tests/file_io/open.rs` |
| NC-F005 | Save (Ctrl+S) | MANDATORY | M4 | SPECIFIED | IN_PROGRESS | Byte-correct in-place save; no data loss | `tests/file_io/save.rs` |
| NC-F006 | Save As + encoding (Ctrl+Shift+S) | MANDATORY | M4 | SPECIFIED | IN_PROGRESS | Chosen encoding applied | `tests/file_io/save_as.rs` |
| NC-F007 | Print / Page setup (Ctrl+P) | OPTIONAL | M8 | SPECIFIED | NOT_STARTED | Print pipeline | `tests/file_io/print.rs` |
| NC-F008 | Close tab (Ctrl+W) | MANDATORY | M6 | SPECIFIED | IMPLEMENTED | Closes the active tab; last tab resets to untitled | `crates/platform/src/ui/state.rs` |
| NC-F009 | Exit (Alt+F4) | MANDATORY | M6 | SPECIFIED | IMPLEMENTED | Closes the window | `crates/platform/src/ui/actions.rs` |
| NC-F010 | Recent files | OPTIONAL | M6 | SPECIFIED | NOT_STARTED | File ▸ Recent lists prior documents | `tests/session/recent.rs` |
| NC-F011 | Session restore | MANDATORY | M6 | SPECIFIED | NOT_STARTED | Restores tabs incl. unsaved state | `tests/session/restore.rs` |
| NC-F012 | Duplicate-open / document identity | MANDATORY | M6 | SPECIFIED | NOT_STARTED | New tab vs focus existing | `tests/session/identity.rs` |

## B. Editing, selection, undo

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-E001 | Undo / Redo multilevel | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Undo depth & grouping | `crates/core/tests/editing/undo.rs` |
| NC-E002 | Cut / Copy / Paste | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Clipboard text + EOL conversion | `crates/core/tests/editing/clipboard.rs` |
| NC-E003 | Select all (Ctrl+A) | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Whole-document selection | `crates/core/tests/keyboard/select_all.rs` |
| NC-E004 | Caret navigation | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Word/line/paragraph boundaries | `crates/core/tests/editing/caret.rs` |
| NC-E005 | Mouse selection (click, drag, double, triple) | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Word/line selection rules | `crates/core/tests/editing/mouse.rs` |
| NC-E006 | Word wrap toggle | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Soft wrap; no bytes changed | `crates/core/tests/editing/word_wrap.rs` |
| NC-E007 | Tabs & indentation behavior | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | Tab stop width, insertion | `crates/core/tests/editing/tabs.rs` |
| NC-E008 | Unicode: astral, combining, emoji, surrogate safety | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | No broken surrogate pairs on edit | `crates/core/tests/editing/unicode.rs` |
| NC-E009 | Drag & drop text | OPTIONAL | M3 | SPECIFIED | IMPLEMENTED | Internal drag; drop target | `crates/core/tests/editing/dnd.rs` |
| NC-E010 | Status bar line/column calculation | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | 1-based line/col | `crates/platform/src/ui/editor.rs` |

## C. Find / replace / navigation

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-S001 | Find (Ctrl+F) | MANDATORY | M5 | SPECIFIED | IN_PROGRESS | Find surface matches the reference | `crates/textproc/tests/editing/find.rs` |
| NC-S002 | Find next / previous (F3 / Shift+F3) | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Direction + wrap semantics | `crates/textproc/tests/editing/find.rs` |
| NC-S003 | Replace / Replace All (Ctrl+H) | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Undo as a single action | `crates/textproc/tests/editing/replace.rs` |
| NC-S004 | Search options (match case, whole word) | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Option set | `crates/textproc/tests/editing/find_options.rs` |
| NC-S005 | Wrap-around + boundary notices | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Status notice | `crates/textproc/tests/editing/find_boundary.rs` |
| NC-S006 | Go to line (Ctrl+G) | OPTIONAL | M5 | SPECIFIED | IMPLEMENTED | Dialog + clamp behavior | `crates/textproc/tests/editing/go_to.rs` |
| NC-S007 | Insert date/time (F5) | OPTIONAL | M5 | SPECIFIED | IMPLEMENTED | Format string | `crates/textproc/tests/editing/time_date.rs` |

## D. Encoding & file I/O data integrity

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-D001 | UTF-8 with & without BOM read/write | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Byte-preserving round-trip | `crates/textproc/tests/file_io/encoding_utf8.rs` |
| NC-D002 | UTF-16 LE/BE (+ UTF-32) read/write | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | BOM-correct round-trip | `crates/textproc/tests/file_io/encoding_utf16.rs` |
| NC-D003 | ANSI (Windows-1252) read/write | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Unrepresentable chars reported | `crates/textproc/tests/file_io/encoding_ansi.rs` |
| NC-D004 | EOL detect/preserve CRLF/LF/CR (incl. mixed) | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Bytes on save preserved | `crates/textproc/tests/file_io/eol.rs` |
| NC-D005 | Dirty-state tracking + unsaved prompts | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Prompt states surfaced | `crates/textproc/tests/file_io/dirty.rs` |
| NC-D006 | Failed-write / read-only / permission errors | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Original file preserved (atomic write) | `crates/textproc/tests/file_io/errors.rs` |
| NC-D007 | Empty / malformed / very large files | MANDATORY | M4/M10 | SPECIFIED | IMPLEMENTED | No crash | `crates/textproc/tests/file_io/stress.rs` |
| NC-D008 | External file-change detection | OPTIONAL | M4 | SPECIFIED | IMPLEMENTED | Detects rewrite/removal | `crates/textproc/tests/file_io/external_change.rs` |

## E. Tabs, session, lifecycle

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-T001 | Tab strip + add-tab button | MANDATORY | M6 | SPECIFIED | IMPLEMENTED | Geometry matches the reference | `crates/platform/src/ui/title_bar.rs` |
| NC-T002 | Tab switching Ctrl+Tab / Ctrl+Shift+Tab | MANDATORY | M6 | SPECIFIED | NOT_STARTED | Cyclic; works with N tabs | `tests/keyboard/tab_switch.rs` |
| NC-T003 | Tab context menu | OPTIONAL | M6 | SPECIFIED | NOT_STARTED | Reference item set | `tests/session/tab_menu.rs` |
| NC-T004 | Dirty indicator on tab/title | MANDATORY | M6 | SPECIFIED | NOT_STARTED | Marker shown when dirty | `tests/session/dirty_indicator.rs` |
| NC-T005 | Close all | MANDATORY | M6 | SPECIFIED | NOT_STARTED | Prompt per dirty doc | `tests/session/close_all.rs` |

## F. Markdown / formatting (M7)

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-M001 | `.md`/`.markdown` association & handling | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Correct path per extension | `tests/markdown/assoc.rs` |
| NC-M002 | Syntax view vs formatted view toggle | MANDATORY | M7 | SPECIFIED | NOT_STARTED | No content/selection loss | `tests/markdown/view_switch.rs` |
| NC-M003 | Bold / Italic (Ctrl+B / Ctrl+I) | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Serialization exact | `tests/markdown/inline.rs` |
| NC-M004 | Strikethrough | MANDATORY | M7 | SPECIFIED | NOT_STARTED | `~~` serialization | `tests/markdown/inline.rs` |
| NC-M005 | Headings | MANDATORY | M7 | SPECIFIED | NOT_STARTED | ATX output | `tests/markdown/headings.rs` |
| NC-M006 | Ordered/unordered lists + nesting | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Nested indent correct | `tests/markdown/lists.rs` |
| NC-M007 | Hyperlinks | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Link syntax exact | `tests/markdown/links.rs` |
| NC-M008 | Tables | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Insert/expand semantics | `tests/markdown/tables.rs` |
| NC-M009 | Clear formatting | OPTIONAL | M7 | SPECIFIED | NOT_STARTED | Result | `tests/markdown/clear.rs` |
| NC-M010 | Formatting participates in undo/redo | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Single-step undo | `tests/markdown/undo.rs` |
| NC-M011 | Malformed / unsupported syntax preserved | MANDATORY | M7 | SPECIFIED | NOT_STARTED | No silent normalization | `tests/markdown/malformed.rs` |

## G. Settings & desktop integration (M8)

| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-P001 | App theme (system/light/dark) | MANDATORY | M8 | SPECIFIED | IN_PROGRESS | Dropdown shown; applies the theme | `crates/platform/src/ui/settings.rs` |
| NC-P002 | Font family & size | MANDATORY | M8 | SPECIFIED | IN_PROGRESS | Dropdown shown; applies to the editor | `crates/platform/src/ui/settings.rs` |
| NC-P003 | Word-wrap preference persistence | MANDATORY | M8 | SPECIFIED | NOT_STARTED | Survives restart | `tests/session/settings.rs` |
| NC-P004 | Spell check + autocorrect | MANDATORY | M8 | SPECIFIED | NOT_STARTED | Squiggle, suggestions | `tests/settings/spell.rs` |
| NC-P005 | Formatting enable/disable toggle | MANDATORY | M8 | SPECIFIED | NOT_STARTED | Toolbar hides when off | `tests/settings/formatting_toggle.rs` |
| NC-P006 | High-DPI scaling | MANDATORY | M2/M8 | SPECIFIED | IMPLEMENTED | Layout follows the display scale | egui runtime |
| NC-P007 | Accessibility / screen-reader support | MANDATORY | M8 | SPECIFIED | NOT_STARTED | Names, focus order | `tests/visual/a11y.rs` |
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
| IMPLEMENTED | 31 |
| IN_PROGRESS | 6 |
| NOT_STARTED | 31 |
| VERIFIED | 0 |

> The engines (`nc-core`, `nc-textproc`) and the shell's core surfaces are
> `IMPLEMENTED`. Dialog surfaces, Markdown, and session work remain. Nothing is
> `VERIFIED` until its acceptance tests pass end to end.
