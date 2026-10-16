# NoteCraft

A fast, modern text and Markdown editor written in Rust, drawn with
[`egui`](https://github.com/emilk/egui) and hosted by
[`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).

NoteCraft is built around a captured UI reference and a single authoritative
command registry: the window draws from the registry and routes every menu,
toolbar, and keyboard action back through it, so the same command runs however
it is invoked.

- One portable UI stack — the same build runs everywhere `eframe` supports.
- Headless engines for documents, text processing, and commands, kept free of
  any window dependency so they build and test anywhere.
- Data integrity first: atomic saves, explicit encoding and line-ending
  handling, and no silent loss of document content.
- Test-backed behaviour rather than a UI that merely looks familiar.

## Status

NoteCraft is in active early development and has no release yet. The workspace
builds and the headless engines are largely implemented; several UI surfaces and
session features are still ahead.

Milestone progress from [`ROADMAP.md`](ROADMAP.md). M0 — the specification and
UI capture — is complete. M1 through M5 — the Rust foundation, the
`egui`/`eframe` shell, the text engine, file I/O, and the editing commands and
search — are in progress and partly implemented; the shell backs each tab with a
`TextFile`, and the Find/Replace, Go-to, and Open/Save-as dialogs are built on
the `nc-textproc` engines, with the unsaved-changes, overwrite, and
unrepresentable prompts and the dirty indicator on the tab. M6 — tabs and
lifecycle — is in progress: tab switching, Save all, duplicate-open identity,
New window, recent files, and session restore (which reopens the previous
documents, including unsaved work) are in place. Markdown, settings and desktop
integration, optional features, performance, validation, and release engineering
are still ahead.

See [`docs/feature-matrix.md`](docs/feature-matrix.md) for per-feature
implementation and verification status — it is the source of truth.

## Features

**File and document lifecycle**

- New document and new tab (`Ctrl+N`), close tab (`Ctrl+W`).
- Open (`Ctrl+O`), Save (`Ctrl+S`), Save As with encoding selection
  (`Ctrl+Shift+S`).
- Session restore: the previous session's tabs, including unsaved work, reopen
  on the next start when no files are named on the command line.
- Recent files: `File ▸ Recent` lists the documents opened so far, newest first,
  and choosing one reopens it — focusing the tab if it is already open.
- Encoding detection and transcoding, with atomic writes that never overwrite a
  file before the replacement data is safely prepared.
- Explicit BOM, line-ending (CRLF/LF/CR), and dirty-state handling.

**Editing**

- Multilevel undo/redo, cut/copy/paste with EOL conversion.
- Caret navigation across word, line, and paragraph boundaries.
- Mouse selection (click, drag, double-click, triple-click).
- Word wrap, tab stops, drag-and-drop text.
- Unicode-correct editing, including astral characters, combining marks, emoji,
  and IME composition.

**Find and navigation**

- Find (`Ctrl+F`), find next/previous (`F3` / `Shift+F3`).
- Replace and Replace All (`Ctrl+H`), applied as a single undoable action.
- Search options: match case and whole word, with wrap-around and boundary
  notices.
- Go to line (`Ctrl+G`).

**UI shell**

- Tab strip and title bar, File/Edit/View menu bar, formatting toolbar, editor
  surface, status bar, and a Settings page, matching
  [`docs/ui-reference.md`](docs/ui-reference.md).
- Find/Replace, Go to line, and Open/Save-as dialogs, with the unsaved-changes,
  overwrite, and unrepresentable prompts.
- A dirty indicator on the tab, and a status bar with one-based line and column
  reporting.
- Keyboard accelerators dispatched from the command registry, so every
  implemented command runs the same whether it is invoked from the menu or the
  keyboard; `F5` inserts the current time and date.

## Architecture

NoteCraft is a Cargo workspace split into headless engines and a thin UI shell.
Dependencies flow in one direction: the engines know nothing about the window
layer, and `cargo xtask check-boundaries` enforces that direction as a CI gate.

| Crate | Package | Role |
| --- | --- | --- |
| [`crates/core`](crates/core) | `nc-core` | Document model: text buffer, selection, caret, undo/redo, clipboard, word wrap, tabs, line endings. Headless. |
| [`crates/textproc`](crates/textproc) | `nc-textproc` | Encoding detection and transcoding, search/replace, time/date insertion, atomic file I/O. Headless. |
| [`crates/commands`](crates/commands) | `nc-commands` | The command registry, menu model, shortcuts, and availability rules. Headless. |
| [`crates/platform`](crates/platform) | `nc-platform` | The `egui`/`eframe` shell: window, tab strip, menus, toolbar, editor surface, status bar, settings page. |
| [`crates/app`](crates/app) | `nc-app` | Startup, logging, command line, and process lifecycle. Produces the executable. |
| [`crates/xtask`](crates/xtask) | `xtask` | Repository maintenance tasks, run as `cargo xtask <task>`. |

`crates/commands` is the single authoritative definition of the command
surface: every menu item, toolbar control, label, shortcut, and availability
rule lives there, including the availability rules that must visibly disable a
control instead of leaving it a silent no-op.

## Getting started

### Prerequisites

- Rust stable, pinned through [`rust-toolchain.toml`](rust-toolchain.toml)
  (edition 2024, minimum Rust 1.85). Install via [rustup](https://rustup.rs/).
- A desktop environment supported by `eframe` (Linux, macOS, or Windows).

### Build and run

```bash
# Build the workspace
cargo build

# Run the editor
cargo run -p nc-app
```

Release builds are optimised for size (`opt-level = "z"`, thin LTO, stripped):

```bash
cargo build --release
```

The executable is named `notepad` on Windows targets and `notecraft` elsewhere.

## Development

Rust is the only implementation language in this repository — application code,
tests, build tooling, and CI included. Repository automation lives in
`crates/xtask` and is invoked as `cargo xtask <task>`.

Run the workspace checks before considering a change complete:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-boundaries
```

`cargo xtask check-boundaries` verifies the crate dependency direction described
under [Architecture](#architecture) and acts as the CI gate.

Contribution rules — workflow, architecture constraints, testing requirements,
error handling, and the definition of done — are in
[`AGENT.md`](AGENT.md).

## Documentation

| Document | Contents |
| --- | --- |
| [`ROADMAP.md`](ROADMAP.md) | Project goal, technology strategy, and milestones. |
| [`docs/ui-reference.md`](docs/ui-reference.md) | The captured UI: screens and the exact items each one shows. |
| [`docs/feature-matrix.md`](docs/feature-matrix.md) | Per-feature implementation and verification status. |
| [`docs/architecture.md`](docs/architecture.md) | Crate boundaries, module layout, and dependency direction. |
| [`docs/test-strategy.md`](docs/test-strategy.md) | Unit, integration, UI, and stress-testing approach. |
| [`AGENT.md`](AGENT.md) | Engineering rules and the required workflow. |

## Repository layout

```text
crates/
  app/        # nc-app — startup, logging, CLI, lifecycle
  commands/   # nc-commands — command registry (headless)
  core/       # nc-core — document model (headless)
  platform/   # nc-platform — egui/eframe shell
  textproc/   # nc-textproc — encoding, search, atomic I/O (headless)
  xtask/      # cargo xtask tasks
docs/         # architecture, feature matrix, test strategy, UI reference
```

## License

Licensed under the MIT License. See [`LICENSE`](LICENSE) for details.

Repository: <https://github.com/CMSpeedAU/NoteCraft>
