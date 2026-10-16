# UI Reference — NoteCraft

Source of truth for the UI NoteCraft builds. It is captured from the supplied
screens of the modern Notepad app (dark theme): the main window, the File / Edit /
View menus, the paragraph-style and list dropdowns, and the Settings page.

Unlike a byte-level software spec, this is a **design reference**: it fixes the
layout, the menu items, the toolbar controls, the status-bar fields, and the
dialog surfaces. Anything the screens do not show is a design decision recorded
here or in the command registry (`crates/commands`), not a guess: the find,
go-to, open / save-as, page-setup and prompt dialogs (§5–§9) are specified from
the behaviour the headless engines already fix, and are marked as decisions
where the reference capture does not reach.

---

## 1. Main window

```
┌───────────────────────────────────────────────────────────────────────────┐
│ ▤  ‹  [ Untitled × ][ Untitled × ][ Untitled ]  ▷  +        ─  ▢  ✕        │  title bar / tab strip
├───────────────────────────────────────────────────────────────────────────┤
│ File  Edit  View │ H1▾  ≡▾  B  I  S  🔗  ▦  Aa        ✨▾  💬  ⬤  ⚙        │  top bar
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│                            ( editor surface )                             │
│                                                                           │
├───────────────────────────────────────────────────────────────────────────┤
│ Ln 1, Col 1 │ 0 characters        Plain text        100% │ Windows (CRLF) │ UTF-8 │  status bar
└───────────────────────────────────────────────────────────────────────────┘
```

| Region | Content |
| --- | --- |
| Title bar | App icon, a back chevron, the tab strip, a run/expand glyph, the `+` new-tab button, then the window controls (minimize, maximize, close). |
| Tab strip | One tab per document; the active tab is underlined in the accent colour; each tab carries a close `✕` (shown on the active or hovered tab), and a document with unsaved changes shows a dot where that `✕` sits. |
| Top bar | The `File` / `Edit` / `View` menus, a divider, the formatting toolbar, and the right-hand controls (AI, feedback, account, settings). |
| Toolbar (left) | `H1` paragraph-style dropdown, `≡` list dropdown, `B`, `I`, `S`, a link glyph, a table glyph, `Aa` clear-formatting. |
| Toolbar (right) | Sparkle (AI) dropdown, feedback icon, account avatar, settings gear. |
| Status bar | `Ln`/`Col`, character count, an optional notification (the search notices in §5), document kind (`Plain text` / `Markdown`), zoom %, line-ending style, encoding. |

The window is drawn without OS decorations: the app draws its own title bar and
window controls.

---

## 2. Menus

Three top-level menus: `File`, `Edit`, `View`. Accelerators are shown right-aligned;
disabled items are greyed; toggle items carry a check mark.

### File

| Item | Shortcut |
| --- | --- |
| New tab | Ctrl+N |
| New window | Ctrl+Shift+N |
| New Markdown tab | — |
| *(separator)* | |
| Open | Ctrl+O |
| Recent ▸ | |
| *(separator)* | |
| Save | Ctrl+S |
| Save as | Ctrl+Shift+S |
| Save all | Ctrl+Alt+S |
| *(separator)* | |
| Page setup | — |
| Print | Ctrl+P |
| *(separator)* | |
| Close tab | Ctrl+W |
| Close window | Ctrl+Shift+W |
| Exit | Alt+F4 |

### Edit

| Item | Shortcut |
| --- | --- |
| Undo | Ctrl+Z |
| Cut | Ctrl+X |
| Copy | Ctrl+C |
| Paste | Ctrl+V |
| Delete | Del |
| *(separator)* | |
| Clear formatting | — |
| Define with Bing | Ctrl+E |
| *(separator)* | |
| Find | Ctrl+F |
| Find next | F3 |
| Find previous | Shift+F3 |
| Replace | Ctrl+H |
| Go to | Ctrl+G |
| *(separator)* | |
| Select all | Ctrl+A |
| Time/Date | F5 |

### View

| Item | Shortcut |
| --- | --- |
| Zoom ▸ | |
| ├ Zoom in | Ctrl+Plus |
| ├ Zoom out | Ctrl+Minus |
| └ Restore default zoom | Ctrl+0 |
| *(separator)* | |
| Status bar | — (toggle, checked) |
| Word wrap | — (toggle, checked) |
| *(separator)* | |
| Markdown ▸ | — (greyed for a plain document) |

