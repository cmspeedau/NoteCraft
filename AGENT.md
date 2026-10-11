AGENTS.md — Rust Notepad Engineering Rules
1. Mission

You are the implementation agent for a Rust application that reproduces the observable behavior of a specific Windows Notepad reference build.

Your primary objective is verified 1:1 feature parity.

Do not optimize for the number of features implemented, the number of files created, or how quickly the application looks convincing. Optimize for correctness, reproducibility, data integrity, native Windows behavior, and test coverage.

ROADMAP.md defines the project milestones. docs/reference-matrix.md defines the target behavior. docs/feature-matrix.md is the source of truth for implementation and verification status.
2. Non-Negotiable Rules

    Rust is the only implementation language. Do not introduce other languages, runtimes, or script interpreters anywhere in the repository — application code, tests, build tooling, and CI included. Do not add shell scripts, batch/PowerShell files, Python, or other-language tooling. All repository automation is written in Rust under `crates/xtask` and invoked as `cargo xtask <task>`.
    Target Windows first; do not introduce cross-platform abstractions without a concrete need.
    Do not invent reference behavior. Inspect the pinned reference application or consult reliable documentation.
    Do not mark a feature VERIFIED without a passing reference-derived test.
    Do not claim full parity while mandatory features remain unverified.
    Never silently discard document content, change encoding, or normalize line endings unexpectedly.
    Do not replace native dialogs or Windows behaviors with approximations unless the reference specification allows it.
    Do not add features that are absent from the reference scope simply because they are easy to implement.
    Do not leave stub implementations, fake success responses, or inert menu items in completed features.
    Preserve existing working behavior unless a documented parity requirement demands a change.

3. Required Workflow

For every task:
Step 1 — Inspect

    Read the relevant parts of ROADMAP.md, docs/reference-matrix.md, and docs/feature-matrix.md.
    Inspect the current implementation and tests before editing.
    Identify the reference feature ID and expected behavior.
    Check whether the behavior depends on a specific Windows or Notepad version.

Step 2 — Specify

    Describe the intended observable behavior.
    Record relevant keyboard shortcuts, menu commands, dialogs, state transitions, and edge cases.
    Identify data-loss risks and compatibility concerns.
    Define tests before or alongside the implementation.

If the reference behavior is unknown, investigate it. Do not fill gaps with guesses and then call the result parity.
Step 3 — Implement

    Make the smallest coherent change that achieves the specification.
    Follow existing module boundaries and coding conventions.
    Reuse established abstractions instead of duplicating state or command logic.
    Keep UI event handling separate from document and filesystem logic.
    Add explicit error paths for expected failures.
    Avoid unrelated refactoring.

Step 4 — Verify

Run the relevant checks, including where applicable:

cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask check-boundaries

Run Windows-specific integration tests on the actual supported target. Do not report a check as passing unless it was executed and passed.

When a check cannot run, state the limitation and leave the relevant verification status unchanged.
Step 5 — Record

    Add or update regression tests.
    Update docs/feature-matrix.md.
    Document known mismatches and platform limitations.
    Update ROADMAP.md only when scope or milestone status actually changes.
    Summarize files changed, tests run, results, and remaining risks.

4. Architecture Rules
Application and UI

    Keep command handling explicit and testable.
    Separate menu presentation from command execution.
    Use a single authoritative source for each piece of application state.
    Ensure keyboard shortcuts and menu commands invoke the same command implementation.
    Keep window lifecycle, tab lifecycle, and document lifecycle distinct.
    Respect Windows focus, activation, DPI, theme, and accessibility conventions.

Editor

    Treat text editing as a correctness-critical subsystem.
    Specify caret and selection behavior precisely.
    Test keyboard navigation, mouse interaction, word boundaries, and line boundaries.
    Preserve Unicode semantics, including surrogate pairs, combining characters, emoji, and IME composition.
    Do not confuse UTF-8 byte offsets, UTF-16 code-unit offsets, Unicode scalar values, and grapheme-cluster indices.
    Use appropriate indexing abstractions and convert offsets explicitly at API boundaries.
    Keep undo/redo history consistent with visible edits and document state.
    Avoid whole-document copies for routine operations where practical.

Document and file I/O

    Keep document contents, file identity, encoding, line endings, and dirty state explicit.
    Separate decoding, editing representation, and serialization.
    Preserve source bytes where the reference behavior requires it.
    Test BOM handling, invalid sequences, empty files, and mixed or unusual line endings.
    Never overwrite an existing file before the replacement data is safely prepared.
    Handle permissions, missing directories, read-only files, and failed writes explicitly.
    Implement atomic replacement where appropriate and compatible with the reference.
    Do not promise perfect byte preservation for malformed input unless the implementation actually supports it.

Markdown

    Treat Markdown syntax and formatted presentation as separate but synchronized representations.
    Never make irreversible transformations during a view switch.
    Preserve unsupported syntax and unknown content.
    Test tables, links, lists, indentation, malformed input, and mixed formatting if they are in scope.
    Keep Markdown parsing and serialization deterministic.
    Ensure formatting operations participate in undo/redo.
    Do not assume a general-purpose Markdown renderer matches Notepad's formatting behavior.

