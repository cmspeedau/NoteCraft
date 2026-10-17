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

*Decision — the capture cannot show this.* A screenshot has no scrollbar in it, so
nothing in §1 asks for the editor to scroll, and no acceptance row covers it. A document
longer than the window is unreadable without one, though, so the editor scrolls on both
axes: vertically for a tall document, horizontally for a line too wide for the window when
`View ▸ Word wrap` is off (§4). The caret is kept on screen as it moves, whether it moves
under a key, a click, or a command that names a position in the document (Find, Go to
line, Select all). The rendered Markdown view scrolls vertically the same way.

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

### What the controls do

*Decision — the capture shows the page and its control labels, but a screenshot cannot
show what a setting changes, and it was taken in one theme on one machine.*

| Control | Behaviour |
| --- | --- |
| App theme | The three choices mean what they say: `Light` and `Dark` are obeyed whatever the operating system reports; `Use system setting` follows the system, and where a platform reports nothing the dark palette stays — the look this file was captured in — rather than guessing light. **The light palette has no capture to copy**, so it keeps the dark one's relationships (content surfaces lighter than the chrome, settings cards the same fill as the chrome, borders a step beyond it, text a step off the extreme) with the accent taken from Windows 11's light-mode blue. Nothing about layout differs between themes: same widgets, same positions, different colours. |
| Theme and extra windows (§11) | Every window of a run shares one `egui` style, so the palette is installed at the start of each window's own drawing rather than once per application — a second window asking for another palette cannot leave the first one painted in the wrong colours. Whether the choice belongs to the window it was made in or to the whole application the capture does not say; the settings page itself belongs to the window it is open in. A new window starts as the window that opened it looks. |
| Word wrap | Wrapping is how a document is *shown*, never a change to it: no byte moves and the tab does not become unsaved. Off means a line stays one line and the editor grows wider than the window, with a horizontal bar to reach the rest of it. The same control is on the View menu (§2); the two are one setting. |
| Formatting | Recorded so the row is not read as finished: this switch's behaviour — what it turns off, and what the toolbar looks like without it — is not implemented, and `NC-P005` is the row that owes it. |
| Opening files | Recorded so the row is not read as finished: the choice is remembered across runs, and nothing acts on it yet. Section G of the matrix carries **no row for this control** — the capture shows it, so the missing thing is the acceptance row, not the screen. |
| What is remembered, and when | A setting is written a frame after it changes, not on exit, so a crash does not undo what the user chose; nothing is written until something *does* change, so an unconfigured run leaves no file behind. The file is `settings` beside the session file, `key value` lines with a version header: a key this build does not know is kept and written back untouched, a value it cannot honour falls back to that key's default, and a header it does not recognise means no settings at all. The main window owns the file — the same policy the session follows — so an extra window changing a setting cannot fight it. |

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

## 11. Windows and tab lifecycle

*Decision — the reference capture does not include these behaviours.* They are
recorded here because the screens do not show them:

| Command | Behaviour |
| --- | --- |
| New window (Ctrl+Shift+N) | Opens an independent top-level window with one empty untitled document: its own tab strip, active document, and view options, separate from the window it was opened from. It closes by the same window-close path as the main window. |
| Close window (Ctrl+Shift+W) / Exit (Alt+F4) | Closes the window, raising the unsaved-changes prompt (§9) for each dirty document in turn. Cancelling any prompt stops the close. There is no separate "close all tabs" item in the captured menus; closing the window is how every document is closed (NC-T005). |
| New tab (Ctrl+N) | Adds an empty plain-text tab and makes it active, leaving the other tabs untouched. |

A file whose **canonical path** is already open is not opened a second time:
`File ▸ Open`, and a command-line argument, focus the tab that already shows that
file instead (NC-F012).


## 12. Markdown formatting

*Decision — the reference capture shows the controls (§1, §3) and the greyed
`Markdown ▸` item (§2), but never what their items write.* The behaviours the
screens cannot show are recorded here:

