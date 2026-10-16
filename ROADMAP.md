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
together by `Document`. Outstanding: horizontal scrolling for unwrapped long
lines, IME input, and rendering of control characters and whitespace (which
belong with the viewport).

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
`save_lossy`, `reload`). Saves are atomic: the file is written to a sibling
temporary file and renamed into place, so a failed write leaves the original
intact. The shell now backs each tab with a `TextFile` — the document, its
encoding and its dirty flag have one source — edits it through a thin
`egui::TextBuffer` adapter, and draws the Open / Save As dialog (filesystem
browsing and the encoding list), with Save / Save As / Open wired and the dirty
indicator on the tab. The unsaved-changes, overwrite, and unrepresentable
prompts are built on `nc-textproc::prompt`. Outstanding: the outside-change
prompt, and end-to-end verification.

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
clipboard events, with the menu entries requesting the same. Outstanding:
end-to-end verification of the commands and the search boundary cases. The F5
insertion writes UTC (a recorded limitation, NC-S007).

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
(`NC-T005`). Outstanding: recent files (NC-F010), session restore (NC-F011), and
the tab context menu (NC-T003).

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

### M8 — Settings and desktop integration

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