Windows integration

    Use documented Windows APIs and supported Rust bindings.
    Keep unsafe blocks narrow and documented.
    Validate pointer lifetimes, buffer lengths, ownership, and error codes at FFI boundaries.
    Respect UI-thread affinity where required.
    Do not block the UI thread on expensive file operations.
    Test native dialogs, clipboard behavior, drag and drop, and accessibility on Windows.
    Do not claim Linux CI proves Windows integration correctness.

5. Dependency Policy

Before adding a crate:

    Check whether the functionality already exists in the project.
    Confirm that the crate supports the required Windows versions and architectures.
    Check maintenance status, licensing, security concerns, and transitive dependencies.
    Verify that it can satisfy the parity requirements.
    Prefer a small, well-maintained dependency over a large framework when both are suitable.
    Pin toolchain and dependency versions through the repository's normal Rust workflow.

Do not change the UI framework, text engine, or core document representation without a documented technical reason and an impact assessment.
6. Testing Requirements
Unit tests

Test document state, encoding, line-ending logic, text operations, selection transformations, search, and serialization independently of the UI wherever possible.
Integration tests

Test opening, editing, saving, switching tabs, closing documents, settings persistence, and application lifecycle.
Differential tests

For each supported reference behavior:

    Prepare an identical initial state in both applications.
    Execute the same action sequence.
    Capture resulting text, selection, document state, prompts, and saved bytes.
    Compare results against documented expectations.
    Save every discovered mismatch as a regression test.

Visual tests

    Use matching display resolution and scaling.
    Capture equivalent window states.
    Compare layout, spacing, typography, menus, dialogs, and status indicators.
    Separate functional mismatches from rendering-only differences.
    Define explicit tolerances rather than relying only on subjective judgment.

Stress and edge-case tests

Include:

    Empty documents
    Very large files
    Extremely long lines
    Unicode and mixed scripts
    CRLF, LF, and CR
    Files with and without BOMs
    Invalid or truncated input
    Repeated undo and redo
    Search at document boundaries
    Failed saves and read-only files
    Repeated tab opening and closing
    Rapid keyboard input and window resizing

Use property-based or randomized tests where useful. Keep failing seeds or minimized reproductions as permanent regressions.
7. Keyboard and Command Parity

Maintain a single command registry or equivalent authoritative specification.

For each command, record:

    Command ID
    Menu location
    Keyboard shortcut
    Availability conditions
    Expected state changes
    Dialog behavior
    Undo/redo behavior
    Reference test
    Implementation status

Do not implement keyboard shortcuts as unrelated UI-specific handlers. Account for keyboard layouts, modifier combinations, text-input contexts, and shortcut conflicts.
8. Error Handling and Data Safety

    Never silently ignore a failed save.
    Never silently truncate or replace document text.
    Never discard unsaved changes without the specified confirmation behavior.
    Never report a file as saved before the write operation succeeds.
    Preserve the original file when a replacement write fails, where feasible.
    Show useful, actionable errors consistent with the reference UI.
    Avoid logging document contents or sensitive file data.
    Treat external file modifications as a potential conflict, not permission to overwrite automatically.

9. Performance Requirements

Measure before optimizing.

Track startup time, idle memory, typing latency, scrolling performance, search latency, save latency, and large-file behavior.

Use representative fixtures and repeatable benchmarks. Avoid premature optimization that changes editing semantics or complicates correctness.

Long-running work must not freeze the UI. Any background operation must handle cancellation, completion, errors, and stale document state safely.
10. Security and Privacy

    Do not add network access unless it is explicitly required by the reference scope.
    Keep optional online features isolated from core editing.
    Never upload document contents without explicit, informed user consent.
    Do not introduce telemetry without a documented requirement and privacy review.
    Validate external input and command-line paths.
    Avoid executing file contents or treating document text as instructions.
    Do not store secrets in source control, logs, or test fixtures.

11. Feature Status Rules

Use the following states consistently:

    NOT_STARTED — no implementation exists.
    SPECIFIED — reference behavior and acceptance criteria are documented.
    IN_PROGRESS — active implementation work.
    IMPLEMENTED — code exists but parity is not fully verified.
    VERIFIED — acceptance tests pass against the reference.
    BLOCKED — a specific dependency or limitation prevents completion.
    OUT_OF_SCOPE — explicitly excluded from the agreed target.

Never mark a feature VERIFIED merely because the application compiles or a unit test passes.
12. Definition of Done

A task is done only when:

    Its behavior matches the documented reference specification.
    The implementation is integrated into the real application.
    Relevant tests pass.
    Regression coverage is added.
    File integrity and undo/redo behavior are preserved.
    Relevant keyboard and menu paths work.
    Formatting, compilation, and applicable lint checks pass.
    Documentation and feature status are updated.
    Remaining mismatches and limitations are disclosed.

13. Agent Reporting Format

At the end of each implementation task, report:

    Implemented: the behavior added or corrected.
    Files changed: the relevant source and test files.
    Verification: exact checks executed and their results.
    Parity status: verified, implemented but unverified, or blocked.
    Known differences: remaining deviations from the reference.
    Next step: the highest-priority unresolved dependency or feature.

Do not claim tests, benchmarks, screenshots, or manual comparisons that were not actually performed.
14. Final Principle

When speed, convenience, elegance, and compatibility conflict, follow the reference specification and protect document integrity first.

The project succeeds when its observable behavior is demonstrably equivalent to the pinned Windows Notepad version—not when the UI merely looks familiar.
