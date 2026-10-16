# Architecture — NoteCraft

A fast, modern text and Markdown editor in Rust, drawn with `egui` and hosted by
`eframe`. This document defines the module layout and boundaries. It is expected
to be revised as milestones land; every change to it must be justified by a
reference screen or an engineering constraint, and reflected in the PR that makes
the change.

## 1. Design principles

1. **UI reference first.** Behaviour is derived from `docs/ui-reference.md`, never
   invented.
2. **Bytes are sacred.** File I/O preserves encoding/EOL unless the user changes
   them.
3. **Commands are central.** Every keyboard shortcut and menu item dispatches one
   `CommandId`; the UI and keymap are thin adapters over a command table.
4. **Document model is testable without a window.** Editing primitives have no UI
   dependency, so they can be unit- and property-tested headlessly.
5. **One portable UI stack.** `egui` draws, `eframe` hosts. The same build runs
   everywhere eframe supports.
6. **Small, replaceable seams.** Services (file dialogs, print, clipboard,
   spell-check, AI) sit behind traits so they can be faked in tests.
7. **Rust-only.** The repository has exactly one implementation language.
   Repository tasks live in `crates/xtask` and run as `cargo xtask <task>`.

## 2. Workspace layout

```
NoteCraft/
├── Cargo.toml                 # workspace
├── rust-toolchain.toml        # pinned toolchain
├── crates/
│   ├── core/                  # nc-core — document model, buffer, undo, selection
│   ├── textproc/              # nc-textproc — encoding, EOL, search/replace, file I/O
│   ├── markdown/              # nc-markdown — loss-keeping Markdown parse and formatting
│   ├── commands/              # nc-commands — command registry, menus, shortcuts
│   ├── platform/              # nc-platform — the egui/eframe shell
│   ├── app/                   # nc-app — binary: startup, logging, lifecycle -> notepad
│   └── xtask/                 # repository tasks (cargo xtask check-boundaries)
├── docs/                      # this spec set
└── tests/                     # integration + fixtures (see test-strategy.md)
```

Crates that exist today: `crates/app` (`nc-app`), `crates/platform`
(`nc-platform`), `crates/commands` (`nc-commands`), `crates/core` (`nc-core`),
`crates/textproc` (`nc-textproc`), `crates/markdown` (`nc-markdown`), and
`crates/xtask`.

## 3. Layering

```
            nc-app  (startup, logging, lifecycle)
              │  depends on
          nc-platform  (window, chrome, menus, toolbar, editor surface)
              │  depends on
          nc-commands  (registry, menus, shortcuts, availability)
              │
        ┌─────┴──────┐
   nc-core ◄──── nc-textproc
```

`nc-core`, `nc-textproc`, `nc-commands`, and `nc-markdown` **must not** depend on
`nc-platform` or `nc-app`; this keeps them headless-testable and is enforced by
`cargo xtask check-boundaries`. `nc-markdown` depends on nothing at all — the
shell calls it directly for the formatting commands.

## 4. `nc-core` — document model

- `TextBuffer`: UTF-8 storage, byte-offset positions clamped to character
  boundaries, `\n` terminators internally.
- `Selection` / caret: positions, anchors, line/column projection.
- `UndoStack`: grouped, multilevel; edits store the removed text so undo restores
  text and caret without recomputing.
- `mouse` / `dnd`: the *meaning* of a click and a drop — word/line selection
  arithmetic (`ClickKind`, `click_selection`, `drag_selection`) and the
  insertion offset a drop lands on (`drop_target`). Hit-testing and the cross-
  application half stay in the shell.
- Overwrite mode: typing replaces one grapheme per grapheme typed and never eats
  the line terminator.
- `Document`: id, path, dirty flag, encoding, EOL policy, view mode.
- No I/O.

## 5. `nc-textproc` — text processing

Depends on `nc-core` and the standard library only, so a file can be read and
written without a window.

- `encoding`: `Encoding` (`Utf8`, `Utf16Le`, `Utf16Be`, `Utf32Le`, `Utf32Be`,
  `Ansi`) with detection (byte order mark first, then byte-pattern heuristics,
  then the ANSI code page) and transcoding both ways. A `Detection` records *why*
  the encoding was chosen so the status bar can say so. ANSI is Windows-1252, and
  every byte of it maps to a character and back, so no file can be silently
  corrupted.