---

## 3. Toolbar dropdowns

**Paragraph style** (`H1 ▾`): Title · Subtitle · Heading · Subheading · Section ·
Subsection · Body (the current style is highlighted).

**Lists** (`≡ ▾`): Bulleted list · Numbered list · Increase indent · Decrease
indent (indent items greyed when they cannot apply).

---

## 4. Settings page

Opened from the gear; replaces the editor surface. Two columns: the settings on the
left, "About this app" on the right.

### Left column

**Appearance**
- App theme — "Select which app theme to display" — dropdown (Use system setting / Light / Dark)

**Text Formatting**
- Font — dropdown
- Word wrap — "Fit text within window by default" — switch (On)
- Formatting — switch (On)

**Opening NoteCraft**
- Opening files — dropdown (Open in a new tab / Open in a new window / Replace the current tab)

### Right column — About this app

- Product name and version
- Copyright line
- Links: Software License Terms · Terms of Service · Privacy Statement · Third-Party Software Acknowledgments
- Buttons: Send feedback · Help

---

## 5. Find and Replace

*Decision — the reference capture does not include this dialog.* Opened from
`Edit ▸ Find` (`Ctrl+F`) and `Edit ▸ Replace` (`Ctrl+H`). It is one dialog in two
modes: **Find** shows the find row alone; **Replace** adds the replace row and
the `Replace` / `Replace All` buttons. The dialog is modeless — the editor stays
usable behind it — and sits as a floating card at the top-right of the editor
surface. `Esc` and `Close` dismiss it; the find text, both options, the
direction and the wrap choice are **kept for the next Find** (a later `Ctrl+F`,
or `F3` / `Shift+F3`). The engine behind it is `nc-textproc::search`
(NC-S001–NC-S005).

```
┌ Find and Replace ─────────────────────────────────────────────┐
│  Find what      [__________________________________________] │
│  Replace with   [__________________________________________] │
│  [ ] Match case      [ ] Match whole word only                │
│  Direction      (•) Down    ( ) Up                            │
│  [✓] Wrap around                                              │
│                    [Find Next] [Replace] [Replace All] [Close] │
└───────────────────────────────────────────────────────────────┘
```

| Control | Behaviour |
| --- | --- |
| Find what | The search text. Empty disables `Find Next`, `Replace` and `Replace All`. |
| Replace with | The replacement text (Replace mode). May be empty, which deletes each match. |
| Match case | Off when the dialog opens; kept for the next Find. |
| Match whole word only | Off when the dialog opens; kept for the next Find. |
| Direction | `Down` (forward, the default) or `Up` (backward). |
| Wrap around | Checked by default. When cleared, a search stops at the end it travels towards instead of continuing from the other end. |
| Find Next | Selects the next match and moves the caret to it. `F3` / `Shift+F3` repeat it without the dialog. |
| Replace | Replaces the current match and advances to the next. |
| Replace All | Replaces every match as **one** undoable action. |
| Close | Dismisses the dialog; the entries are kept. |

**Notices** are shown on the status-bar notification line (§1), not in the
dialog, and are the shell's wording for the engine's `SearchResult`:

| Result | Notice |
| --- | --- |
| `NoMatch` | `Cannot find "…"` |
| `Wrapped` | `Reached the end of the document and wrapped around` (or *beginning*, for `Up`) |
| `Boundary` (wrap off) | `Reached the end of the document` (or *beginning*) |

## 6. Go to line

*Decision — the reference capture does not include this dialog.* Opened from
`Edit ▸ Go to` (`Ctrl+G`). The field starts with the caret's current line,
selected. The engine is `nc-textproc::search::parse_line_number` and `go_to_line`
(NC-S006).

```
┌ Go to line ───────────────────────────┐
│  Line number   [__________]           │
│                 [Go to]   [Cancel]    │
└───────────────────────────────────────┘
```

| Control | Behaviour |
| --- | --- |
| Line number | Digits with surrounding blanks only. Anything else — empty, letters, a sign — leaves `Go to` disabled. |
| Go to | Moves the caret to the **start of that line** and closes the dialog. `Enter` does the same. |
| Cancel | Closes without moving. `Esc` does the same. |

