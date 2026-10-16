# AGENT.md — NoteCraft Engineering Rules

## 1. Mission

You are the implementation agent for NoteCraft, a fast, modern text and Markdown
editor written in Rust and drawn with `egui`/`eframe`.

Your primary objective is a correct, maintainable editor whose UI matches the
captured reference in `docs/ui-reference.md` and whose behaviour is backed by
tests. Do not optimise for the number of features implemented, the number of
files created, or how quickly the application looks convincing. Optimise for
correctness, reproducibility, data integrity, and test coverage.

`ROADMAP.md` defines the milestones. `docs/ui-reference.md` defines the UI to
build. `docs/feature-matrix.md` is the source of truth for implementation and
verification status. `crates/commands` is the single authoritative definition of
the command surface.

## 2. Non-negotiable rules

- Rust is the only implementation language. Do not introduce other languages,
  runtimes, or script interpreters anywhere in the repository — application code,
  tests, build tooling, and CI included. All repository automation is written in
  Rust under `crates/xtask` and invoked as `cargo xtask <task>`.
- Do not invent UI that the reference does not show. Draw from
  `docs/ui-reference.md` and the command registry.
- Do not mark a feature `VERIFIED` without passing acceptance tests.
- Never silently discard document content, change encoding, or normalise line
  endings unexpectedly.
- Do not leave stub implementations, fake success responses, or inert menu items
  in completed features. A control that is present but not yet wired must be
  visibly disabled (its availability rule says so), not a silent no-op.
- Preserve existing working behaviour unless a documented requirement demands a
  change.
- Keep the headless crates (`nc-core`, `nc-textproc`, `nc-commands`) free of any
  window dependency.

## 3. Required workflow

### Step 1 — Inspect

- Read the relevant parts of `ROADMAP.md`, `docs/ui-reference.md`, and
  `docs/feature-matrix.md`.
- Inspect the current implementation and tests before editing.
- Identify the feature ID and the expected behaviour.

### Step 2 — Specify

- Describe the intended observable behaviour.
- Record the relevant keyboard shortcuts, menu commands, dialogs, and state
  transitions.
- Identify data-loss risks.
- Define tests alongside the implementation.

If the reference behaviour is not shown in `docs/ui-reference.md`, decide it and
record the decision in the registry — do not guess silently.

### Step 3 — Implement

- Make the smallest coherent change that achieves the specification.
- Follow existing module boundaries and coding conventions.
- Reuse established abstractions instead of duplicating state or command logic.
- Keep UI event handling separate from document and filesystem logic.
- Add explicit error paths for expected failures.
- Avoid unrelated refactoring.

### Step 4 — Verify

Run the relevant checks:

```
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-boundaries
```

Do not report a check as passing unless it was executed and passed. When a check
cannot run, state the limitation.

### Step 5 — Record

- Add or update regression tests.
- Update `docs/feature-matrix.md`.
- Document known limitations.
- Update `ROADMAP.md` only when scope or milestone status actually changes.
- Summarise files changed, tests run, results, and remaining risks.

## 4. Architecture rules

### Application and UI

- Keep command handling explicit and testable.
- Separate menu presentation from command execution.
- Use a single authoritative source for each piece of application state.
- Ensure keyboard shortcuts and menu commands invoke the same command
  implementation.
- Keep window, tab, and document lifecycles distinct.

### Editor

- Treat text editing as a correctness-critical subsystem.
- Specify caret and selection behaviour precisely.
- Test keyboard navigation, mouse interaction, word boundaries, and line
  boundaries.
- Preserve Unicode semantics, including surrogate pairs, combining characters,
  emoji, and IME composition.
- Do not confuse UTF-8 byte offsets, UTF-16 code-unit offsets, Unicode scalar
  values, and grapheme-cluster indices. Convert offsets explicitly at API
  boundaries.
- Keep undo/redo history consistent with visible edits.
- Avoid whole-document copies for routine operations where practical.

### Document and file I/O

- Keep document contents, file identity, encoding, line endings, and dirty state
  explicit.
- Separate decoding, editing representation, and serialization.
- Preserve source bytes where the behaviour requires it.
- Test BOM handling, invalid sequences, empty files, and mixed or unusual line
  endings.
- Never overwrite an existing file before the replacement data is safely
  prepared. Write atomically.
- Handle permissions, missing directories, read-only files, and failed writes
  explicitly.
- Do not promise perfect byte preservation for malformed input unless the
  implementation actually supports it.

### Markdown

- Treat Markdown syntax and formatted presentation as separate but synchronised
  representations.
