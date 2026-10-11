# Reference Matrix — Windows 11 Notepad Parity Target

Source of truth for the **target observable behavior** of the reference application.
Every claim below carries an evidence tag. See [§0 Evidence conventions](#0-evidence-conventions).

This file is a *living specification*. Items tagged `TO_CAPTURE` are `BLOCKED` until they
are observed on the pinned reference machine and copied here with a fixture or screenshot.

---

## 0. Evidence conventions

| Tag | Meaning |
| --- | --- |
| `DOCUMENTED` | Taken from Microsoft documentation / release notes / the reference app's own published change log. Reliable, but not yet reproduced locally. |
| `CAPTURED` | Observed and reproduced on the pinned reference machine (Windows 11 + pinned Notepad build). Backed by a fixture in `tests/fixtures/` and/or a screenshot. |
| `TO_CAPTURE` | Not yet verified. Behavior is unknown or only partially known. **Must not be implemented from a guess.** |
| `BLOCKED` | Cannot be captured without the reference machine, a licensed service, or a device capability we do not have. |

Rule (AGENT.md §2): do not invent reference behavior and do not mark a feature `VERIFIED`
without a reference-derived test. `DOCUMENTED` is a starting spec, **not** parity.

---

## 1. Reference baseline

| Field | Value | Evidence |
| --- | --- | --- |
| Operating system | Windows 11 (25H2 or later) | DOCUMENTED |
| Reference application | Microsoft Notepad (Store/UWP app) | DOCUMENTED |
| Reference app version | **`11.2607.14`** (July 2026 Update, 2026-09-02) — current stable channel | DOCUMENTED |
| Insider build of record | `11.2606.15.0` (rolled out 2026-07-24) | DOCUMENTED |
| Windows build | TO_CAPTURE (`winver` on the pinned reference machine) | TO_CAPTURE |
| Display scaling matrix | 100%, 150%, 200% | TO_CAPTURE |
| Architectures | x64 (primary); ARM64 (secondary) | DOCUMENTED |
| Languages / keyboard layouts | en-US (primary); es-ES & pt-BR (Ctrl+A conflict regression); others later | DOCUMENTED |
| Distribution | Microsoft Store app id `9MSMLRH6LZF3` | DOCUMENTED |

**PINNING ACTION (must complete before M8+):** on the reference machine, record
`Get-AppxPackage *Microsoft.WindowsNotepad*` version + `winver` build, and freeze
Windows Update for the Notepad package. Until then the baseline is *documented*, not *pinned*.

### 1.1 Version history of record (feature-bearing releases)

| Date | Version | Change | Evidence |
| --- | --- | --- | --- |
| 2022-02-16 | — | Windows 11 redesign; dark mode; Find/Replace flyout | DOCUMENTED |
| 2023-01-19 | 11.2212.33.0 | Tabs / multi-document window (Insider) | DOCUMENTED |
| 2024-03..04 | 11.2402.22.0 | Spell check + autocorrect | DOCUMENTED |
| 2024-11-06 | 11.2410.17.0 | Rewrite (AI, tone/length/format) — preview | DOCUMENTED |
| 2025-03-13 | 11.2501.30.0 | Summarize; Recent files | DOCUMENTED |
| 2025-05-22 | 11.2504.46.0 | Write (AI generation) — preview | DOCUMENTED |
| 2025-05..06 | — | Lightweight formatting (bold, italic, links, lists, headings) | DOCUMENTED |
| 2025-10-16 | — | 25H2 positioning; local AI on Copilot+ PCs | DOCUMENTED |
| 2025-11-21 | 11.2510.6.0 | Tables; streaming AI results — preview | DOCUMENTED |
| 2026-01-21 | 11.2512.10.0 | Strikethrough; nested lists; wider streaming | DOCUMENTED |
| 2026-06-24 | 11.2605.29.0 | Add-tab shows Ctrl+N; Find/Replace wrap-around notices; Ctrl+A conflict fix (es/pt) | DOCUMENTED |
| 2026-07-24 | 11.2606.15.0 | Formatting fixes for `.md`/`.markdown`; nested-list copy/paste; concurrent print; launch perf | DOCUMENTED |
| 2026-09-02 | 11.2607.14 | Current stable (July 2026 Update) | DOCUMENTED |

> Note: release notes for the app are published on Microsoft Learn (Windows Insider →
> Notepad) and are the authoritative changelog. Re-read before each parity pass.

---

## 2. Window, chrome and menus

Modern Notepad is a single window with a **tab strip**, a **toolbar** (formatting buttons
centre-top, AI + Settings at the right), an **editor viewport**, and a **status bar**
(line/column, encoding, zoom). Evidence: DOCUMENTED (Thurrott field guide, app screenshots).

| Region | Content | Evidence |
| --- | --- | --- |
| Title bar | `{filename} - Notepad` (dirty indicator via `*`?) — exact dirty marker TO_CAPTURE | TO_CAPTURE |
| Tab strip | One tab per document; `+` add-tab button (hover shows `Ctrl+N`) | DOCUMENTED |
| Toolbar (centre) | Markdown formatting buttons (B, I, S, headings, lists, table…) shown only when Formatting enabled | DOCUMENTED |
| Toolbar (right) | AI/Copilot entry, Settings (gear) | DOCUMENTED |
| Status bar | Line:Column, encoding, zoom %, word-wrap indicator — exact fields TO_CAPTURE | TO_CAPTURE |
| Menu bar | Classic File/Edit/View/Help structure retained; exact submenus TO_CAPTURE | TO_CAPTURE |
| Context menu | Editor right-click menu; spelling submenu on misspelled word; formatting submenu in Markdown mode | DOCUMENTED |

### 2.1 Capture procedure (menus)
For each menu and submenu: screenshot at 100/150/200% scaling, record exact item labels,
accelerators, enabled/disabled states for {empty doc, dirty doc, no tab, read-only, .md doc}.
Store under `tests/fixtures/visual/menus/`.

---

## 3. Command registry

Single authoritative registry (`platform`/`app` command table). Every keyboard shortcut and
menu item resolves to the same command id. Status: **`SPEC`** (specified) vs **`VER`** (verified).

Availability legend: `always`, `edited` (a document/tab exists), `dirty`, `selection`,
`md` (Markdown formatting enabled), `service` (requires signed-in MS account / Copilot+ PC).

### 3.1 File commands

| ID | Command | Menu | Shortcut | Availability | Expected behavior | Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `cmd.file.new` | New | File ▸ New | Ctrl+N | always | New empty document in a new tab (per June-2026 note, `+` tab button labelled Ctrl+N). Default encoding UTF-8 (no BOM), default EOL CRLF | DOCUMENTED / partially TO_CAPTURE | SPEC |
| `cmd.file.new_tab` | New tab | (tab `+`) | Ctrl+T | always | New tab in current window | DOCUMENTED | SPEC |
| `cmd.file.new_window` | New window | File ▸ New window | Ctrl+Shift+N | always | New top-level window | DOCUMENTED | SPEC |
| `cmd.file.open` | Open | File ▸ Open | Ctrl+O | always | Standard Windows Open dialog; opens into new tab | DOCUMENTED | SPEC |
| `cmd.file.save` | Save | File ▸ Save | Ctrl+S | edited | Overwrites in place with original encoding/EOL; no dialog for an already-named doc | DOCUMENTED | SPEC |
| `cmd.file.save_as` | Save As | File ▸ Save As | Ctrl+Shift+S | edited | Save-As dialog with Encoding dropdown | DOCUMENTED | SPEC |
| `cmd.file.print` | Print | File ▸ Print | Ctrl+P | edited | Print dialog; concurrent print jobs across windows/tabs supported | DOCUMENTED | SPEC |
| `cmd.file.page_setup` | Page setup | File ▸ Page setup | — | edited | Page setup dialog | DOCUMENTED | SPEC |
| `cmd.file.close_tab` | Close tab | File ▸ Close tab | Ctrl+W | edited | Close active tab; prompts if dirty; closes window if last tab | DOCUMENTED | SPEC |
| `cmd.file.exit` | Exit | File ▸ Exit | Alt+F4 | always | Close window; unsaved prompt per tab | DOCUMENTED | SPEC |
| `cmd.file.recent` | Recent files | File ▸ Recent | — | always | List of recent documents | DOCUMENTED | SPEC |
| `cmd.file.session_restore` | (implicit) | — | — | startup | Restore prior tabs/documents in same saved/unsaved state | DOCUMENTED | SPEC |

### 3.2 Edit commands

| ID | Command | Menu | Shortcut | Availability | Expected behavior | Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `cmd.edit.undo` | Undo | Edit ▸ Undo | Ctrl+Z | edited | Multilevel undo | DOCUMENTED | SPEC |
| `cmd.edit.redo` | Redo | Edit ▸ Redo | Ctrl+Y | edited | Redo undone edit | DOCUMENTED | SPEC |
| `cmd.edit.cut` | Cut | Edit ▸ Cut | Ctrl+X | selection | Copy to clipboard + delete | DOCUMENTED | SPEC |
| `cmd.edit.copy` | Copy | Edit ▸ Copy | Ctrl+C | selection | Copy to clipboard | DOCUMENTED | SPEC |
| `cmd.edit.paste` | Paste | Edit ▸ Paste | Ctrl+V | edited | Insert clipboard text at caret | DOCUMENTED | SPEC |
| `cmd.edit.delete` | Delete | Edit ▸ Delete | Del | selection | Delete selection | DOCUMENTED | SPEC |
| `cmd.edit.select_all` | Select all | Edit ▸ Select all | Ctrl+A | edited | Select whole document | DOCUMENTED | SPEC |
| `cmd.edit.find` | Find | Edit ▸ Find | Ctrl+F | edited | Open Find flyout/replace bar | DOCUMENTED | SPEC |
| `cmd.edit.find_next` | Find next | Edit ▸ Find next | F3 | edited | Next match | DOCUMENTED | SPEC |
| `cmd.edit.find_prev` | Find previous | Edit ▸ Find previous | Shift+F3 | edited | Previous match | DOCUMENTED | SPEC |
| `cmd.edit.replace` | Replace | Edit ▸ Replace | Ctrl+H | edited | Replace UI; Replace / Replace All | DOCUMENTED | SPEC |
| `cmd.edit.go_to` | Go to | Edit ▸ Go to | Ctrl+G | edited | Go-to-line dialog | DOCUMENTED (classic; re-confirm modern) | SPEC |
| `cmd.edit.time_date` | Time/Date | Edit ▸ Time/Date | F5 | edited | Insert current date/time at caret | TO_CAPTURE | SPEC |
| `cmd.edit.word_wrap` | Word wrap | View/Edit | — | edited | Toggle soft wrap | DOCUMENTED | SPEC |

### 3.3 View / zoom commands

| ID | Command | Menu | Shortcut | Availability | Expected behavior | Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `cmd.view.zoom_in` | Zoom in | View ▸ Zoom | Ctrl+Plus (+/=) | edited | Increase zoom | DOCUMENTED | SPEC |
| `cmd.view.zoom_out` | Zoom out | View ▸ Zoom | Ctrl+Minus | edited | Decrease zoom | DOCUMENTED | SPEC |
| `cmd.view.zoom_reset` | Reset zoom | View ▸ Zoom | Ctrl+0 | edited | 100% | DOCUMENTED | SPEC |
| `cmd.view.status_bar` | Status bar | View ▸ Status bar | — | always | Toggle status bar | DOCUMENTED | SPEC |

### 3.4 Tabs / navigation

| ID | Command | Menu | Shortcut | Availability | Expected behavior | Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `cmd.tab.next` | Next tab | — | Ctrl+Tab | ≥2 tabs | Cyclic forward | DOCUMENTED | SPEC |
| `cmd.tab.prev` | Previous tab | — | Ctrl+Shift+Tab | ≥2 tabs | Cyclic backward | DOCUMENTED | SPEC |
| `cmd.tab.close_all` | Close all | File ▸ Close all | — | edited | Close every tab, prompting per dirty doc | DOCUMENTED | SPEC |

### 3.5 Markdown / formatting commands (availability `md`)

| ID | Command | Menu | Shortcut | Expected behavior | Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `cmd.md.bold` | Bold | Format/toolbar | Ctrl+B | Wrap/apply `**` to selection | DOCUMENTED | SPEC |
| `cmd.md.italic` | Italic | Format/toolbar | Ctrl+I | Apply `*` | DOCUMENTED | SPEC |
| `cmd.md.strike` | Strikethrough | Format/toolbar | — | Apply `~~` (added Jan 2026) | DOCUMENTED | SPEC |
| `cmd.md.h1`..`h6` | Headings | Format/toolbar | — | Apply ATX heading level | DOCUMENTED | SPEC |
| `cmd.md.bullets` | Bulleted list | Format/toolbar | — | `-` list; nested supported | DOCUMENTED | SPEC |
| `cmd.md.numbering` | Numbered list | Format/toolbar | — | `1.` list | DOCUMENTED | SPEC |
| `cmd.md.link` | Hyperlink | Format/toolbar | — | Insert Markdown link | DOCUMENTED | SPEC |
| `cmd.md.table` | Table | Format/toolbar | — | Insert/expand table (Nov 2025) | DOCUMENTED | SPEC |
| `cmd.md.clear` | Clear formatting | Format/toolbar | — | Strip inline formatting | TO_CAPTURE | SPEC |
| `cmd.md.toggle_source` | Source/formatted view | View ▸ Markdown | — | Switch between raw syntax and rendered view without data loss | TO_CAPTURE | SPEC |

### 3.6 AI / service-dependent commands (`service`, M9)

| ID | Command | Shortcut | Availability | Evidence | Status |
| --- | --- | --- | --- | --- | --- |
| `cmd.ai.rewrite` | Rewrite | Ctrl+? TO_CAPTURE | selection + service | DOCUMENTED | SPEC / BLOCKED |
| `cmd.ai.summarize` | Summarize | TO_CAPTURE | selection + service | DOCUMENTED | SPEC / BLOCKED |
| `cmd.ai.write` | Write | TO_CAPTURE | service | DOCUMENTED | SPEC / BLOCKED |
| `cmd.ai.tone` / `length` / `format` | Adjust | — | service | DOCUMENTED | SPEC / BLOCKED |

---

## 4. Encoding and line endings

| Aspect | Reference behavior | Evidence |
| --- | --- | --- |
| Default for new doc | UTF-8 **without** BOM | DOCUMENTED |
| Encodings read | ANSI (locale code page), UTF-8 (with & without BOM), UTF-16 LE/BE | DOCUMENTED |
| Save As encoding list | to record exact order: e.g. ANSI, UTF-8, UTF-8 with BOM, UTF-16 LE, UTF-16 BE, UCS-2 LE, UTF-32 LE — **exact list TO_CAPTURE** | TO_CAPTURE |
| BOM handling | Since Windows 10 BOM is optional for UTF-8; UTF-16 carries BOM | DOCUMENTED |
| Line endings | CRLF (default new files); LF and CR read since Win10 1809; **preserve on save** | DOCUMENTED |
| No silent normalization | Encoding and EOL of an opened file are preserved on save unless changed via Save As | DOCUMENTED |
| Status bar | Shows current encoding (e.g. "UTF-8" / "ANSI") | DOCUMENTED |
| External modification | TO_CAPTURE whether detected/relaunched | TO_CAPTURE |

**Byte-preservation fixtures required:** see `docs/test-strategy.md` §Fixtures.

---

## 5. Edge cases to capture on the reference machine

- [ ] Empty file open/save (bytes written)
- [ ] File with BOM vs without, each of UTF-8/16/32
- [ ] Mixed CRLF/LF/CR in one file — what is written back
- [ ] Very large file (≥ 100 MB) — load time, memory, wrap-off behavior
- [ ] Extremely long single line (≥ 1 MB)
- [ ] Unicode: astral plane, combining marks, emoji ZWJ sequences, RTL
- [ ] Surrogate handling on cut/paste across the boundary
- [ ] IME composition (e.g. Japanese) caret behavior
- [ ] Tabs vs spaces, tab stop width
- [ ] Read-only file Save → error dialog text
- [ ] Save into deleted/permission-denied directory
- [ ] Full disk / failed write — original preserved?
- [ ] Ctrl+A on es-ES / pt-BR keyboard layouts (2026 regression)
- [ ] Dirty prompt wording for Close vs Exit vs Restart
- [ ] Tab drag-reorder and detach-to-window (if supported)

Each captured case becomes a fixture + acceptance test and its `TO_CAPTURE` tag flips to
`CAPTURED` here.

---

## 6. Known version-specific differences to watch

| Area | Difference | Source |
| --- | --- | --- |
| `Ctrl+A` | Was intermittently broken on es/pt layouts; fixed 2026-06 | DOCUMENTED |
| Underscore rendering | Fixed 2026-07 | DOCUMENTED |
| `.md`/`.markdown` formatting | Fixed 2026-07 | DOCUMENTED |
| Find/Replace wrap | Wrap-around disabled → "Beginning/end of document reached" inline notice (2026-06) | DOCUMENTED |

---

## 7. Capture to-do log

| # | Item | Blocked by | Owner |
| --- | --- | --- | --- |
| 1 | Pin exact app version + Windows build on reference machine | Windows reference machine | TBD |
| 2 | Full menu/submenu label + accelerator screenshots | Windows reference machine | TBD |
| 3 | Save As encoding dropdown exact list & order | Windows reference machine | TBD |
| 4 | Status bar exact field set | Windows reference machine | TBD |
| 5 | File-menu recent-files entry format | Windows reference machine | TBD |
| 6 | Edge-case fixture set (§5) | Windows reference machine | TBD |
| 7 | AI feature availability/entitlement matrix | MS account / Copilot+ PC | TBD |
