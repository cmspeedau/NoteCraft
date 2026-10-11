# Test Strategy — NoteCraft

Parity is proven by **differential testing against the reference**, not by the author's
opinion. This document defines how "done" is measured.

## 1. Levels

| Level | Scope | Runs on | Tooling |
| --- | --- | --- | --- |
| Unit | `nc-core`, `nc-textproc`, `nc-markdown`, `nc-commands` logic | any host | `cargo test` + `proptest` |
| Golden / fixture | byte-exact I/O, markdown serialization, search semantics | any host | `insta`/`expect-test` + `tests/fixtures/` |
| Integration (headless) | command dispatch, session/undo flows via fake platform | any host | `cargo test` |
| UI / visual | window chrome, toolbar, dialogs, DPI, theme | Windows | `windows-rs` driver + screenshot diff |
| Differential | same input → reference vs ours, compare observable output | Windows + reference | `xtask differential` harness |
| Accessibility | UIA names, focus order, screen-reader narration | Windows | Inspect.exe / UIA client test |

## 2. Reference-derived acceptance

- Every feature in `docs/feature-matrix.md` has acceptance criteria and a named test file.
- A feature is `VERIFIED` only when a test exercises the behavior **captured from the
  reference** (`CAP`), with the fixture committed.
- `TO_CAPTURE` features cannot be `VERIFIED` and cannot be implemented from a guess.

## 3. Fixtures

`tests/fixtures/` layout:

```
tests/fixtures/
├── encoding/     # byte files: {utf8,utf8-bom,utf16le,utf16be,ansi,empty,large}.txt + .expected
├── eol/          # crlf.txt, lf.txt, cr.txt, mixed.txt + .expected save bytes
├── markdown/     # <name>.md (source) and <name>.expected.md (after command)
├── find/         # input + query + expected match spans/counts
├── visual/       # reference screenshots + our screenshots + diff tolerance
└── errors/       # expected-dialog-text transcripts
```

Every fixture records: reference app version, Windows build, date captured, capturer.

## 4. Differential harness (`xtask differential`)

1. Provide an input file + a scripted command sequence (keyboard/menu actions).
2. Apply the sequence to the reference (recorded once, human-in-the-loop) and to our binary.
3. Compare: saved bytes, markdown output, status-bar values, and any dialog text.
4. Emit a diff report → `docs/parity-report.md` table row.

Reference runs are recorded manually on the reference machine the first time, then frozen
as golden data; the harness replays against our binary thereafter.

## 5. Property tests (call these out explicitly)

- Encoding round-trip: `decode(encode(x)) == x` for the encodings that can represent `x`;
  otherwise the documented lossy rule holds.
- EOL preservation: saving an unmodified file yields identical bytes.
- Undo/redo: applying N edits then undoing N returns to the byte-identical starting buffer.
- Markdown formatting idempotence where the reference is idempotent (capture first).
- Search: every reported match contains the query under the active options.

Encoding round-trip, EOL preservation, text-through-save-and-reopen, undo-to-start, and
"a save leaves no partial file" run today in `crates/textproc/tests/properties.rs` (M4) over
arbitrary byte strings and generated text; the rest arrive with their engines.

## 6. CI

- On every push: `cargo fmt --check`, `cargo clippy -D warnings`, unit + golden tests.
- Dependency-boundary check: `nc-core`/`nc-textproc`/`nc-markdown`/`nc-commands` must not
  depend on `nc-app`/`nc-platform` (`cargo xtask check-boundaries`, run in CI).
- Windows runner job: build + integration + visual smoke.
- Visual/AI/differential suites are gated (manual or nightly) because they need the
  reference machine or the service.

## 7. Manual test procedures (for `TO_CAPTURE` items)

Each still-uncaptured item in `reference-matrix.md` gets a numbered manual procedure in
`tests/manual/` describing exactly what to do and what to record, so capture is
reproducible and does not drift between operators.

## 8. Definition of done (per feature)

1. Spec entry exists in `docs/reference-matrix.md` (not `TO_CAPTURE`).
2. Acceptance test exists and passes.
3. Differential evidence recorded (reference vs ours) where the level applies.
4. `docs/feature-matrix.md` status is `VERIFIED`, or `IMPLEMENTED` with a tracked reason.
5. `docs/parity-report.md` row updated.
6. CI green.
