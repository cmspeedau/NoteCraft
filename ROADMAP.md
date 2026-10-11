Rust Notepad — Feature-Parity Roadmap
1. Project Goal

Build a Windows-native text editor in Rust that achieves 1:1 feature parity with a defined reference version of Microsoft Windows Notepad.

The application must reproduce the reference application's observable behavior, including:

    UI layout, menus, dialogs, and window behavior
    Text editing, selection, clipboard, and undo/redo
    File opening, saving, encoding, and line endings
    Tabs, session restoration, and unsaved-document handling
    Search, replace, navigation, and status information
    Markdown editing and formatting, where supported by the reference build
    Accessibility, keyboard navigation, and Windows integration
    Performance, reliability, and edge-case behavior

Parity means behavioral equivalence, not merely visual similarity.
2. Reference Baseline

Before implementation, record the exact reference environment.

    Operating system: Windows 11
    Reference application: Microsoft Notepad
    Reference app version: [record exact version]
    Windows build: [record exact build]
    Display scaling: 100%, 150%, and 200%
    Architectures: x64 initially; ARM64 where required
    Languages and keyboard layouts: English initially, then additional layouts
    Feature availability: record stable, preview, and optional features separately

Maintain docs/reference-matrix.md containing screenshots, observed behavior, shortcuts, menu structures, test cases, and version-specific differences.

Do not claim complete parity until every in-scope feature has passed its acceptance tests.
3. Technology Strategy
Language and platform

    Rust stable, pinned through rust-toolchain.toml
    Windows APIs through the windows crate
    Native Windows integration for dialogs, clipboard, accessibility, DPI, and window management
    Cargo workspaces for modularity
    Automated tests and CI for Windows x64
    ARM64 validation when supported by the chosen dependencies

UI and text engine

Evaluate two implementations before committing:

    Native Win32 UI with a custom text engine: maximum control over behavior and performance, but substantial engineering effort.
    Native Windows controls with a compatibility layer: faster initial development, but only acceptable if the controls can reproduce the reference behavior.

Do not assume a standard Rust text widget or RichEdit control automatically provides Notepad parity. Verify selection semantics, line endings, Unicode handling, undo history, large-file performance, and accessibility experimentally.
Architecture

Separate the application into these modules:

    app — startup, process lifecycle, and command-line arguments
    ui — windows, menus, toolbar, tabs, dialogs, and status bar
    editor — document model, text engine, selection, caret, and editing commands
    document — file state, dirty tracking, and document identity
    encoding — encoding detection, decoding, and serialization
    filesystem — file operations, metadata, and error handling
    search — find, replace, navigation, and search state
    markdown — Markdown parsing, editing, formatting, and view switching
    session — tabs, recent files, and session restoration
    settings — preferences, theme, font, and persistence
    platform — Windows APIs, clipboard, shell integration, and accessibility
    tests — behavioral fixtures, regression tests, and parity tests

Keep platform-specific behavior behind narrow interfaces. Avoid duplicating document state across the UI and text engine.
4. Milestones
M0 — Specification and reference capture

    Pin the target Notepad version and Windows build.
    Capture screenshots of every major window, menu, dialog, and settings page.
    Record all visible commands and keyboard shortcuts.
    Test each command against the reference application.
    Record text editing and file-format edge cases.
    Create a feature inventory with IDs, priorities, and acceptance criteria.
    Separate mandatory features from optional or service-dependent features.

Exit criteria: Every in-scope feature has a specification and a reproducible test or documented verification procedure.

Status: IN_PROGRESS — specification, feature inventory, architecture, and test strategy drafted
(`docs/reference-matrix.md`, `docs/feature-matrix.md`, `docs/architecture.md`, `docs/test-strategy.md`,
`docs/parity-report.md`). Reference capture on a Windows reference machine is outstanding
(see `docs/reference-matrix.md` §7 and §1 "PINNING ACTION"); items tagged TO_CAPTURE are not yet
reproducible, so M0 cannot close until they are observed and committed as fixtures.

M1 — Rust foundation

    Create the Cargo workspace and Windows executable.
    Configure logging, error reporting, and release builds.
    Establish UI-thread and background-work boundaries.
    Implement process startup and shutdown.
    Support opening files through command-line arguments and Windows shell integration where required.
    Add unit-test, integration-test, and CI infrastructure.

Exit criteria: The application launches reliably, opens an initial window, and passes clean Windows builds.

