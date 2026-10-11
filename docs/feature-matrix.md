# Feature Matrix — Implementation & Verification Status

Source of truth for **implementation and verification status** (AGENT.md §11 statuses).
Behavior specification lives in [`reference-matrix.md`](reference-matrix.md).

**Status values:** `NOT_STARTED` · `SPECIFIED` · `IN_PROGRESS` · `IMPLEMENTED` · `VERIFIED` · `BLOCKED` · `OUT_OF_SCOPE`

`IMPLEMENTED` ≠ parity. Only `VERIFIED` means tested against the reference.
`Ref` = reference evidence: `DOC` (documented) · `CAP` (captured on reference machine) · `TC` (to capture).

**Current phase: M0 — Specification & reference capture. No code exists yet; every
implementation status is `NOT_STARTED`.** Spec status is the column that is meaningful now.

---

## A. File & document lifecycle

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| NC-F001 | New empty document (Ctrl+N) | MANDATORY | M4 | SPECIFIED | DOC/TC | NOT_STARTED | Opens UTF-8 (no BOM), CRLF, empty, in new tab | `tests/editing/new_document.rs` |
| NC-F002 | New tab (Ctrl+T) | MANDATORY | M6 | SPECIFIED | DOC | NOT_STARTED | Adds tab without disturbing others | `tests/session/tabs.rs` |
| NC-F003 | New window (Ctrl+Shift+N) | MANDATORY | M6 | SPECIFIED | DOC | NOT_STARTED | Independent top-level window | `tests/session/windows.rs` |
| NC-F004 | Open file (Ctrl+O) | MANDATORY | M4 | SPECIFIED | DOC | NOT_STARTED | Native Open dialog; new tab; encoding/EOL detected | `tests/file_io/open.rs` |
| NC-F005 | Save (Ctrl+S) | MANDATORY | M4 | SPECIFIED | DOC | NOT_STARTED | Byte-correct in-place save; no data loss | `tests/file_io/save.rs` |
| NC-F006 | Save As + Encoding dropdown (Ctrl+Shift+S) | MANDATORY | M4 | SPECIFIED | DOC/TC | NOT_STARTED | Dialog matches reference; chosen encoding applied | `tests/file_io/save_as.rs` |
| NC-F007 | Print / Page setup (Ctrl+P) | OPTIONAL | M8 | SPECIFIED | DOC | NOT_STARTED | Native print pipeline; concurrent jobs OK | `tests/file_io/print.rs` |
| NC-F008 | Close tab (Ctrl+W) | MANDATORY | M6 | SPECIFIED | DOC | NOT_STARTED | Dirty prompt; window closes on last tab | `tests/session/tabs.rs` |
| NC-F009 | Exit (Alt+F4) | MANDATORY | M6 | SPECIFIED | DOC | NOT_STARTED | Per-tab unsaved prompts | `tests/session/lifecycle.rs` |
| NC-F010 | Recent files | OPTIONAL | M6 | SPECIFIED | DOC | NOT_STARTED | File ▸ Recent lists prior docs | `tests/session/recent.rs` |
| NC-F011 | Session restore | MANDATORY | M6 | SPECIFIED | DOC/TC | NOT_STARTED | Restores tabs incl. unsaved in same state | `tests/session/restore.rs` |
| NC-F012 | Duplicate-open / document identity | MANDATORY | M6 | SPECIFIED | TC | NOT_STARTED | Reference's choice (new tab vs focus existing) | `tests/session/identity.rs` |

## B. Editing, selection, undo

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| NC-E001 | Undo (Ctrl+Z) / Redo (Ctrl+Y) multilevel | MANDATORY | M3 | SPECIFIED | DOC/TC | NOT_STARTED | Undo depth & grouping match reference | `tests/editing/undo.rs` |
| NC-E002 | Cut / Copy / Paste (Ctrl+X/C/V) | MANDATORY | M3 | SPECIFIED | DOC | NOT_STARTED | Clipboard text + EOL conversion reference-exact | `tests/editing/clipboard.rs` |
| NC-E003 | Select all (Ctrl+A) | MANDATORY | M3 | SPECIFIED | DOC | NOT_STARTED | Whole-document selection; es/pt layout safe | `tests/keyboard/select_all.rs` |
| NC-E004 | Caret navigation (arrows, Home/End, Ctrl+arrow, PgUp/PgDn) | MANDATORY | M3 | SPECIFIED | DOC | NOT_STARTED | Word/line/paragraph boundaries reference-exact | `tests/editing/caret.rs` |
| NC-E005 | Mouse selection (click, drag, double, triple) | MANDATORY | M3 | SPECIFIED | DOC/TC | NOT_STARTED | Word/line selection matches reference | `tests/editing/mouse.rs` |
| NC-E006 | Word wrap toggle | MANDATORY | M3 | SPECIFIED | DOC | NOT_STARTED | Soft wrap; no bytes changed | `tests/editing/word_wrap.rs` |
| NC-E007 | Tabs & indentation behavior | MANDATORY | M3 | SPECIFIED | TC | NOT_STARTED | Tab stop width, tab insertion vs focus | `tests/editing/tabs.rs` |
| NC-E008 | Unicode: astral, combining, emoji, IME, surrogate safety | MANDATORY | M3 | SPECIFIED | DOC | NOT_STARTED | No broken surrogate pairs on edit | `tests/editing/unicode.rs` |
| NC-E009 | Drag & drop text | OPTIONAL | M3 | SPECIFIED | TC | NOT_STARTED | Internal + cross-app drag | `tests/editing/dnd.rs` |
| NC-E010 | Status bar line/column calculation | MANDATORY | M5 | SPECIFIED | DOC/TC | NOT_STARTED | 1-based line/col; encoding; matches reference | `tests/keyboard/status_bar.rs` |

