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
rather than opened twice (`NC-F010`, `NC-F012`). M7 has begun: `nc-markdown` holds
the formatting model, tested against its acceptance criteria but not yet reachable
from the window (see the verification summary). The page-setup surface and the
outside-change prompt are still ahead. Seven M6 behaviour criteria are `VERIFIED` by
an acceptance suite (`crates/platform/src/ui/lifecycle.rs`) and the dirty marker by
its painted shapes (`crates/platform/src/ui/appearance.rs`); the remaining visual
criteria and the tab context menu are not — see the verification summary for exactly
which rows rest on what. Nothing is `VERIFIED` until its acceptance tests pass end to
end.

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
| NC-F001 | New empty document (Ctrl+N) | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | UTF-8 (no BOM), CRLF, empty, in a new tab | `tests/editing/new_document.rs` |
| NC-F002 | New tab (Ctrl+N) | MANDATORY | M6 | SPECIFIED | VERIFIED | Adds a tab without disturbing others | `crates/platform/src/ui/lifecycle.rs` |
| NC-F003 | New window (Ctrl+Shift+N) | MANDATORY | M6 | SPECIFIED | IMPLEMENTED | Independent top-level window | `crates/platform/src/ui/windows.rs` |
| NC-F004 | Open file (Ctrl+O) | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Open dialog (§7); new tab; encoding/EOL detected | `crates/platform/src/ui/actions.rs` |
| NC-F005 | Save (Ctrl+S) | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Byte-correct in-place save; no data loss | `crates/platform/src/ui/actions.rs` |
| NC-F006 | Save As + encoding (Ctrl+Shift+S) | MANDATORY | M4 | SPECIFIED | IMPLEMENTED | Save-as dialog (§7); overwrite prompt (§9); chosen encoding applied | `crates/platform/src/ui/actions.rs` |
| NC-F007 | Print / Page setup (Ctrl+P) | OPTIONAL | M8 | SPECIFIED | NOT_STARTED | Page-setup dialog (§8); print pipeline | `tests/file_io/print.rs` |
| NC-F008 | Close tab (Ctrl+W) | MANDATORY | M6 | SPECIFIED | VERIFIED | Closes the active tab; last tab resets to untitled | `crates/platform/src/ui/lifecycle.rs` |
| NC-F009 | Exit (Alt+F4) | MANDATORY | M6 | SPECIFIED | VERIFIED | Closes the window | `crates/platform/src/ui/lifecycle.rs` |
| NC-F010 | Recent files | OPTIONAL | M6 | SPECIFIED | IMPLEMENTED | File ▸ Recent lists prior documents | `crates/platform/src/ui/state.rs` |
| NC-F011 | Session restore | MANDATORY | M6 | SPECIFIED | VERIFIED | Restores tabs incl. unsaved state | `crates/platform/src/ui/lifecycle.rs`, `…/ui/session.rs` |
| NC-F012 | Duplicate-open / document identity | MANDATORY | M6 | SPECIFIED | VERIFIED | New tab vs focus existing | `crates/platform/src/ui/lifecycle.rs` |

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
| NC-S001 | Find (Ctrl+F) | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Find dialog (§5) | `crates/platform/src/ui/dialogs.rs` |
| NC-S002 | Find next / previous (F3 / Shift+F3) | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Direction + wrap semantics (§5) | `crates/textproc/tests/editing/find.rs` |
| NC-S003 | Replace / Replace All (Ctrl+H) | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Undo as a single action (§5) | `crates/textproc/tests/editing/replace.rs` |
| NC-S004 | Search options (match case, whole word) | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Option set (§5) | `crates/textproc/tests/editing/find_options.rs` |
| NC-S005 | Wrap-around + boundary notices | MANDATORY | M5 | SPECIFIED | IMPLEMENTED | Wrap option + status notices (§5) | `crates/textproc/tests/editing/find_boundary.rs` |
| NC-S006 | Go to line (Ctrl+G) | OPTIONAL | M5 | SPECIFIED | IMPLEMENTED | Dialog + clamp behavior (§6) | `crates/textproc/tests/editing/go_to.rs` |
| NC-S007 | Insert date/time (F5) | OPTIONAL | M5 | SPECIFIED | IMPLEMENTED | Format string | `crates/textproc/tests/editing/time_date.rs` |

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
| NC-D008 | External file-change detection | OPTIONAL | M4 | SPECIFIED | IMPLEMENTED | Detects rewrite/removal; outside-change prompt (§9) | `crates/textproc/tests/file_io/external_change.rs` |

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
| NC-M001 | `.md`/`.markdown` association & handling | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Correct path per extension | `tests/markdown/assoc.rs` |
| NC-M002 | Syntax view vs formatted view toggle | MANDATORY | M7 | SPECIFIED | NOT_STARTED | No content/selection loss | `tests/markdown/view_switch.rs` |
| NC-M003 | Bold / Italic (Ctrl+B / Ctrl+I) | MANDATORY | M7 | SPECIFIED | IN_PROGRESS | Serialization exact | `crates/markdown/tests/markdown/inline.rs` |
| NC-M004 | Strikethrough | MANDATORY | M7 | SPECIFIED | IN_PROGRESS | `~~` serialization | `crates/markdown/tests/markdown/inline.rs` |
| NC-M005 | Headings | MANDATORY | M7 | SPECIFIED | IN_PROGRESS | ATX output | `crates/markdown/tests/markdown/headings.rs` |
| NC-M006 | Ordered/unordered lists + nesting | MANDATORY | M7 | SPECIFIED | IN_PROGRESS | Nested indent correct | `crates/markdown/tests/markdown/lists.rs` |
| NC-M007 | Hyperlinks | MANDATORY | M7 | SPECIFIED | IN_PROGRESS | Link syntax exact | `crates/markdown/tests/markdown/links.rs` |
| NC-M008 | Tables | MANDATORY | M7 | SPECIFIED | IN_PROGRESS | Insert/expand semantics | `crates/markdown/tests/markdown/tables.rs` |
| NC-M009 | Clear formatting | OPTIONAL | M7 | SPECIFIED | IN_PROGRESS | Result | `crates/markdown/tests/markdown/clear.rs` |
| NC-M010 | Formatting participates in undo/redo | MANDATORY | M7 | SPECIFIED | NOT_STARTED | Single-step undo | `tests/markdown/undo.rs` |
| NC-M011 | Malformed / unsupported syntax preserved | MANDATORY | M7 | SPECIFIED | IN_PROGRESS | No silent normalization | `crates/markdown/tests/markdown/malformed.rs` |

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
| IMPLEMENTED | 34 |
| IN_PROGRESS | 10 |
| NOT_STARTED | 15 |
| VERIFIED | 8 |
| BLOCKED | 1 |