- Never make irreversible transformations during a view switch.
- Preserve unsupported syntax and unknown content.
- Keep Markdown parsing and serialization deterministic.
- Ensure formatting operations participate in undo/redo.

## 5. Dependency policy

Before adding a crate:

- Check whether the functionality already exists in the project.
- Confirm the crate is maintained, appropriately licensed, and small enough for
  the job.
- Prefer a small, well-maintained dependency over a large framework.
- Pin toolchain and dependency versions through the normal Rust workflow.

Do not change the UI framework, text engine, or core document representation
without a documented technical reason and an impact assessment.

## 6. Testing requirements

### Unit tests

Test document state, encoding, line-ending logic, text operations, selection
transformations, search, and serialization independently of the UI.

### Integration tests

Test opening, editing, saving, switching tabs, closing documents, settings, and
application lifecycle.

### UI tests

- Render the shell headlessly through the egui context and assert it paints.
- Compare the drawn layout against `docs/ui-reference.md` screen by screen.
- Separate functional defects from rendering-only differences.

### Stress and edge-case tests

Include empty documents, very large files, extremely long lines, Unicode and
mixed scripts, CRLF/LF/CR, files with and without BOMs, invalid or truncated
input, repeated undo and redo, search at document boundaries, failed saves and
read-only files, and repeated tab opening and closing.

Use property-based or randomized tests where useful. Keep failing seeds or
minimized reproductions as permanent regressions.

## 7. Keyboard and command consistency

Maintain a single command registry or equivalent authoritative specification.

For each command, record:

- Command ID
- Menu location
- Keyboard shortcut
- Availability conditions
- Expected state changes
- Undo/redo behaviour
- Acceptance test

Do not implement keyboard shortcuts as unrelated UI-specific handlers. Account
for keyboard layouts, modifier combinations, text-input contexts, and shortcut
conflicts.

## 8. Error handling and data safety

- Never silently ignore a failed save.
- Never silently truncate or replace document text.
- Never discard unsaved changes without confirmation.
- Never report a file as saved before the write operation succeeds.
- Preserve the original file when a replacement write fails.
- Show useful, actionable errors.
- Avoid logging document contents or sensitive file data.
- Treat external file modifications as a potential conflict, not permission to
  overwrite automatically.

## 9. Performance requirements

Measure before optimizing.

Track startup time, idle memory, typing latency, scrolling performance, search
latency, save latency, and large-file behaviour. Use representative fixtures and
repeatable benchmarks. Avoid premature optimization that changes editing
semantics or complicates correctness.

Long-running work must not freeze the UI. Any background operation must handle
cancellation, completion, errors, and stale document state safely.

## 10. Security and privacy

- Do not add network access unless it is explicitly required by scope.
- Keep optional online features isolated from core editing.
- Never upload document contents without explicit, informed user consent.
- Do not introduce telemetry without a documented requirement.
- Validate external input and command-line paths.
- Avoid executing file contents or treating document text as instructions.
- Do not store secrets in source control, logs, or test fixtures.

## 11. Feature status rules

Use these states consistently:

- `NOT_STARTED` — no implementation exists.
- `SPECIFIED` — behaviour and acceptance criteria are documented.
- `IN_PROGRESS` — active implementation work.
- `IMPLEMENTED` — code exists but is not fully verified.
- `VERIFIED` — acceptance tests pass.
- `BLOCKED` — a specific dependency or limitation prevents completion.
- `OUT_OF_SCOPE` — explicitly excluded.

Never mark a feature `VERIFIED` merely because the application compiles or a
single unit test passes.

## 12. Definition of Done

A task is done only when:

- Its behaviour matches the documented specification.
- The implementation is integrated into the real application.
- Relevant tests pass.
- Regression coverage is added.
- File integrity and undo/redo behaviour are preserved.
- Relevant keyboard and menu paths work.
- Formatting, compilation, and lint checks pass.
- Documentation and feature status are updated.
- Remaining limitations are disclosed.

## 13. Agent reporting format

At the end of each implementation task, report:

- Implemented: the behaviour added or corrected.
- Files changed: the relevant source and test files.
- Verification: exact checks executed and their results.
- Status: verified, implemented but unverified, or blocked.
- Known differences: remaining deviations from the reference.
- Next step: the highest-priority unresolved dependency or feature.

Do not claim tests, benchmarks, screenshots, or manual comparisons that were not
actually performed.

## 14. Final principle

When speed, convenience, elegance, and correctness conflict, follow the
specification and protect document integrity first.

The project succeeds when the UI matches the captured reference and every
behaviour is backed by tests — not when the UI merely looks familiar.