## C. Find / replace / navigation

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| NC-S001 | Find (Ctrl+F) | MANDATORY | M5 | SPECIFIED | DOC | NOT_STARTED | Flyout/bar matches reference UI | `tests/editing/find.rs` |
| NC-S002 | Find next / previous (F3 / Shift+F3) | MANDATORY | M5 | SPECIFIED | DOC | NOT_STARTED | Direction + wrap semantics | `tests/editing/find.rs` |
| NC-S003 | Replace / Replace All (Ctrl+H) | MANDATORY | M5 | SPECIFIED | DOC | NOT_STARTED | Count + undo as single action | `tests/editing/replace.rs` |
| NC-S004 | Search options (match case, whole word, direction) | MANDATORY | M5 | SPECIFIED | TC | NOT_STARTED | Reference-exact option set | `tests/editing/find_options.rs` |
| NC-S005 | Wrap-around + boundary notices | MANDATORY | M5 | SPECIFIED | DOC | NOT_STARTED | Inline "Beginning/end of document reached" notice | `tests/editing/find_boundary.rs` |
| NC-S006 | Go to line (Ctrl+G) | OPTIONAL | M5 | SPECIFIED | DOC/TC | NOT_STARTED | Dialog + clamp behavior | `tests/editing/go_to.rs` |
| NC-S007 | Insert date/time (F5) | OPTIONAL | M5 | SPECIFIED | TC | NOT_STARTED | Format string reference-exact | `tests/editing/time_date.rs` |

## D. Encoding & file I/O data integrity

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| NC-D001 | UTF-8 with & without BOM read/write | MANDATORY | M4 | SPECIFIED | DOC | NOT_STARTED | Byte-preserving round-trip | `tests/file_io/encoding_utf8.rs` |
| NC-D002 | UTF-16 LE/BE (+ UTF-32) read/write | MANDATORY | M4 | SPECIFIED | DOC | NOT_STARTED | BOM-correct round-trip | `tests/file_io/encoding_utf16.rs` |
| NC-D003 | ANSI (locale code page) read/write | MANDATORY | M4 | SPECIFIED | DOC | NOT_STARTED | Unrepresentable chars: reference behavior | `tests/file_io/encoding_ansi.rs` |
| NC-D004 | EOL detect/preserve CRLF/LF/CR (incl. mixed) | MANDATORY | M4 | SPECIFIED | DOC/TC | NOT_STARTED | Bytes on save match reference | `tests/file_io/eol.rs` |
| NC-D005 | Dirty-state tracking + unsaved prompts | MANDATORY | M4 | SPECIFIED | DOC | NOT_STARTED | Prompt wording/threshold match | `tests/file_io/dirty.rs` |
| NC-D006 | Failed-write / read-only / permission errors | MANDATORY | M4 | SPECIFIED | TC | NOT_STARTED | Original file preserved; reference error text | `tests/file_io/errors.rs` |
| NC-D007 | Empty / malformed / very large files | MANDATORY | M4/M10 | SPECIFIED | TC | NOT_STARTED | No crash; reference limits | `tests/file_io/stress.rs` |
| NC-D008 | External file-change detection | OPTIONAL | M4 | SPECIFIED | TC | NOT_STARTED | Match reference (detect / prompt / ignore) | `tests/file_io/external_change.rs` |

## E. Tabs, session, lifecycle

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| NC-T001 | Tab strip + add-tab button | MANDATORY | M6 | SPECIFIED | DOC | NOT_STARTED | Geometry matches reference within tolerance | `tests/session/tabs.rs` |
| NC-T002 | Tab switching Ctrl+Tab / Ctrl+Shift+Tab | MANDATORY | M6 | SPECIFIED | DOC | NOT_STARTED | Cyclic; works with N tabs | `tests/keyboard/tab_switch.rs` |
| NC-T003 | Tab context menu (close, close others, …) | OPTIONAL | M6 | SPECIFIED | TC | NOT_STARTED | Reference item set | `tests/session/tab_menu.rs` |
| NC-T004 | Dirty indicator on tab/title | MANDATORY | M6 | SPECIFIED | TC | NOT_STARTED | Marker matches reference | `tests/session/dirty_indicator.rs` |
| NC-T005 | Close all | MANDATORY | M6 | SPECIFIED | DOC | NOT_STARTED | Prompt per dirty doc | `tests/session/close_all.rs` |