| Control | Behaviour |
| --- | --- |
| Paragraph style (`H1 ▾`) | The seven styles are one per ATX level, in the order §3 lists them: Title `#`, Subtitle `##`, Heading `###`, Subheading `####`, Section `#####`, Subsection `######`, and `Body` as ordinary text. Choosing the style a line already carries takes the marker off, which is what makes the highlight in §3 worth showing. A line that carries a list or quote marker gives it up rather than ending up `- # Title`. |
| Lists (`≡ ▾`) | Bulleted list writes `- `, numbered list numbers from `1.` across the selected lines. Choosing the kind the lines already are takes the markers off. Indentation is left alone, so the nesting of an item survives a change of kind. |
| Increase / Decrease indent | One level is two spaces, added to or taken from the front of each selected line. |
| `B` / `I` / `S` | Wrap the selection in `**`, `*`, `~~`. The selection then covers the markers, so pressing the same command again unwraps them; selecting the marked *words* (markers outside the selection) unmarks them. With only a caret, both markers are written and the caret stays between them. |
| Link glyph (`Ctrl+K`) | **The capture includes no hyperlink dialog**, and §5–§9 specify none, so there is no item set to build one from: the command writes `[selected text]()` and leaves the caret between the parentheses to type the target. A bare caret gets `[]()` with the caret inside the brackets. |
| Table glyph | **The capture includes no table dialog**, so the command asks nothing of the user: fired outside a table it inserts a fixed GitHub pipe table — three columns, two body rows, header and delimiter — and leaves the caret in the first header cell. **Fired inside a table it grows that table by one row** instead: the row appears below the caret's row, carries as many cells as the table's widest row, and can never land between a header and its delimiter (a row there is a broken table, not a row), so a caret in the header or the delimiter makes the new row the first body row. Existing rows are never resized or re-padded — the command lengthens a table, it does not tidy it. *A decision, because the registry has one table command and the capture shows no dialog: the document decides which job the keystroke is doing.* Adding a **column** has nothing to fire it — no dialog, no item, no shortcut — so it is not implemented; widening a table is a decision still outstanding. |
| `Aa` clear formatting | Removes heading, list and blockquote markers, and the indentation in front of them; then `**`, `~~`, and paired `*`. Underscores stay, so `snake_case_words` survives, and so does a marker with nothing to pair with — `2 * 3 = 6` is arithmetic, not emphasis. |
| Block commands and the selection | Headings, lists and indentation apply to every line a selection touches, which is what selecting partway through a paragraph means; an inline marker has to be inside the selection to be wrapped or removed. A selection that ends exactly at the start of a line does not drag that line in. |
| Plain-text document | Every Markdown command is greyed (§2's `Markdown ▸` rule), and the handlers refuse to run on a plain document rather than write `**` into a `.txt` file. |
| `.md` / `.markdown` | A file is Markdown by extension, matched case-insensitively; anything else, including `.mdown` and `.md.txt`, opens as plain text (NC-M001). |
| Source / Formatted view (`View ▸ Markdown`) | **The capture never shows a rendered document**, so what the formatted view looks like is a decision rather than a copy. It is *derived and read-only*: the source is the document (§6 forbids a rich internal tree that could lose bytes), so there is never a second copy of the text to keep in step, and typing always happens in the source view. Exactly one of the two items carries a check mark. The choice belongs to the tab, so two Markdown documents can be shown two ways at once. |
| Which view a document opens in | **Source view** — a `.md` file, a restored session, and a new Markdown tab alike. A rendered view takes no typing, so a document that opened there would open unreadable to the keyboard. |
| What the rendered view shows | Headings at six sizes, `#` largest; `**`, `*`, `~~` and backticks drawn as weight, slant, strike and a smaller face rather than as their markers; a `-`, `+` or `*` item as a bullet and a numbered item keeping its number; a blockquote indented and dimmed; a pipe table left in the mono face so its columns line up; a link's label in the accent colour. Nesting is drawn with the source's own spaces, which an all-monospace rendering makes exact. |
| What the rendered view must not do | Hide anything. Only the delimiters of emphasis the parser actually recognised come off screen; `[^1]:`, raw HTML, fences, `==highlight==`, `$x^2$` and an unpaired `**` are painted as written, in step with the rule that nothing is silently normalised (NC-M011). |
| What a view switch costs | Nothing: not an undo step, not a dirty flag, not a selection. Coming back hands the caret to the editor widget, which is newly created and would otherwise start at offset zero (NC-M002). |
| Across a restart | The session stores each document's path, encoding, line endings and unsaved text, but **not which view it was in**: a restored Markdown document opens in source view. |

Markdown syntax the editor does not implement — footnotes, raw HTML, fenced code,
definition lists, `==highlight==`, admonitions — is not parsed and not rewritten:
it is text, it is saved back byte for byte, and formatting a neighbouring line
does not touch it (NC-M011).