Status: IN_PROGRESS — Cargo workspace (`crates/app` = `nc-app` with the `notepad` binary,
`crates/platform` = `nc-platform`), command-line handling, logging, startup/shutdown lifecycle,
unit + integration tests, and CI (fmt, clippy, host tests, Windows build/test, Windows
cross-compile check, crate dependency-direction check) are in place and green on Linux
(`cargo build/test/clippy/check --target x86_64-pc-windows-msvc`). A headless platform backend
keeps the workspace buildable off Windows; creating a real window is exercised by the Windows CI
job.

M2 — Native window and visual shell

    Main window and title bar behavior
    Client area, menus, and command routing
    Editor viewport and scrollbars
    Status bar
    Context menus
    Standard open/save dialogs
    Resizing, maximizing, minimizing, and restoring
    High-DPI scaling and theme handling
    Window focus, activation, and keyboard navigation

Exit criteria: The application's geometry, command structure, and major visual states match the reference screenshots within defined tolerances.

Status: IN_PROGRESS — the command layer and the window shell are started. New `crates/commands`
(`nc-commands`) is the single authoritative specification of commands, menus, shortcuts, and
availability: Win32 virtual-key mapping, availability rules evaluated against a supplied
context, the reference-matrix §3 registry with a per-row evidence grade and spec status, and a
resolved menu-bar model. The Windows shell (`crates/platform`) now builds its menu bar from that
registry and routes `WM_COMMAND` back through it; the headless backend resolves the same model,
so the command surface has exactly one definition and is testable off Windows. Verification is
green on Linux: fmt, clippy `-D warnings`, 46 tests, the crate-boundary guard, and
`cargo check --target x86_64-pc-windows-msvc`. Outstanding for M2: the client area (editor
viewport, scrollbars), status bar, context menus, standard open/save dialogs,
resize/maximize/minimize/restore, high-DPI scaling and theming, and focus/activation/keyboard
navigation — each needs the pinned reference machine to verify against screenshots
(`docs/reference-matrix.md` §2 and §5).
M3 — Text engine

    Unicode text model
    Caret movement and selection
    Mouse selection, double-click, and line selection
    Insert, delete, backspace, and overwrite behavior if applicable
    Multilevel undo and redo
    Clipboard copy, cut, paste, and select all
    Drag and drop
    Word wrap and horizontal scrolling
    Tabs and indentation behavior
    Long lines and empty lines
    CRLF, LF, and CR line-ending behavior
    Unicode, surrogate pairs, combining marks, emoji, and IME input
    Control characters and whitespace rendering where supported

Exit criteria: All editing commands pass differential tests against the reference application.

Status: IN_PROGRESS — the text engine is started as a new dependency-light crate, `crates/core`
(`nc-core`), which owns the document model and the editing primitives the command registry
names. It provides `TextBuffer` (UTF-8-only, `\n` terminators, byte-offset positions clamped to
character boundaries), `Selection`, `UndoStack` (edits store the removed text so undo restores
text and caret without recomputing), `CaretMovement` (character, word, line, and page steps with
a remembered desired column for vertical moves), `LineEnding` detection and reporting, `TabStop`
arithmetic, grapheme-aware soft wrap, a `Clipboard` trait with an in-memory implementation so the
engine never touches the platform clipboard, and the `Document` aggregate that ties them
together. Scope so far is the pure text model: caret movement, insertion and deletion, multilevel
undo/redo, copy/cut/paste/select-all, line-ending detection and round-tripping, word wrap, tabs,
Unicode/grapheme handling, mouse selection (single-, double-, and triple-click rules resolved in
the engine so the viewport only hit-tests), drag and drop of a selection within the document, and
overwrite mode. The suites are named after the feature matrix (`NC-E001`–`NC-E003`, `NC-E005`–
`NC-E009`) plus property tests over arbitrary input. Verification is green on Linux: fmt, clippy
`-D warnings`, 179 tests, the crate-boundary guard, and `cargo check --target
x86_64-pc-windows-msvc`. Outstanding for M3: horizontal scrolling for unwrapped long lines, IME
input, and rendering of control characters and whitespace — the input/interaction and rendering
items belong with the viewport in M2/M5, and differential testing against the reference is
blocked until the pinned Windows reference machine is available. Where the reference's exact
behaviour is not yet captured (`docs/reference-matrix.md` §4 and §5), the engine names its
behaviour an approximation rather than parity: default tab width 8, CRLF as the default style for
untitled documents, a 10-line page step, a greedy wrap rule that breaks at the last whitespace
that fits, a double-click taking one UAX #29 word segment and a triple-click the whole line
including its terminator (`NC-E005`), a drag moving in place with the copy modifier left to the
viewport (`NC-E009`), and an overwrite replacing one grapheme per grapheme typed without ever
eating a line terminator or splitting a character.

