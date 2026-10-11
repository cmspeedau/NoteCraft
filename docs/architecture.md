# Architecture — NoteCraft

A Windows-native, near-drop-in replacement for the modern Notepad app, written in Rust.
This document defines the module layout and boundaries. It is expected to be revised as
milestones land; every change to it must be justified by a reference behavior or an
engineering constraint, and reflected in the PR that makes the change.

## 1. Design principles

1. **Reference first.** Behavior is derived from `docs/reference-matrix.md`, never invented.
2. **Bytes are sacred.** File I/O preserves encoding/EOL unless the user changes them.
3. **Commands are central.** Every keyboard shortcut and menu item dispatches one
   `CommandId`; UI and keymap are thin adapters over a command table.
4. **Document model is testable without a window.** Editing primitives have no UI dependency
   so they can be unit- and property-tested headlessly.
5. **Native feel.** Uses Windows theming, DPI, and control conventions; not a web view.
6. **Small, replaceable seams.** Platform services (file dialogs, print, clipboard,
   spell-check, AI) sit behind traits so they can be faked in tests.
7. **Rust-only.** The repository has exactly one implementation language. Application code,
   tests, and maintenance tooling are all Rust; no shell, batch/PowerShell, or Python scripts
   are added. Repository tasks live in `crates/xtask` and run as `cargo xtask <task>`.

## 2. Workspace layout

```
notepad/
├── Cargo.toml                 # workspace
├── rust-toolchain.toml        # pinned toolchain
├── crates/
│   ├── nc-core/               # document model, buffer, undo, selection (platform-agnostic)
│   ├── nc-textproc/           # encoding detection, EOL handling, search/replace engine
│   ├── nc-markdown/           # markdown model: syntax view <-> formatted view, serialization
│   ├── nc-commands/           # command registry, CommandId, availability, keymap table
│   ├── nc-platform/           # Windows services: file dialogs, clipboard, print, DPI, UIA
│   ├── nc-settings/           # persisted preferences
│   ├── nc-session/            # tabs, windows, restore, recent files
│   ├── nc-ai/                 # optional service-backed AI commands (M9)
│   ├── nc-app/                # binary: window, chrome, wiring  -> notepad.exe
│   └── xtask/                 # repository tasks (cargo xtask check-boundaries)
├── docs/                      # this spec set
└── tests/                     # integration + fixtures (see test-strategy.md)
```

Crates that exist today: `crates/app` (`nc-app`), `crates/platform` (`nc-platform`),
`crates/commands` (`nc-commands`, landed in M2), `crates/core` (`nc-core`, landed in M3) and
`crates/xtask`. The rest are planned.

## 3. Layering

```
        nc-app  (window, toolbar, tab strip, menus, dialogs)
          │  depends on
   ┌──────┼───────────────┬──────────────┐
nc-session  nc-commands  nc-settings   nc-platform (Win32/UIA)
   │            │            │
   └────────────┴──────┬─────┘
                  nc-core ◄── nc-textproc
                     ▲
                nc-markdown          nc-ai (isolated, optional)
```

`nc-core`, `nc-textproc`, `nc-markdown` and `nc-commands` **must not** depend on `nc-platform`
or `nc-app`; this keeps them headless-testable and is enforced by `cargo xtask check-boundaries`.

## 4. `nc-core` — document model

- `Buffer`: UTF-8 (or transcoded) text storage; ro or rw.
- `Selection` / `Caret`: positions, anchors, line/column projection.
- `UndoStack`: grouped, multilevel, coalescing rules mirroring the reference.
- `mouse` / `dnd`: the *meaning* of a click and a drop — word/line selection
  arithmetic (`ClickKind`, `click_selection`, `drag_selection`) and the insertion
  offset a drop lands on (`drop_target`). Hit-testing, the gesture itself, and the
  cross-application half stay in the viewport and the platform, so the window shell
  never re-implements selection math and the engine stays testable off Windows.
  The reference's double/triple-click rule (`NC-E005`) and its copy-on-drag
  modifier (`NC-E009`) are capture items, so the engine names what it does an
  approximation: a double-click takes one UAX #29 word segment, a triple-click the
  whole line including its terminator, and an internal drop *moves* — the copy case
  is reachable through the clipboard commands.
- Overwrite mode: typing replaces one grapheme per grapheme typed and never eats the
  line terminator. The reference's behaviour is not captured and no matrix row covers
  it yet, so the guarantee is deliberately narrow.
- `Document`: id, path, dirty flag, encoding, EOL policy, view mode.
- No I/O, no allocation policy leaks (all mutation APIs return new state or take `&mut`).

## 5. `nc-textproc` — text processing

- `Encoding` enum + detection (BOM, heuristics, ANSI code page).
- `Eol` detection and preservation (CRLF/LF/CR, mixed).
- Search/replace engine (match case, whole word, direction, wrap notices).
- Transcoding on Save-As.

## 6. `nc-markdown` — formatting model

- Parses the editable source into spans (a tolerant, loss-keeping model).
- Applies formatting commands by editing the *source* (never a rich internal tree that
  loses bytes); renders a formatted view derived from the source.
- Serialization is deterministic and matches reference output (`reference-matrix.md` §3.5).

## 7. `nc-commands` — command layer

Landed in M2 as `crates/commands`, dependency-free and host-agnostic (it depends on nothing
outside the standard library, so it is fully testable on any host).

- `CommandId` registry (see `reference-matrix.md` §3 tables). Each row carries its evidence grade
  and spec status, and `registry_integrity()` asserts the table is internally consistent, so the
  registry cannot drift silently from the captured reference.
- `Availability` predicates over document/session state, evaluated against an `AvailabilityContext`
  the shell supplies — "why is this greyed out?" is answerable without a window.
- `shortcut`: `Shortcut`, `Modifiers`, `Key` and the Win32 virtual-key numbers. Data lives here;
  the native shell only maps a key press to a virtual key, so key handling is testable off Windows.
- `menu`: `build_menu_bar(context)` resolves the registry into the ordered tree a shell renders,
  applying availability and normalising separators at build time.
- Menus and toolbar are generated from the same registry. `MenuBar` is the static node tree keyed
  by `CommandId`; the shell translates it (Win32 `AppendMenuW`) and routes `WM_COMMAND` ids back
  through `by_menu_id`.

## 8. `nc-platform` — Windows services

Traits + Win32 implementations: file dialogs, clipboard, print, DPI/theme, UIA exposure,
shell association, single-instance activation. Fake implementations live in `tests/`.

## 9. `nc-session` — tabs & windows

Tab model, window model, dirty tracking, restore persistence, recent-files list.

## 10. `nc-ai` (M9)

Isolated, feature-gated; never on the critical startup path. Failures degrade to
non-AI behavior. Behavior gated on the reference's service availability states.

## 11. Cross-cutting concerns

- **Logging/telemetry:** local-only structured logs; no content leaves the machine.
- **Error handling:** `thiserror` in libraries, `anyhow` at the binary edge; user-facing
  errors reproduced from reference wording.
- **Threading:** UI thread owns the window; long I/O and AI on worker threads with
  progress/cancel; no ambiguity about which thread mutates a `Document`.

## 12. Build targets

- Primary: `x86_64-pc-windows-msvc`.
- Secondary: `aarch64-pc-windows-msvc`.
- Developer/test host: Linux (`x86_64-unknown-linux-gnu`) for headless crates via fakes.
