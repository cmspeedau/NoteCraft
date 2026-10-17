# NoteCraft — Roadmap

## 1. Project goal

Build a fast, modern text and Markdown editor in Rust, drawn with `egui` and
hosted by `eframe`, matching the UI captured in `docs/ui-reference.md`: the tab
strip and title bar, the File/Edit/View menu bar, the formatting toolbar, the
editor surface, the status bar, and the Settings page.

The UI is the specification. Every menu item, toolbar control, label, shortcut,
and availability rule lives in the command registry (`crates/commands`,
`nc-commands`), which is the single authoritative definition; the shell draws
from it and routes clicks back through it.

A feature is done when it is integrated into the real application, its entry
points (menu, toolbar, keyboard, dialog) work, its edge cases are tested, and
the workspace checks pass.

## 2. UI reference

`docs/ui-reference.md` records the screens the UI is built from and the exact
items each one shows. Behaviour the screens do not show is a design decision
recorded in the registry, not a guess.

## 3. Technology strategy

### Language and platform

- Rust stable, pinned through `rust-toolchain.toml`.
- One portable UI stack: `egui` for drawing, `eframe` for the window and event
  loop. The same build runs everywhere eframe supports.
- Cargo workspace for modularity; `cargo xtask` for repository tasks.
- Automated tests and CI on the host: `cargo fmt`, `cargo clippy -D warnings`,
  `cargo test --workspace`, and `cargo xtask check-boundaries`.

### Architecture

Modules, in dependency order:

- `nc-core` — document model: text buffer, selection, caret, undo/redo,
  clipboard, word wrap, tabs, line endings.
- `nc-textproc` — encoding detection and transcoding, search/replace, time/date,
  atomic file I/O.
- `nc-commands` — the command registry, menu model, shortcuts, and availability.
- `nc-platform` — the egui/eframe shell: window, tab strip, menus, toolbar,
  editor surface, status bar, and settings page.
- `nc-app` — startup, logging, command line, and the process lifecycle.
- `xtask` — repository tasks (`cargo xtask check-boundaries`).

Keep the headless crates free of any window dependency so they build and test
anywhere; `cargo xtask check-boundaries` enforces the direction.

## 4. Milestones

### M0 — Specification and UI capture

- Capture the UI: main window, menus, toolbar, status bar, and settings page.
- Record every visible command, label, and keyboard shortcut.
- Record text-editing and file-format decisions.
- Create a feature inventory with IDs, priorities, and acceptance criteria.
- Separate interactive features from service-dependent ones.

Exit criteria: Every in-scope feature has a place in `docs/ui-reference.md` or
the command registry, with acceptance criteria.

Status: COMPLETE — `docs/ui-reference.md` captures the main window, the menus,
the toolbar dropdowns and the settings page, and specifies the remaining
dialogs (Find/Replace, Go to, Open/Save As, Page setup, and the unsaved /
overwrite / unrepresentable / outside-change prompts) as design decisions.
`docs/feature-matrix.md` is the feature inventory with IDs, priorities, and
acceptance criteria; `docs/architecture.md` and `docs/test-strategy.md` record
the structural and testing decisions; the command registry is defined; and the
service-dependent features are separated in the matrix (section H). Every
in-scope feature has a place in the reference or the registry with acceptance
criteria.

### M1 — Rust foundation

- Create the Cargo workspace and executable.
- Configure logging, error reporting, and release builds.
- Implement process startup and shutdown.
- Support opening files through command-line arguments.
- Add unit-test, integration-test, and CI infrastructure.

Exit criteria: The application launches, opens a window, and passes the
workspace checks.

Status: IN_PROGRESS — the workspace (`nc-app` with the `notepad` binary,
`nc-platform`, `nc-core`, `nc-textproc`, `nc-commands`, `xtask`), command-line
handling, logging, startup/shutdown lifecycle, unit + integration tests, and CI
(fmt, clippy, host tests, crate boundary check) are in place and green.

### M2 — Window and visual shell

- Main window and title bar.
- Tab strip and window controls.
- Menu bar and command routing.
- Formatting toolbar.
- Editor viewport.
- Status bar.
- Settings page.
- Resizing, maximizing, and minimizing.

Exit criteria: The layout, command structure, and visual states match
`docs/ui-reference.md`.

