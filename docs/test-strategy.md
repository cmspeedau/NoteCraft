# Test Strategy — NoteCraft

"Done" is proven by tests. This document defines how it is measured.

## 1. Levels

| Level | Scope | Runs on | Tooling |
| --- | --- | --- | --- |
| Unit | `nc-core`, `nc-textproc`, `nc-commands` logic | any host | `cargo test` + `proptest` |
| Golden / fixture | byte-exact I/O, search semantics | any host | `tests/fixtures/` |
| Integration | command dispatch, file/session flows | any host | `cargo test` |
| UI | the shell: chrome, toolbar, menus, settings | any host | egui headless render (`Context::run_ui`) |
| Visual | layout compared with `docs/ui-reference.md` | any host | screenshot diff |

The UI is drawn with egui, so a frame can be rendered headlessly and asserted
(the shell's own tests already do this). No platform-specific runner is required.

## 2. Acceptance

- Every feature in `docs/feature-matrix.md` has acceptance criteria and a named
  test file.
- A feature is `VERIFIED` only when its tests exercise the acceptance criteria
  and pass.
- Behaviour the reference does not show is a recorded design decision, tested
  like any other.

## 3. Fixtures

`tests/fixtures/` layout:

```
tests/fixtures/
├── encoding/     # byte files: {utf8,utf8-bom,utf16le,utf16be,ansi,empty,large}.txt + .expected
├── eol/          # crlf.txt, lf.txt, cr.txt, mixed.txt + .expected save bytes
├── markdown/     # <name>.md (source) and <name>.expected.md (after command)
├── find/         # input + query + expected match spans/counts
└── visual/       # reference screens + our screenshots + diff tolerance
```

Each fixture records what it covers and, where relevant, the reference screen it
came from.

## 4. UI checks

1. Render the shell headlessly through the egui context for each major state
   (empty editor, menus open, settings page, Markdown document).
2. Snapshot the frame and compare it against the matching screen in
   `docs/ui-reference.md`.
3. Compare layout, spacing, typography, menus, and status indicators.
4. Separate functional defects from rendering-only differences.

## 5. Property tests (call these out explicitly)

- Encoding round-trip: `decode(encode(x)) == x` for the encodings that can
  represent `x`; otherwise the documented lossy rule holds.
- EOL preservation: saving an unmodified file yields identical bytes.
- Undo/redo: applying N edits then undoing N returns to the byte-identical
  starting buffer.
- Search: every reported match contains the query under the active options.

Encoding round-trip, EOL preservation, text-through-save-and-reopen, undo-to-
start, and "a save leaves no partial file" run in
`crates/textproc/tests/properties.rs`; the search property compares `find_all`
against `str::match_indices` over generated text; the rest arrive with their
engines.

## 6. CI

- On every push: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test
  --workspace`.
- Dependency-boundary check: `nc-core` / `nc-textproc` / `nc-commands` must not
  depend on `nc-app` / `nc-platform` (`cargo xtask check-boundaries`).
- Visual checks run locally/nightly because they need a rendered frame and a
  comparison image.

## 7. Definition of done (per feature)

1. Spec entry exists in `docs/ui-reference.md` or the command registry.
2. Acceptance test exists and passes.
3. `docs/feature-matrix.md` status is `VERIFIED`, or `IMPLEMENTED` with a tracked
   reason.
4. CI green.