An out-of-range number **clamps** rather than failing: `0` or `1` is the start of
the document, and a number past the last line is the start of the last line.
Lines are counted from 1, the way the status bar counts them.

## 7. Open and Save As

*Decision — the reference capture does not include this dialog.* Opened from
`File ▸ Open` (`Ctrl+O`) and `File ▸ Save as` (`Ctrl+Shift+S`); the two share one
frame and differ in their title, their primary button, and the encoding
default. The shell browses the filesystem itself; `nc-textproc::TextFile` does
the reading and writing, and detects the encoding (NC-F004–NC-F006, NC-D001–NC-D004).

```
┌ Open ──────────────────────────────────────────────────────────┐
│  [ ‹ ]  /home/user/Documents                                    │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Documents/                        (folders first)        │ │
│  │  notes.txt                                                │ │
│  └───────────────────────────────────────────────────────────┘ │
│  File name   [_____________________________________]            │
│  Encoding ▾  Auto-detect                                        │
│                                     [Open]   [Cancel]           │
└─────────────────────────────────────────────────────────────────┘
```

| Control | Behaviour |
| --- | --- |
| Path / `‹` | The folder being browsed; `‹` goes up one level. |
| List | The current folder only, folders before files. A double-click enters a folder or opens a file; a single click selects it and fills **File name**. |
| File name | Editable, and may be a path. The primary button is disabled while it is empty. |
| Encoding | `Auto-detect` (default when **Open**) or one of: `UTF-8` · `UTF-8 with BOM` · `UTF-16 LE` · `UTF-16 BE` · `ANSI`. **Save as** defaults to the open document's own encoding instead of `Auto-detect`. |
| Open / Save | Reads through `TextFile::open`, or writes through `TextFile::save`/`save_as`. |
| Cancel | Closes without a change. |

Opening a path that does not exist, or a file whose chosen encoding cannot decode
it, shows the reason in the dialog and leaves it open. A **read-only** or
otherwise unwritable file, or a chosen encoding that cannot hold the text, does
the same for Save As. Saving over an existing file raises the overwrite prompt
(§9) before any bytes are written.

## 8. Page setup

*Decision — the reference capture does not include this dialog; it is M8 and
**greyed** until then.* Opened from `File ▸ Page setup`. It describes the printed
page (NC-F007).

| Field | Options |
| --- | --- |
| Paper size | `Letter` · `A4` · `Legal` |
| Orientation | `Portrait` · `Landscape` |
| Margins | Left · Right · Top · Bottom, in millimetres |
| Header / Footer | Text with `&p` (page number) and `&d` (date) placeholders |

Buttons: `Print` (raises the system print pipeline) and `Cancel`. `File ▸ Print`
(`Ctrl+P`) prints with the current settings and does not open this dialog.

## 9. Prompts

*Decision — the reference capture does not include these dialogs.* Small modal
dialogs raised **before** something is destroyed. `nc-textproc::prompt` decides
whether each one is asked and hands the shell the state; the wording below is
the shell's. `Esc` always cancels, and no prompt proceeds on its own.

| Prompt | Shown when | Text and buttons |
| --- | --- | --- |
| Unsaved changes | A tab or the window is closing with a dirty document (NC-D005) | `Save changes to "<name>"?` — `Save` · `Don't save` · `Cancel` |
| Overwrite | Save As names a file that already exists (NC-F006) | `"<name>" already exists. Do you want to replace it?` — `Replace` · `Cancel` |
| Unrepresentable | Saving would lose characters the encoding cannot hold (NC-D003) | `<n> characters cannot be saved in <encoding>. Saving will replace them with "?". Save anyway?` — `Save anyway` · `Choose another encoding` · `Cancel` |
| Outside change | The file changed on disk since NoteCraft read it (NC-D008) | `"<name>" has changed on disk since it was last read.` — `Reload` · `Keep my version` · `Cancel` |

`Don't save` discards the edits and continues the close; `Keep my version`
leaves the editor's text as it is and marks the document dirty against the file
now on disk. `Choose another encoding` returns to the Save As dialog (§7) with
the encoding list focused.

---

## 10. Command registry

`crates/commands` (`nc-commands`) is the single authoritative definition of every
command: its label, menu placement, default shortcut, availability rule, and the
milestone that first gives it an effect. The menus above are generated from it, so
the drawn UI and the registry cannot drift apart.