M4 — File I/O and data integrity

    Open and save files
    Save As and overwrite confirmation
    New document and reopen behavior
    Encoding detection and explicit encoding behavior
    UTF-8, UTF-16, BOM, and legacy encodings where applicable
    Preserve or correctly select line endings
    Detect external file changes if the reference does
    Handle permissions, locked files, and missing paths
    Handle empty, malformed, and very large files
    Dirty-state tracking
    Unsaved-changes prompts
    Recover gracefully from failed writes

Exit criteria: Opening and saving supported fixtures produces the same expected bytes and prompts as the reference. No silent data loss is permitted.

Status: IN_PROGRESS — file I/O is started as `crates/textproc` (`nc-textproc`), a
dependency-light crate that owns how a note's bytes become text and text becomes bytes; it
depends on `nc-core` and the standard library only, so the whole stage is testable headlessly.
It provides `Encoding` detection and transcoding (UTF-8 with and without a byte order mark,
UTF-16 and UTF-32 in either byte order, and ANSI — Windows-1252, the Western code page the
reference falls back to), `nc-core`'s `LineEnding` detected on read and round-tripped on write
(CRLF, LF and CR, with a mixed file reported and left alone rather than normalised), and
`TextFile`: the path, the bytes as they are on
disk, and the `Document` the text was read into. `open` reads and detects, `save` rewrites in
the encoding the document carries, `save_as` transcodes to a chosen encoding with or without a
mark, `save_lossy` is the explicit "write what you can, `?` the rest" path, and `reload`
re-reads the file. A save serialises the whole file to a sibling temporary file and renames it
into place only once it is complete, so a failed write leaves the original exactly as it was
and the document keeps its changes and its path (`NC-D006`). Unrepresentable text fails the
save and names the characters rather than substituting silently, and `external_change()` reports
the file being rewritten or removed under the document instead of assuming the in-memory copy is
current. `prompt` names the states in which the reference asks before destroying something —
unsaved changes, an existing file in Save As, characters the target encoding cannot hold, and a
file changed outside the editor — as values the shell can turn into a box; the wording and the
dialog itself stay with the viewport. The suites are named after the feature matrix
(`NC-F001`, `NC-F004`–`NC-F006`, `NC-D001`–`NC-D008`) plus property tests over arbitrary bytes
and lines. Verification is green on Linux: fmt, clippy `-D warnings`, 275 tests, the
crate-boundary guard, and `cargo check --target x86_64-pc-windows-msvc`. Outstanding for M4:
the Open/Save/Save As dialogs and the encoding list they show, the wording of the
unsaved-changes and overwrite prompts, dirty-state display, and differential byte comparison
against the reference (exit criterion), which is blocked until the pinned Windows reference
machine is available. Where the reference's exact behaviour is not yet captured
(`docs/reference-matrix.md` §4 and §5), the crate names its behaviour an approximation rather
than parity: ANSI is Windows-1252 (another locale's code page reads with the Western table,
which is wrong for those languages but never lossy), a file with no byte order mark and no
evidence to the contrary is UTF-8, and CRLF is the style a new document is written in.
M5 — Editing commands and search

    Find and find next/previous
    Replace and Replace All
    Search direction and wrap-around behavior
    Match-case and other supported search options
    Go To line, if supported
    Date/time insertion, if supported
    Select All and other Edit commands
    Keyboard shortcuts and menu accelerators
    Status-bar line and column calculations
    Search feedback, empty results, and boundary behavior

Exit criteria: Search and editing commands match reference behavior, including boundary cases and keyboard layouts.
M6 — Tabs and application lifecycle

    Multiple documents and tab switching
    New-tab and new-window commands
    Tab closing and close-all behavior
    Dirty-state indicators
    Save prompts during tab and application closure
    Session restoration
    Recent files and recent-document commands
    Duplicate opens and document identity
    Window and tab lifecycle edge cases
    Command-line file opening and activation of existing instances, if applicable

Exit criteria: Tab operations and application restarts preserve the expected documents and state without losing unsaved work.
M7 — Markdown functionality

Implement this milestone only to the extent supported by the selected reference build.

    Markdown file association and .md / .markdown handling
    Syntax view and formatted view
    Heading formatting
    Bold and italic
    Strikethrough
    Hyperlinks
    Ordered and unordered lists
    Nested lists and indentation
    Tables, where supported
    Formatting toolbar and context-menu commands
    Markdown source serialization
    Switching views without losing content or selection unexpectedly
    Undo/redo across formatting operations
    Malformed Markdown and unsupported syntax behavior

Exit criteria: Formatting operations, Markdown serialization, and view transitions pass a reference-derived fixture suite.