Status: IN_PROGRESS — the shell is built in `nc-platform` from the command
registry: the custom title bar with tabs and window controls, the
File/Edit/View menus, the formatting toolbar with the paragraph-style and list
dropdowns, the editor surface, the status bar, and the settings page, all in the
dark theme. Every command this build implements is now dispatched from the
registry by its keyboard accelerator (`crates/platform/src/ui/input.rs`), so the
menu and the keyboard run the same command. The Find/Replace and Go to line
dialogs are built on the search engine (NC-S001, NC-S006). Outstanding: the
Page setup surface, context menus, and high-DPI polish.

### M3 — Text engine

- Unicode text model.
- Caret movement and selection.
- Mouse selection, double-click, and line selection.
- Insert, delete, backspace, and overwrite.
- Multilevel undo and redo.
- Clipboard copy, cut, paste, and select all.
- Drag and drop.
- Word wrap and horizontal scrolling.
- Tabs and indentation.
- CRLF, LF, and CR line endings.
- Unicode, surrogate pairs, combining marks, emoji, and IME input.

Exit criteria: All editing commands pass their acceptance tests against the
editor contract.

Status: IN_PROGRESS — `crates/core` (`nc-core`) is the text engine:
`TextBuffer`, `Selection`, `UndoStack`, `CaretMovement`, `LineEnding`,
`TabStop`, grapheme-aware soft wrap, a `Clipboard` trait with an in-memory
implementation, mouse selection rules, drag and drop, and overwrite mode, tied
together by `Document`. Scrolling is no longer outstanding — the shell scrolls
on both axes and keeps the caret in view, which landed with M8 and is recorded
there. The Edit rows verified against this engine (`NC-E001` undo grouping,
`NC-E002` clipboard and its CRLF conversion, `NC-E003` select all, `NC-E008`
surrogate safety) are pinned at both ends — the `nc-core` suites and a shell test
that reaches the command. Three rows stay `IMPLEMENTED` for a reason worth
recording: `NC-E004` caret navigation, `NC-E005` mouse selection and `NC-E007`
tabs are modelled and tested in `nc-core`, but the live editor is an
`egui::TextEdit` over the `DocBuffer` adapter, which handles arrow keys,
click/drag/double/triple-click and focus **natively** — the `nc-core` rule
functions are called nowhere in that path, and the two models disagree (an
`nc-core` triple-click takes the line terminator, egui's does not). Wiring the
editor to the engine's rules, so those tests describe the running program, is an
open decision, not a gap a test can close. Outstanding: IME input, and rendering
of control characters and whitespace (which belong with the viewport).

### M4 — File I/O and data integrity

- Open and save files.
- Save As and overwrite confirmation.
- New document and reopen behaviour.
- Encoding detection and explicit encoding behaviour.
- UTF-8, UTF-16, BOM, and legacy encodings.
- Preserve or correctly select line endings.
- Detect external file changes.
- Handle permissions, locked files, and missing paths.
- Handle empty, malformed, and very large files.
- Dirty-state tracking.
- Unsaved-changes prompts.
- Recover gracefully from failed writes.

Exit criteria: Opening and saving supported fixtures produces the expected bytes
and prompts. No silent data loss.

Status: IN_PROGRESS — `crates/textproc` (`nc-textproc`) owns bytes-to-text and
text-to-bytes: `Encoding` detection and transcoding (UTF-8 with/without a mark,
UTF-16/UTF-32 in either order, and ANSI as Windows-1252), `LineEnding`
round-tripping, and `TextFile` (`open`, `open_as`, `save`, `save_as`,
`save_lossy`, `reload`, `keep_local_version`). Saves are atomic: the file is written to a sibling
temporary file and renamed into place, so a failed write leaves the original
intact. The shell now backs each tab with a `TextFile` — the document, its
encoding and its dirty flag have one source — edits it through a thin
`egui::TextBuffer` adapter, and draws the Open / Save As dialog (filesystem
browsing and the encoding list), with Save / Save As / Open wired and the dirty
indicator on the tab. The unsaved-changes, overwrite, and unrepresentable prompts
are built on `nc-textproc::prompt`, and so is the outside-change prompt: a file
written by another program while the window was in the background raises it when
focus returns, with Reload, Keep my version and Cancel, each tested end to end
(`NC-D008`). Outstanding: end-to-end verification of the remaining prompts.

### M5 — Editing commands and search

- Find and find next/previous.
- Replace and Replace All.
- Search direction and wrap-around behaviour.
- Match-case and other search options.
- Go to line.
- Date/time insertion.
- Select All and the other Edit commands.
- Keyboard shortcuts and menu accelerators.
- Status-bar line and column calculations.

Exit criteria: Search and editing commands pass their acceptance tests,
including boundary cases.

Status: IN_PROGRESS — the engines are in place and the dialog surfaces are
built. `nc-textproc` provides `search` (`SearchOptions`, `Direction`,
`find_next`/`find_previous` returning `Found`/`Wrapped`/`Boundary`/`NoMatch`,
`find_all`, `go_to_line`) and `time_date`; `nc-core` provides
`Document::select_all`, `replace_range`/`replace_ranges` (one undo step per
Replace All), and `status::StatusPosition`. The dialog surface is built in
`nc-platform`: the Find/Replace dialog with match case, whole word, direction
and wrap options; the Ctrl+F / Ctrl+H / F3 / Shift+F3 wiring; the Go to line
dialog; and the status-notification line, all over `nc-textproc::search`. The
shell now dispatches every implemented command's accelerator from the registry
(`crates/platform/src/ui/input.rs`) and routes undo/redo, select all, delete, and
the F5 time/date insertion through the document; cut/copy/paste reuse `egui`'s
clipboard events, with the menu entries requesting the same. That verification has now
landed: every search and edit command is driven from its accelerator through
`actions::run` and checked against the document (see *How the Edit and Search rows are
verified* in `docs/feature-matrix.md`), the boundary cases included — the no-wrap edge,
the wrap notice, Replace All undone in one step, the option set reaching the search, and
the shell clipboard carrying CRLF. The F5 insertion writes UTC (a recorded limitation,
NC-S007).

### M6 — Tabs and application lifecycle

- Multiple documents and tab switching.
- New-tab and new-window commands.
- Tab closing and close-all behaviour.
- Dirty-state indicators.
- Save prompts during tab and application closure.
- Session restoration.
- Recent files.
- Duplicate opens and document identity.

Exit criteria: Tab operations and restarts preserve the expected documents and
state without losing unsaved work.

Status: IN_PROGRESS — the tab model lives in `nc-platform`'s shell state. Tab
switching now works from the keyboard (`Ctrl+Tab`/`Ctrl+Shift+Tab` cycle through
the open tabs, wrapping), `File ▸ Save all` writes every dirty path-backed
document, and a file whose canonical path is already open — through `File ▸ Open`
or a command-line argument — focuses its existing tab rather than opening a
second (`NC-F012`). `File ▸ New window` opens an independent window with its own
tabs and active document (`crates/platform/src/ui/windows.rs`). Closing the
window, and `Exit`, walk the dirty documents with the unsaved-changes prompt
(`NC-T005`). Session restore (`NC-F011`) is in place: on exit the shell writes its
open documents — paths, encodings, line endings, and the text of anything with
unsaved changes — under the user's configuration directory, and on the next start
reads them back when no files are named on the command line
(`crates/platform/src/ui/session.rs`). `File ▸ Recent` lists the documents opened
this session, most recent first, and opening one from it follows the same identity
rule as `File ▸ Open` (`NC-F010`). The milestone's behaviour criteria are now
verified by an acceptance suite (`crates/platform/src/ui/lifecycle.rs`) that drives
the shell's own state and the real session file: a restart reopens every document
with its unsaved text, encoding, and line endings intact and leaves a clean file
byte for byte alone; closing prompts per dirty document and a cancel loses nothing;
tab switching cycles for N tabs; a path already open is focused, a symlink to it
recognised as the same document; and `Exit` emits the window-close command only
once nothing is dirty.

What remains is not code. `NC-T004`'s dirty marker is now verified by painting the
shell headlessly and reading back the shapes
(`crates/platform/src/ui/appearance.rs`): an accent dot on exactly the dirty tabs,
none on a clean one, and the marker gone once the unsaved prompt is answered —
mutation-checked, so the assertions are known to fail if the paint is removed.
Three rows are left, each for a reason worth naming. `NC-T001` asks that tab strip
geometry "match the reference", and the reference gives the strip no coordinates;
the structural claims it does make (§1 — one tab per document, side by side, one
accent underline) are now asserted, but that is the shell agreeing with itself, not
a comparison against the capture, and judging the likeness still takes a person
looking at a running window. `NC-F003` needs a second real top-level window.
`NC-F010`'s Recent submenu needs its drawing observed. And the tab context menu
(`NC-T003`) is blocked on the reference: `docs/ui-reference.md` documents no
context menu at all, so its "Reference item set" does not exist to build against.
It needs a capture, or a recorded decision, before it can be built.

### M7 — Markdown functionality

- Markdown file association and `.md` / `.markdown` handling.
- Syntax view and formatted view.
- Headings, bold, italic, strikethrough, and hyperlinks.
- Ordered and unordered lists, nesting, and indentation.
- Tables.
- Formatting toolbar and context-menu commands.
- Markdown source serialization.
- Switching views without losing content or selection.
- Undo/redo across formatting operations.
- Malformed Markdown and unsupported syntax.

Exit criteria: Formatting operations, Markdown serialization, and view
transitions pass their acceptance tests.

Status: COMPLETE — the formatting model is landed as `crates/markdown`
(`nc-markdown`), built to §6's three rules: it edits the Markdown source rather than
a rich internal tree, its parse keeps every byte (each block carries the exact source
it came from, and property tests assert a document reads back unchanged), and
serialization is deterministic enough to assert on whole strings. Its acceptance
suite lives at the paths the matrix names — `tests/markdown/inline.rs`,
`headings.rs`, `lists.rs`, `links.rs`, `tables.rs`, `clear.rs`, `malformed.rs` — and
covers bold/italic/strike serialization, ATX headings, bullets, numbering and
two-space nesting, link syntax, table insertion, the clear-formatting result, and
that unsupported syntax (`[^1]:`, raw HTML, fences, `==highlight==`) survives
untouched. Commands return the new text *and* the remapped selection, so the shell
can apply a format as one undoable edit.

All three parts of the exit criterion have passing acceptance tests: formatting
operations, Markdown serialization, and the source/formatted view transition. Tables
came last because half of the row was a question rather than a task — one command, no
captured dialog — and the answer taken is that the keystroke grows the table it lands
in (§12). Widening a table is the one thing deliberately left undone: nothing in the
capture would fire it, so it waits for a decision rather than being invented.

The formatted view renders the document from its own Markdown and does not edit it,
which is what `docs/architecture.md` §6 requires: the source is the document, so there
is never a second copy of the text that could drift from it or drop a byte of it. The
switch is consequently free — no edit, no dirty flag, no lost selection, and the caret
is handed back to the editor widget on the way out (`NC-M002`, tested in
`crates/platform/src/ui/markdown.rs` and read back off the painted frame in
`crates/platform/src/ui/appearance.rs`). The one thing the view has to be careful
about is what it shows: it takes off screen nothing but the delimiters of emphasis the
parser recognised, so footnotes, raw HTML, fences and `==highlight==` are painted as
written. §12 of `docs/ui-reference.md` records the look, because the capture contains
no rendered document to copy. Every Markdown document opens in the source view, since
a rendered view takes no typing.

Everything a user can click is wired and tested through the shell
(`crates/platform/src/ui/markdown.rs`, which fires each command through the same
`actions::run` a menu item and a key chord reach): bold, italic, strikethrough, the
seven paragraph styles as ATX levels, bulleted and numbered lists with two-space
nesting, links, tables — writing one and growing one — clear formatting, one-undo-step
formats, the `.md` / `.markdown` decision, and both views. Two toolbar dropdowns that had been
writing shell state directly — or nothing at all — now fire registry commands, and the
table glyph is no longer inert. The reference shows no hyperlink or table-size dialog,
so neither was invented: the commands write their syntax and leave the caret where the
user types (§12). `LANDED_MILESTONE` reads 7: every milestone-7 command now changes
something a user can see, which is the bar that constant exists to hold. Nothing in M7
is `BLOCKED` or `NOT_STARTED`, and `NC-T003`'s context menu is the only row in the whole
matrix waiting on a capture.

### M8 — Settings and desktop integration

Status: IN_PROGRESS — five rows are verified (theme, font, persistence, the formatting
switch, accessibility) plus the word-wrap row it shares with the editor, and every setting
the capture shows now produces the behaviour its row names; what remains is the rows that
need a surface the capture alone does not settle.

*The theme applies* (`NC-P001`). `Settings ▸ App theme` wrote a field no code read;
the colours are now a `Palette` value resolved per frame from the choice plus what the
system reports, the colour constants are gone so a missed site is a compile error
rather than a widget left dark, and painted frames prove a light window contains no
dark-palette colour, that an explicit choice ignores the system, and that `Use system
setting` recolours the window that is already open.

*Settings persist* (`NC-P003`, and `NC-P001` with it). A `settings` file beside the
session file is written a frame after a change and read at startup; a table test moves
one setting at a time through a write and a restart, so a setting that is stored but
never restored fails as itself.

*Word wrap was a check mark over an editor that wrapped either way* — the flag is now
honoured in both directions and proven from the painted layout, which also settles the
shell half of `NC-E006`.

*And the editor could not scroll*, found while proving the above: no matrix row asks for
scrolling, because a screenshot cannot show a scrollbar, so the gap is recorded in the
matrix's verification summary rather than invented as a row. The text edit now sits in one
scroll area enabled on both axes, so a tall document reaches its last line and the caret
stays on screen whether it moved under a key, a click, or a command naming a position.
The painted harness had to start reporting a clock: `egui` applies a scroll request from
the timestamp of a later frame, and without one no request is ever applied.

*The font list is real* (`NC-P002`, and `NC-P007` gains its lever). The dropdown offered
Consolas, Cascadia Mono, Segoe UI and Courier New — four Windows names this build cannot
reach, so every option painted the same face. It now offers the four monospaced families
whose OFL files are embedded here (JetBrains Mono, IBM Plex Mono, Atkinson Hyperlegible
Mono, Fira Mono) under their own names, and a painted frame proves choosing one changes how
the editor sets its text. The row's other half, size, has no control in the capture, so the
family comes from settings and the size from zoom, recorded in §4.

*The formatting switch switches something* (`NC-P005`). Its row was built as
`let mut formatting = shell.word_wrap;` with a switch on top: it displayed word wrap's
value, wrote into a local that was dropped, and the toolbar never moved. Off now takes the
formatting half of the top bar — and the divider that introduced it — with it, and the
settings page is asserted to show each switch its own setting, which is the assertion the
bug survived.

*And *Opening files* now opens files where it says it will* — the last setting that was
stored, restored and shown while nothing read it, which was the last thing standing between
M8 and its exit criterion. The three choices act at the one funnel every open goes through
(`actions::open_path`), with two rules ahead of them: an already-open file is focused rather
than duplicated (`NC-F012`), and a file that cannot be read is reported where it was asked
for. *Replace the current tab* asks before discarding unsaved edits (`NC-D005`), holding the
incoming document while the prompt is answered. Proving it exposed a second bug: the
close that follows an unsaved-changes `Save` was carried by a flag set only in Save As's
branch, so a write that detoured through the encoding question left the tab open and the
window half-closed. That continuation is now one value — `SaveThen::{Keep, Close, Replace}`
— recorded before any detour and spent by whichever path finishes the write.

*And every control says what it is* (`NC-P007`). The row was "Names, focus order", and
nothing was named: a screen reader reached the tab strip, the glyph buttons (`B`, `🔗`,
`▦`, `✕`) and the pill switches and read a list of characters, or nothing at all. Names
now come from a small module that says what to call a control the shell paints or writes
as a glyph, what its state is, and — for the captured back arrow this build does not act
on — that it is unavailable; `egui` already names what it draws itself. The tests read the
tree a painted frame produces, which is the tree a screen reader is handed: every stop Tab
can reach, on seven surfaces, carries a name with a letter in it, and the order is pinned
as the one the frame registers, including the two right-aligned groups that walk right to
left because that is the direction their layout is built in. The formatted view, which
paints the document instead of editing it, now exposes the document's own text — a reader
used to find the body of the window empty. Two measurements shaped it: the title-bar drag
area had become a Tab stop by sensing clicks, and now senses drags only; and nesting a
left-to-right group inside a right-to-left one puts it at the bar's left edge, so the
visual order and the registration order cannot both be had and the capture's layout wins.

*And a word the shell thinks is a slip is underlined* (`NC-P004`, built as invented UI). The
capture has no spell checker at all — no squiggle, no `Spelling` menu, no command — so
the row was written from a design (`docs/ui-reference.md` §14) rather than read off a
picture. `Settings ▸ Spell check` (off by default, persisted like its neighbours) draws a
red squiggle under a likely misspelling, and `Edit ▸ Spelling suggestions` (F7) opens the
corrections for the word at the caret, which replace it when picked, keeping a capital the
original had. The engine — a small embedded list of common base words in
`crates/textproc::spell`, inflections understood by stripping back to a base — flags a word
only when it is unknown *and* one edit from a known word, so a few hundred words squiggle a
typo without squiggling correct prose. That one-edit rule is the whole of its limitation and
is stated in §14. It stays `IMPLEMENTED`, not `VERIFIED`: with no captured control there is
no acceptance row to verify against, so it is pinned by tests in `spell.rs`, `appearance.rs`
(the squiggle paints, and only when the setting is on) and `lifecycle.rs`. Autocorrect, the
row's second half, is not built — nothing is silently rewritten.

What remains is the rest of the settings the capture shows: file associations and printing,
with page setup (§8) the captured screen that printing needs first.

- Font family and size.
- Word wrap and view preferences.
- Theme and system-theme behaviour.
- Settings persistence.
- Accessibility and screen-reader support.
- Keyboard access and focus order.
- File associations and shell integration, if in scope.
- Printing and page setup, if supported.
- Localization and keyboard-layout correctness.

Exit criteria: Each supported setting persists correctly and produces the
expected behaviour across restarts.

### M9 — Optional and service-dependent functionality

Isolate features that require external services, accounts, or entitlements.

- AI writing, rewriting, and summarization.
- Optional onboarding or welcome experiences.
- Service errors, authentication failures, and disabled-feature states.

Keep service-dependent functionality isolated from the core editor. AI commands
are present in the surface but greyed until the service is configured.

### M10 — Performance and reliability

- Measure startup time, memory, and idle CPU.
- Benchmark large files and extremely long lines.
- Benchmark search and Replace All.
- Benchmark scrolling, typing, and selection.
- Avoid unnecessary full-document copies.
- Add stress tests for repeated tab and file operations.
- Test invalid encodings, inaccessible paths, and interrupted saves.

Exit criteria: Performance is within the project's tolerances, without
correctness regressions.

### M11 — Validation

- Exercise every feature's acceptance tests.
- Compare the running UI against `docs/ui-reference.md` screen by screen.
- Test Unicode and input methods.
- Test all supported file encodings and line endings.
- Run randomized editing sequences.
- Update `docs/feature-matrix.md` with results.

Exit criteria: Every mandatory feature passes its acceptance tests, and any
remaining differences are documented and reproducible.

### M12 — Release engineering

- Produce reproducible release builds.
- Test installation, upgrade, and uninstall.
- Sign binaries and installers where appropriate.
- Prepare crash reporting and privacy documentation.
- Create release notes and known-limitations documentation.

Exit criteria: A clean machine can install, launch, use, upgrade, and remove the
application reliably.

## 5. Repository layout

```
NoteCraft/
├── AGENT.md
├── ROADMAP.md
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── app/          # nc-app — startup, logging, lifecycle (binary `notepad`)
│   ├── platform/     # nc-platform — the egui/eframe shell
│   ├── commands/     # nc-commands — command registry, menus, shortcuts
│   ├── core/         # nc-core — document model and editing
│   ├── textproc/     # nc-textproc — encoding, search, file I/O
│   └── xtask/        # repository tasks
├── docs/
│   ├── ui-reference.md
│   ├── feature-matrix.md
│   ├── architecture.md
│   └── test-strategy.md
└── tests/            # integration + fixtures
```

Adjust crate boundaries when a simpler design is supported by evidence. Avoid
creating empty crates solely to match a template.

## 6. Definition of Done

A feature is complete only when:

- Its behaviour is documented (in `docs/ui-reference.md` or the registry).
- Its implementation is integrated into the real application.
- Its keyboard, mouse, menu, and dialog entry points work as applicable.
- Its edge cases are tested.
- Its behaviour is correct and no data is lost.
- It passes formatting, linting, and the automated tests.
- Its regression tests are committed.
- Its status is updated in `docs/feature-matrix.md`.

## 7. Feature tracking

Every feature carries one of these statuses:

- `NOT_STARTED`
- `SPECIFIED`
- `IN_PROGRESS`
- `IMPLEMENTED`
- `VERIFIED`
- `BLOCKED`
- `OUT_OF_SCOPE`

`IMPLEMENTED` means the code exists; `VERIFIED` means its acceptance tests pass.
Track each feature with a stable ID, acceptance criteria, test location, status,
and any known limitations.

## 8. Core engineering principles

- Correctness before cosmetic polish.
- Observable behaviour before assumptions about internals.
- Byte preservation before convenience transformations.
- Explicit error handling rather than silent recovery.
- Automated regression tests for every fixed defect.
- No unsubstantiated feature claims.
- No unnecessary dependencies or architecture complexity.
- No feature removal merely because it is inconvenient.

The objective is a maintainable Rust editor whose UI matches the captured
reference and whose behaviour is backed by tests.