**What `VERIFIED` means here.** Eight rows are verified, by two suites that drive
the real shell rather than a model of it.

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

**What M7's `IN_PROGRESS` rows mean.** `nc-markdown` has landed, and eight of its
rows are `IN_PROGRESS` rather than `IMPLEMENTED` — deliberately. The engine is
tested against exactly the criteria the rows name (exact serialization, ATX output,
nested indent, link syntax, the clear result, no silent normalization), but through
`nc_markdown::…` rather than through the running application. `Ctrl+B` is registered
in the registry and not yet dispatched, so nobody can trigger any of it from the
window. Wiring the commands into the shell is what makes them `IMPLEMENTED`. Three
M7 rows stay `NOT_STARTED` because the formatter cannot answer them at all:
`NC-M001` (per-extension paths), `NC-M002` (the view toggle), and `NC-M010`
(single-step undo) are properties of the shell and the document buffer, not of
Markdown parsing. `NC-M008` has a second reason to wait: insertion is implemented
and tested, expanding an existing table is not.

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
> on them, with the unsaved-changes, overwrite, and unrepresentable prompts. M6
> core — tab switching, Save all, duplicate-open identity (NC-F012), New
> window, session restore (NC-F011), and recent files (NC-F010) — is in place,
> and M6's behaviour criteria are now `VERIFIED` against a dedicated acceptance
> suite. The page-setup surface, the outside-change prompt, Markdown, and the
> remaining session work are still ahead; the visual M6 criteria await a display,
> and the tab context menu awaits a reference decision.