- `TextFile`: the path, the bytes as they were on disk, the `Document` read from
  them, and the chosen encoding/BOM. `open` reads and detects, `open_as` reads as
  an explicitly chosen encoding (the Open dialog's list), `save`
  re-serialises in the encoding the file carries, `save_as` transcodes to a
  chosen encoding (with or without a mark), `save_lossy` writes what the target
  can hold, and `reload` re-reads the file.
- Writes are atomic per file: the text is serialised to a sibling temporary file
  which is renamed over the target only once it is complete, so a failed write
  leaves the previous contents intact.
- `ExternalChange`: whether the file on disk is still the one that was read.
- `prompt`: the decisions to put to the user as values — unsaved changes,
  characters a target encoding cannot hold, overwriting an existing file, and a
  file changed outside the editor. The wording and the box belong to the shell.
- `search`: `SearchOptions` (*Match case*, *Match whole word only*) with a
  `Direction`, `find_next`/`find_previous` returning `Found` / `Wrapped` /
  `Boundary` / `NoMatch`, `find_all` for Replace All, and `go_to_line`. `time_date`:
  `TimeDate`, formatted `3:47 PM 10/12/2026`.

## 6. `nc-markdown` — formatting model (M7)

- Parses the editable source into spans (a tolerant, loss-keeping model).
- Applies formatting commands by editing the *source*, never a rich internal
  tree that loses bytes; renders a formatted view derived from the source.
- Serialization is deterministic.

Landed in `crates/markdown` as M7's first step: `blocks` and `spans` for the
tolerant read (every block carries the exact bytes it came from, so a document
reads back byte for byte), and the formatting commands as source edits that return
new text plus the remapped selection — which is what lets the shell treat a
format as one ordinary undoable edit.

## 7. `nc-commands` — command layer

Dependency-free and host-agnostic (it depends on nothing outside the standard
library, so it is fully testable on any host).

- `CommandId` registry: each command carries its label, menu placement, shortcut,
  availability rule, and the milestone that first gives it an effect.
  `registry_integrity()` asserts the table is internally consistent, so the
  registry cannot drift silently.
- `Availability` predicates over document/session state, evaluated against an
  `AvailabilityContext` the shell supplies — "why is this greyed out?" is
  answerable without a window.
- `shortcut`: `Shortcut`, `Modifiers`, `Key`.
- `menu`: `build_menu_bar(context)` resolves the registry into the ordered tree
  the shell renders, applying availability and normalising separators at build
  time. The shell draws the menus from this model and routes a fired command
  back through `CommandId`.

## 8. `nc-platform` — the shell

The window, drawn with `egui` and hosted by `eframe`, is the only UI backend:

- `title_bar`: app icon, back chevron, the tab strip, the `+` new-tab button, and
  the window controls.
- `menu_bar`: the File/Edit/View menus (built from `nc-commands`), the formatting
  toolbar (paragraph styles, lists, B/I/S/link/table/clear), and the right-hand
  controls (AI, feedback, account, settings).
- `editor`: the zoomable, optionally-wrapped text surface.
- `status_bar`: caret position, character count, document kind, zoom, line
  endings, and encoding.
- `settings`: the two-column Settings page.
- `input`: the keyboard accelerator layer — it turns a key press into the
  `CommandId` the registry names for that key and runs it through `actions`, so
  the menu and the keyboard converge on one implementation. It dispatches only
  commands this build implements and that are available, and leaves the focused
  widget's own editing keys alone.
- `windows`: the extra top-level windows `File ▸ New window` opens. Each is an
  `egui` viewport with a `Shell` of its own, drawn every frame with
  `Context::show_viewport_immediate`, so a window's tabs and active document are
  independent of the main window's (`NC-F003`).
- `state`: the shell state (tabs, active document, view options) and the derived
  availability context.
- `actions`: the one place a fired command turns into a change in the shell.

`crates/platform/src/clock.rs` reads the system clock for the F5 time/date
insertion; it is kept out of `ui` because it is platform work, not drawing.

Services the shell will need (file dialogs, print, clipboard, spell-check) sit
behind traits so they can be faked in tests.

## 9. `nc-session` — tabs & windows (planned)

Tab model, window model, dirty tracking, restore persistence, recent-files list.
Currently the tab model lives in `nc-platform`'s shell state and the window model
in `nc-platform`'s `ui::windows`. A window is an `egui` viewport rather than a
separate process, so the shell stays one event loop.

## 10. `nc-ai` (M9)

Isolated, feature-gated; never on the critical startup path. Failures degrade to
non-AI behaviour. AI commands are present in the surface but greyed until the
service is configured.

## 11. Cross-cutting concerns

- **Logging:** local-only structured logs; no content leaves the machine.
- **Error handling:** `thiserror` in libraries, `anyhow` at the binary edge.
- **Threading:** the UI thread owns the window; long I/O and AI work on worker
  threads with progress/cancel; no ambiguity about which thread mutates a
  `Document`.

## 12. Build targets

One portable build via `egui`/`eframe`. The primary developer/test host is Linux
(`x86_64-unknown-linux-gnu`); the application builds and runs wherever eframe is
supported.