## F. Markdown / formatting (M7)

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| NC-M001 | `.md`/`.markdown` association & handling | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | Correct formatting path per extension | `tests/markdown/assoc.rs` |
| NC-M002 | Syntax view vs formatted view toggle | MANDATORY | M7 | SPECIFIED | DOC/TC | NOT_STARTED | No content/selection loss on switch | `tests/markdown/view_switch.rs` |
| NC-M003 | Bold / Italic (Ctrl+B / Ctrl+I) | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | Serialization reference-exact | `tests/markdown/inline.rs` |
| NC-M004 | Strikethrough | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | `~~` serialization | `tests/markdown/inline.rs` |
| NC-M005 | Headings (H1–H6) | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | ATX output matches | `tests/markdown/headings.rs` |
| NC-M006 | Ordered/unordered lists + nesting | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | Nested indent + copy/paste correct | `tests/markdown/lists.rs` |
| NC-M007 | Hyperlinks | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | Link syntax reference-exact | `tests/markdown/links.rs` |
| NC-M008 | Tables | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | Insert/expand semantics | `tests/markdown/tables.rs` |
| NC-M009 | Clear formatting | OPTIONAL | M7 | SPECIFIED | TC | NOT_STARTED | Reference result | `tests/markdown/clear.rs` |
| NC-M010 | Formatting participates in undo/redo | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | Single-step undo per op | `tests/markdown/undo.rs` |
| NC-M011 | Malformed / unsupported syntax preserved | MANDATORY | M7 | SPECIFIED | DOC | NOT_STARTED | No silent normalization | `tests/markdown/malformed.rs` |

## G. Settings & Windows integration (M8)

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Acceptance criteria | Test |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| NC-P001 | App theme (system/light/dark) | MANDATORY | M8 | SPECIFIED | DOC | NOT_STARTED | Matches OS + manual override | `tests/visual/theme.rs` |
| NC-P002 | Font family & size | MANDATORY | M8 | SPECIFIED | DOC | NOT_STARTED | Persists; rendering matches | `tests/visual/font.rs` |
| NC-P003 | Word-wrap preference persistence | MANDATORY | M8 | SPECIFIED | DOC | NOT_STARTED | Survives restart | `tests/session/settings.rs` |
| NC-P004 | Spell check + autocorrect | MANDATORY | M8 | SPECIFIED | DOC | NOT_STARTED | Squiggle, suggestions, dictionary | `tests/settings/spell.rs` |
| NC-P005 | Formatting (Markdown) enable/disable toggle | MANDATORY | M8 | SPECIFIED | DOC | NOT_STARTED | Toolbar hides when off | `tests/settings/formatting_toggle.rs` |
| NC-P006 | High-DPI 100/150/200% scaling | MANDATORY | M2/M8 | SPECIFIED | TC | NOT_STARTED | Layout within tolerance | `tests/visual/dpi.rs` |
| NC-P007 | Accessibility / UIA / screen reader | MANDATORY | M8 | SPECIFIED | TC | NOT_STARTED | Names, focus order correct | `tests/visual/a11y.rs` |
| NC-P008 | File association & shell integration | OPTIONAL | M8 | SPECIFIED | DOC | NOT_STARTED | Open-with, default app | `tests/file_io/assoc.rs` |
| NC-P009 | Command-line file args + instance activation | MANDATORY | M1/M6 | SPECIFIED | TC | NOT_STARTED | Opens file; reuses/activates instance per reference | `tests/file_io/cli.rs` |
| NC-P010 | Localization / keyboard-layout correctness | OPTIONAL | M8 | SPECIFIED | DOC | NOT_STARTED | es/pt shortcuts correct | `tests/keyboard/layouts.rs` |

## H. AI / service-dependent (M9) — `BLOCKED` pending entitlement access

| ID | Feature | Priority | Milestone | Spec | Ref | Impl | Note |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-X001 | Rewrite (tone/length/format) | SERVICE | M9 | SPECIFIED | DOC | NOT_STARTED | Requires MS account / Copilot+ PC |
| NC-X002 | Summarize | SERVICE | M9 | SPECIFIED | DOC | NOT_STARTED | Requires service |
| NC-X003 | Write (generate from instruction) | SERVICE | M9 | SPECIFIED | DOC | NOT_STARTED | Requires service |
| NC-X004 | Streaming results | SERVICE | M9 | SPECIFIED | DOC | NOT_STARTED | Progressive display |
| NC-X005 | Service error / auth-failure / disabled states | SERVICE | M9 | SPECIFIED | TC | NOT_STARTED | Observable experience reproduced where feasible |

---

## Verification summary (M0)

| Metric | Value |
| --- | --- |
| Total tracked features | 60 |
| SPECIFIED | 60 |
| Reference-captured (`CAP`) | **0** |
| IMPLEMENTED | 0 |
| VERIFIED | 0 |
| BLOCKED (entitlement) | 5 (NC-X001..X005) |

> Nothing is `VERIFIED`; no code exists. M0 is **not** complete until reference-capture
> (`TC`→`CAP`) is performed on the pinned reference machine (see `reference-matrix.md` §7).