Markdown must not be silently normalized or rewritten in ways that differ from the reference application's behavior.
M8 — Settings and Windows integration

    Font family and size
    Word wrap and supported view preferences
    Theme and system-theme behavior
    Settings persistence
    Spell checking and autocorrect, if supported
    Accessibility and screen-reader support
    Keyboard access and focus order
    File associations and shell integration, if in scope
    Printing and page setup, if supported by the target
    Localization and keyboard-layout correctness

Exit criteria: Each supported setting persists correctly and produces the expected behavior across application restarts.
M9 — Optional and service-dependent functionality

Create separate compatibility specifications for features that require external services, accounts, entitlements, or device-specific capabilities.

    AI writing, rewriting, and summarization, if part of the target
    Optional onboarding or welcome experiences
    Preview-only features
    Service errors, authentication failures, and disabled-feature states

Do not substitute a different service and claim 1:1 parity. Reproduce the observable experience only where legally and technically feasible. Keep service-dependent functionality isolated from the core editor.
M10 — Performance and reliability

    Measure startup time
    Measure memory usage and idle CPU usage
    Benchmark large files and extremely long lines
    Benchmark search and Replace All
    Benchmark scrolling, typing, and selection
    Prevent unnecessary full-document copies
    Profile allocations and rendering
    Add stress tests for repeated tab and file operations
    Test invalid encodings, inaccessible paths, and interrupted saves
    Add crash diagnostics and regression tests

Exit criteria: Performance is within predefined tolerances of the reference on identical hardware and datasets, without correctness regressions.
M11 — Differential parity validation

    Run the same test fixtures through both applications.
    Compare saved output byte-for-byte.
    Compare keyboard-command outcomes and selection state.
    Compare screenshots at identical resolutions and scaling.
    Compare dialogs, prompts, and state transitions.
    Test Unicode and input methods.
    Test all supported file encodings and line endings.
    Run randomized editing sequences.
    Verify fixes against the complete regression suite.
    Publish a feature-by-feature parity report.

Exit criteria: Every mandatory feature passes. Remaining differences are documented, reproducible, and explicitly excluded from the parity claim.
M12 — Release engineering

    Produce reproducible release builds.
    Test installation, upgrade, and uninstall.
    Sign binaries and installers where appropriate.
    Validate Windows Defender and application reputation behavior.
    Document supported Windows versions and architectures.
    Prepare crash reporting and privacy documentation.
    Create release notes and known-limitations documentation.

Exit criteria: A clean Windows machine can install, launch, use, upgrade, and remove the application reliably.
5. Repository Layout

rust-notepad/
├── AGENTS.md
├── ROADMAP.md
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── app/
│   ├── ui/
│   ├── editor/
│   ├── document/
│   ├── encoding/
│   ├── filesystem/
│   ├── search/
│   ├── markdown/
│   ├── session/
│   └── platform/
├── tests/
│   ├── fixtures/
│   ├── editing/
│   ├── file_io/
│   ├── keyboard/
│   ├── markdown/
│   └── visual/
└── docs/
    ├── reference-matrix.md
    ├── feature-matrix.md
    ├── architecture.md
    ├── test-strategy.md
    └── parity-report.md

Adjust crate boundaries when evidence supports a simpler design. Avoid creating empty crates solely to match this template.
6. Definition of Done

A feature is complete only when:

    Its reference behavior is documented.
    Its implementation is integrated into the actual application.
    Its keyboard, mouse, menu, and dialog entry points work as applicable.
    Its edge cases are tested.
    Its file output and state transitions match expectations.
    It passes formatting, linting, and automated tests.
    Its regression tests are committed.
    Its parity status is updated in docs/feature-matrix.md.

7. Parity Tracking

Every feature must have one of these statuses:

    NOT_STARTED
    SPECIFIED
    IN_PROGRESS
    IMPLEMENTED
    VERIFIED
    BLOCKED
    OUT_OF_SCOPE

IMPLEMENTED does not mean parity is achieved. Only VERIFIED means the behavior has been tested against the reference.

Track each feature with a stable ID, reference version, acceptance criteria, test location, implementation status, and known differences.
8. Core Engineering Principles

    Correctness before cosmetic polish.
    Observable behavior before assumptions about internals.
    Byte preservation before convenience transformations.
    Native Windows integration where it materially improves fidelity.
    Explicit error handling rather than silent recovery.
    Automated regression tests for every fixed mismatch.
    No unverified feature-parity claims.
    No unnecessary dependencies or architecture complexity.
    No feature removal merely because the reference behavior is inconvenient.

The final objective is a maintainable Rust application whose behavior is demonstrably equivalent to the pinned Windows Notepad reference, within the explicitly documented scope.
