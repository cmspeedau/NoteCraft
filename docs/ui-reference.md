# UI Reference — NoteCraft

Source of truth for the UI NoteCraft builds. It is captured from the supplied
screens of the modern Notepad app (dark theme): the main window, the File / Edit /
View menus, the paragraph-style and list dropdowns, and the Settings page.

Unlike a byte-level software spec, this is a **design reference**: it fixes the
layout, the menu items, the toolbar controls, and the status-bar fields. Anything
the screens do not show is a design decision recorded in the command registry
(`crates/commands`), not a guess.

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
| Tab strip | One tab per document; the active tab is underlined in the accent colour; each tab carries a close `✕` (shown on the active or hovered tab). |
| Top bar | The `File` / `Edit` / `View` menus, a divider, the formatting toolbar, and the right-hand controls (AI, feedback, account, settings). |
| Toolbar (left) | `H1` paragraph-style dropdown, `≡` list dropdown, `B`, `I`, `S`, a link glyph, a table glyph, `Aa` clear-formatting. |
| Toolbar (right) | Sparkle (AI) dropdown, feedback icon, account avatar, settings gear. |
| Status bar | `Ln`/`Col`, character count, document kind (`Plain text` / `Markdown`), zoom %, line-ending style, encoding. |

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

## 5. Command registry

`crates/commands` (`nc-commands`) is the single authoritative definition of every
command: its label, menu placement, default shortcut, availability rule, and the
milestone that first gives it an effect. The menus above are generated from it, so
the drawn UI and the registry cannot drift apart.
