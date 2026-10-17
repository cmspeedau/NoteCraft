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
(see the verification summary). The page-setup surface and the outside-change prompt are
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
| NC-E006 | Word wrap toggle | MANDATORY | M3 | SPECIFIED | VERIFIED | Soft wrap; no bytes changed | `crates/core/tests/editing/word_wrap.rs`, `crates/platform/src/ui/appearance.rs`, `crates/platform/src/ui/settings.rs` |
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
| NC-P002 | Font family & size | MANDATORY | M8 | SPECIFIED | IN_PROGRESS | Dropdown shown; applies to the editor | `crates/platform/src/ui/settings.rs` |
| NC-P003 | Word-wrap preference persistence | MANDATORY | M8 | SPECIFIED | VERIFIED | Survives restart | `crates/platform/src/ui/settings.rs` |
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
| IMPLEMENTED | 33 |
| IN_PROGRESS | 1 |
| NOT_STARTED | 11 |
| VERIFIED | 22 |
| BLOCKED | 1 |

**What `VERIFIED` means here.** Twenty-two rows are verified, by suites that drive the
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
`VERIFIED` on that; `NC-P002` stays `IN_PROGRESS` because the font dropdown still does
not reach the editor.

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
> survives a restart, and settings are stored. Still ahead are the font reaching the
> editor (`NC-P002`), spell check, the formatting toolbar's hide, accessibility
> evidence, printing and page setup, and the outside-change prompt. The visual
> criteria await a display, and the tab context menu awaits a reference decision.
