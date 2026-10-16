//! The shell's observable state: open tabs, the active document, and the view
//! options the chrome reflects.

use std::path::{Path, PathBuf};

use nc_commands::{AvailabilityContext, CommandId};
use nc_core::{Document, LineEnding};
use nc_textproc::{Direction, Encoding, SearchOptions, TextFile};

/// Whether a document is plain text or Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    /// Plain text.
    Plain,
    /// Markdown.
    Markdown,
}

/// The paragraph style selected in the formatting toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphStyle {
    /// `Title`.
    Title,
    /// `Subtitle`.
    Subtitle,
    /// `Heading`.
    Heading,
    /// `Subheading`.
    Subheading,
    /// `Section`.
    Section,
    /// `Subsection`.
    Subsection,
    /// `Body`.
    Body,
}

impl ParagraphStyle {
    /// Every style, in the order the dropdown lists them.
    pub const ALL: [ParagraphStyle; 7] = [
        ParagraphStyle::Title,
        ParagraphStyle::Subtitle,
        ParagraphStyle::Heading,
        ParagraphStyle::Subheading,
        ParagraphStyle::Section,
        ParagraphStyle::Subsection,
        ParagraphStyle::Body,
    ];

    /// The label shown in the dropdown.
    pub const fn label(self) -> &'static str {
        match self {
            ParagraphStyle::Title => "Title",
            ParagraphStyle::Subtitle => "Subtitle",
            ParagraphStyle::Heading => "Heading",
            ParagraphStyle::Subheading => "Subheading",
            ParagraphStyle::Section => "Section",
            ParagraphStyle::Subsection => "Subsection",
            ParagraphStyle::Body => "Body",
        }
    }

    /// The command this style fires.
    ///
    /// The dropdown and the menu have to do the same thing, and only the command
    /// layer knows what a style writes, so the control names a command rather than
    /// editing the tab itself.
    pub const fn command(self) -> CommandId {
        match self {
            ParagraphStyle::Title => CommandId("cmd.format.title"),
            ParagraphStyle::Subtitle => CommandId("cmd.format.subtitle"),
            ParagraphStyle::Heading => CommandId("cmd.format.heading"),
            ParagraphStyle::Subheading => CommandId("cmd.format.subheading"),
            ParagraphStyle::Section => CommandId("cmd.format.section"),
            ParagraphStyle::Subsection => CommandId("cmd.format.subsection"),
            ParagraphStyle::Body => CommandId("cmd.format.body"),
        }
    }
}

/// The app theme chosen in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    /// Follow the operating system.
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl AppTheme {
    /// Every choice, in menu order.
    pub const ALL: [AppTheme; 3] = [AppTheme::System, AppTheme::Light, AppTheme::Dark];

    /// The label shown in the dropdown.
    pub const fn label(self) -> &'static str {
        match self {
            AppTheme::System => "Use system setting",
            AppTheme::Light => "Light",
            AppTheme::Dark => "Dark",
        }
    }
}

/// Where a newly opened file goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenBehavior {
    /// Open in a new tab.
    NewTab,
    /// Open in a new window.
    NewWindow,
    /// Replace the current tab.
    ReplaceTab,
}

impl OpenBehavior {
    /// Every choice, in menu order.
    pub const ALL: [OpenBehavior; 3] = [
        OpenBehavior::NewTab,
        OpenBehavior::NewWindow,
        OpenBehavior::ReplaceTab,
    ];

    /// The label shown in the dropdown.
    pub const fn label(self) -> &'static str {
        match self {
            OpenBehavior::NewTab => "Open in a new tab",
            OpenBehavior::NewWindow => "Open in a new window",
            OpenBehavior::ReplaceTab => "Replace the current tab",
        }
    }
}

/// The Find and Replace dialog (`docs/ui-reference.md` §5).
///
/// One dialog in two modes: **Find** shows the find row alone, **Replace** adds
/// the replace row and the Replace buttons. The find text, both options, the
/// direction and the wrap choice are kept between shows, which is what lets
/// `F3` repeat a find after the dialog has been closed.
pub struct FindDialog {
    /// Whether the dialog is on screen.
    pub open: bool,
    /// Whether the Replace row and buttons are shown (`Ctrl+H`, not `Ctrl+F`).
    pub replace: bool,
    /// The search text.
    pub query: String,
    /// The replacement text.
    pub replacement: String,
    /// Match case / match whole word, kept between finds.
    pub options: SearchOptions,
    /// The direction `Find Next` travels in.
    pub direction: Direction,
    /// Whether the search wraps around.
    pub wraps: bool,
    /// Set when the dialog opens, so its first field takes focus once.
    pub focus: bool,
}

impl FindDialog {
    /// A closed dialog with the reference defaults: both options off,
    /// searching down, wrapping on.
    pub fn new() -> Self {
        Self {
            open: false,
            replace: false,
            query: String::new(),
            replacement: String::new(),
            options: SearchOptions::new(),
            direction: Direction::Forward,
            wraps: true,
            focus: false,
        }
    }
}

impl Default for FindDialog {
    fn default() -> Self {
        Self::new()
    }
}

/// The Go to line dialog (`docs/ui-reference.md` §6).
pub struct GoToDialog {
    /// Whether the dialog is on screen.
    pub open: bool,
    /// The typed line number.
    pub line: String,
    /// Set when the dialog opens, so the field takes focus once.
    pub focus: bool,
}

impl GoToDialog {
    /// A closed dialog with an empty field.
    pub fn new() -> Self {
        Self {
            open: false,
            line: String::new(),
            focus: false,
        }
    }
}

impl Default for GoToDialog {
    fn default() -> Self {
        Self::new()
    }
}

/// Which file dialog is open (`docs/ui-reference.md` §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogMode {
    /// Read a file into a new tab.
    Open,
    /// Write a tab to a named file.
    SaveAs,
}

/// The encoding the Open / Save As dialog will use (`docs/ui-reference.md` §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingChoice {
    /// Work it out from the bytes (Open only).
    Auto,
    /// A chosen encoding, and whether the file carries a byte order mark.
    Chosen {
        /// The encoding to read or write.
        encoding: Encoding,
        /// Whether a mark is written.
        bom: bool,
    },
}

impl EncodingChoice {
    /// The list Open offers, in the order the dropdown shows it.
    pub const OPEN: &'static [Self] = &[
        Self::Auto,
        Self::Chosen {
            encoding: Encoding::Utf8,
            bom: false,
        },
        Self::Chosen {
            encoding: Encoding::Utf8,
            bom: true,
        },
        Self::Chosen {
            encoding: Encoding::Utf16Le,
            bom: true,
        },
        Self::Chosen {
            encoding: Encoding::Utf16Be,
            bom: true,
        },
        Self::Chosen {
            encoding: Encoding::Ansi,
            bom: false,
        },
    ];

    /// The list Save As offers: the same, without `Auto-detect`.
    pub const SAVE_AS: &'static [Self] = &[
        Self::Chosen {
            encoding: Encoding::Utf8,
            bom: false,
        },
        Self::Chosen {
            encoding: Encoding::Utf8,
            bom: true,
        },
        Self::Chosen {
            encoding: Encoding::Utf16Le,
            bom: true,
        },
        Self::Chosen {
            encoding: Encoding::Utf16Be,
            bom: true,
        },
        Self::Chosen {
            encoding: Encoding::Ansi,
            bom: false,
        },
    ];

    /// The label the dropdown shows.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto-detect",
            Self::Chosen { encoding, bom } => match (encoding, bom) {
                (Encoding::Utf8, false) => "UTF-8",
                (Encoding::Utf8, true) => "UTF-8 with BOM",
                (Encoding::Utf16Le, _) => "UTF-16 LE",
                (Encoding::Utf16Be, _) => "UTF-16 BE",
                (Encoding::Ansi, _) => "ANSI",
                (Encoding::Utf32Le, _) => "UTF-32 LE",
                (Encoding::Utf32Be, _) => "UTF-32 BE",
            },
        }
    }

    /// The chosen encoding and mark, or `None` for `Auto-detect`.
    pub const fn chosen(self) -> Option<(Encoding, bool)> {
        match self {
            Self::Auto => None,
            Self::Chosen { encoding, bom } => Some((encoding, bom)),
        }
    }
}

/// One row in the file dialog's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// The file or folder's name.
    pub name: String,
    /// Its full path.
    pub path: PathBuf,
    /// Whether it is a folder.
    pub is_dir: bool,
}

/// The Open / Save As dialog (`docs/ui-reference.md` §7).
///
/// One frame in two modes: Open browses and reads, Save As names a file and
/// writes it. The browsing, the reading and the writing are the shell's; the
/// dialog only holds what is on screen.
pub struct FileDialog {
    /// Whether this dialog opens or saves.
    pub mode: FileDialogMode,
    /// The tab Save As writes.
    pub tab: usize,
    /// The folder being browsed.
    pub directory: PathBuf,
    /// The folder's contents, folders first.
    pub entries: Vec<DirEntry>,
    /// The File name field, which may be a bare name or a path.
    pub file_name: String,
    /// The chosen encoding.
    pub encoding: EncodingChoice,
    /// Why the last Open or Save failed, shown in the dialog.
    pub error: Option<String>,
    /// Set when the dialog opens, so the File name field takes focus once.
    pub focus: bool,
}

impl FileDialog {
    /// The dialog as `File ▸ Open` shows it, browsing the working folder.
    pub fn open() -> Self {
        let mut dialog = Self {
            mode: FileDialogMode::Open,
            tab: 0,
            directory: default_directory(),
            entries: Vec::new(),
            file_name: String::new(),
            encoding: EncodingChoice::Auto,
            error: None,
            focus: true,
        };
        dialog.refresh();
        dialog
    }

    /// The dialog as `File ▸ Save as` shows it, named after `tab`.
    pub fn save_as(tab: usize, name: String, encoding: Encoding, bom: bool) -> Self {
        let mut dialog = Self {
            mode: FileDialogMode::SaveAs,
            tab,
            directory: default_directory(),
            entries: Vec::new(),
            file_name: name,
            encoding: EncodingChoice::Chosen { encoding, bom },
            error: None,
            focus: true,
        };
        dialog.refresh();
        dialog
    }

    /// The encoding choices this mode offers.
    pub fn choices(&self) -> &'static [EncodingChoice] {
        match self.mode {
            FileDialogMode::Open => EncodingChoice::OPEN,
            FileDialogMode::SaveAs => EncodingChoice::SAVE_AS,
        }
    }

    /// Re-reads the folder being browsed.
    pub fn refresh(&mut self) {
        self.entries = read_directory(&self.directory);
    }

    /// Goes up one folder. Returns whether there was one to go to.
    pub fn go_up(&mut self) -> bool {
        let Some(parent) = self.directory.parent() else {
            return false;
        };
        self.directory = parent.to_path_buf();
        self.refresh();
        true
    }

    /// Enters `path` if it names a folder.
    pub fn enter(&mut self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        if !path.is_dir() {
            return false;
        }
        self.directory = path.to_path_buf();
        self.file_name.clear();
        self.refresh();
        true
    }

    /// The path the File name field names: as typed when it is absolute,
    /// otherwise inside the browsed folder.
    pub fn target(&self) -> PathBuf {
        let typed = Path::new(&self.file_name);
        if typed.is_absolute() {
            typed.to_path_buf()
        } else {
            self.directory.join(typed)
        }
    }
}

/// The folder a dialog opens on.
fn default_directory() -> PathBuf {
    std::env::current_dir()
        .ok()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Reads a folder for the dialog: folders before files, each alphabetical and
/// case-insensitive. An unreadable folder lists nothing.
fn read_directory(directory: &Path) -> Vec<DirEntry> {
    let Ok(read) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut entries: Vec<DirEntry> = read
        .flatten()
        .map(|entry| {
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            DirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
                is_dir,
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

/// A prompt raised before something is destroyed (`docs/ui-reference.md` §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    /// A dirty tab is closing.
    UnsavedChanges {
        /// The tab about to close.
        tab: usize,
    },
    /// Save As names a file that already exists.
    Overwrite {
        /// The file that would be replaced.
        path: PathBuf,
        /// The tab being written.
        tab: usize,
        /// The encoding Save As chose.
        encoding: Encoding,
        /// Whether to write a byte order mark.
        bom: bool,
    },
    /// Saving would replace characters the encoding cannot hold.
    Unrepresentable {
        /// The tab being written.
        tab: usize,
        /// The encoding the file would be written in.
        encoding: Encoding,
        /// The characters with no representation.
        characters: Vec<char>,
    },
}

/// A single open document: the file behind it, plus the shell-only state the
/// toolbar and tab strip show.
///
/// The [`TextFile`] is the single authoritative source for the text, the path,
/// the encoding, the line endings and the dirty flag. `nc-platform` only adds
/// what the viewport needs on top.
pub struct Tab {
    /// The file: its path, encoding, bytes, text, and undo history.
    pub file: TextFile,
    /// Whether the document is plain text or Markdown.
    pub kind: DocKind,
    /// The paragraph style selected in the toolbar.
    pub paragraph: ParagraphStyle,
    /// Whether the paragraph style is being edited in formatted view.
    pub formatted: bool,
}

impl Tab {
    /// A new, empty plain-text document.
    pub fn plain() -> Self {
        Self {
            file: TextFile::untitled(),
            kind: DocKind::Plain,
            paragraph: ParagraphStyle::Body,
            formatted: false,
        }
    }

    /// A new, empty Markdown document.
    pub fn markdown() -> Self {
        Self {
            file: TextFile::untitled(),
            kind: DocKind::Markdown,
            paragraph: ParagraphStyle::Body,
            formatted: true,
        }
    }

    /// Opens `path`, detecting its encoding and line endings (`NC-F004`).
    pub fn open(path: impl AsRef<Path>) -> nc_textproc::Result<Self> {
        let file = TextFile::open(path)?;
        let markdown = is_markdown(file.path());
        Ok(Self {
            file,
            kind: if markdown {
                DocKind::Markdown
            } else {
                DocKind::Plain
            },
            paragraph: ParagraphStyle::Body,
            formatted: markdown,
        })
    }

    /// Opens `path` as `encoding` rather than detecting it — the Open dialog's
    /// encoding list (`docs/ui-reference.md` §7).
    pub fn open_as(
        path: impl AsRef<Path>,
        encoding: Encoding,
        bom: bool,
    ) -> nc_textproc::Result<Self> {
        let file = TextFile::open_as(path, encoding, bom)?;
        let markdown = is_markdown(file.path());
        Ok(Self {
            file,
            kind: if markdown {
                DocKind::Markdown
            } else {
                DocKind::Plain
            },
            paragraph: ParagraphStyle::Body,
            formatted: markdown,
        })
    }

    /// Rebuilds a tab from a file and the kind the session recorded, for session
    /// restore (`NC-F011`). Unlike [`open`](Self::open) the kind is given rather
    /// than deduced from a path, so an unsaved Markdown document keeps its kind.
    pub fn restore(file: TextFile, kind: DocKind) -> Self {
        Self {
            file,
            kind,
            paragraph: ParagraphStyle::Body,
            formatted: kind == DocKind::Markdown,
        }
    }

    /// The name shown on the tab and in the title bar.
    pub fn title(&self) -> String {
        self.file
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned())
    }

    /// The document text.
    pub fn text(&self) -> &str {
        self.file.text()
    }

    /// The document, to edit in place.
    pub fn document_mut(&mut self) -> &mut Document {
        self.file.document_mut()
    }

    /// Whether the document has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.file.is_dirty()
    }

    /// The encoding name the status bar shows.
    pub fn encoding_label(&self) -> &'static str {
        self.file.encoding_label()
    }

    /// The line-ending name the status bar shows.
    pub fn line_ending_label(&self) -> &'static str {
        match self.file.line_ending() {
            LineEnding::CrLf => "Windows (CRLF)",
            LineEnding::Lf => "Unix (LF)",
            LineEnding::Cr => "Mac (CR)",
        }
    }

    /// The label the status bar shows for the document kind.
    pub const fn kind_label(&self) -> &'static str {
        match self.kind {
            DocKind::Plain => "Plain text",
            DocKind::Markdown => "Markdown",
        }
    }

    /// Whether this tab shows `path`, comparing canonical paths so two spellings
    /// of one file count as the same document (`NC-F012`).
    pub fn matches_path(&self, path: &Path) -> bool {
        self.file.has_path() && canonical_path(self.file.path()) == canonical_path(path)
    }
}

/// A path in canonical form, or the path itself when it cannot be resolved yet
/// (it does not exist, say). Used to recognize two spellings of one file.
fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Whether a path names a Markdown document.
fn is_markdown(path: &Path) -> bool {
    let name = path.to_string_lossy().to_ascii_lowercase();
    name.ends_with(".md") || name.ends_with(".markdown")
}

/// The editor font families offered in Settings.
pub const FONT_FAMILIES: [&str; 4] = ["Consolas", "Cascadia Mono", "Segoe UI", "Courier New"];

/// How many paths `File ▸ Recent` keeps (`NC-F010`).
pub const RECENT_LIMIT: usize = 10;

/// The whole shell state.
pub struct Shell {
    /// The open documents, in tab order.
    pub tabs: Vec<Tab>,
    /// The index of the active tab.
    pub active: usize,
    /// The documents opened this session, most recent first (`NC-F010`). Shown
    /// as `File ▸ Recent`; a path opened twice moves back to the front rather
    /// than appearing twice.
    pub recent_files: Vec<PathBuf>,
    /// Whether the settings page is shown instead of the editor.
    pub show_settings: bool,
    /// Whether the status bar is visible (`View ▸ Status bar`).
    pub status_bar_visible: bool,
    /// Whether word wrap is on (`View ▸ Word wrap`).
    pub word_wrap: bool,
    /// Whether a Markdown document is shown formatted.
    pub formatted_view: bool,
    /// The zoom percentage.
    pub zoom: u32,
    /// The chosen app theme.
    pub theme: AppTheme,
    /// The editor font family name.
    pub font_family: &'static str,
    /// Where opened files go.
    pub opening: OpenBehavior,
    /// Whether the AI service is available (enables the sparkle menu items).
    pub ai_enabled: bool,
    /// The 1-based caret line, for the status bar.
    pub line: usize,
    /// The 1-based caret column, for the status bar.
    pub column: usize,
    /// The caret's character index in the active document.
    pub caret: usize,
    /// A selection the editor applies on its next frame, as
    /// `(primary, secondary)` character indices — `egui` calls the moving end
    /// the primary one, so it is the end a following Find travels from.
    pub pending_selection: Option<(usize, usize)>,
    /// A one-line notice for the status bar (`docs/ui-reference.md` §5).
    pub notification: Option<String>,
    /// The Find and Replace dialog.
    pub find: FindDialog,
    /// The Go to line dialog.
    pub go_to: GoToDialog,
    /// The Open / Save As dialog, when one is on screen.
    pub file_dialog: Option<FileDialog>,
    /// The prompt on screen, when one is waiting for an answer.
    pub prompt: Option<Prompt>,
    /// The tab to close once a Save As opened for it succeeds
    /// (`docs/ui-reference.md` §9: Save continues the close).
    pub close_after_save: Option<usize>,
    /// Whether the window is closing, prompting for each dirty tab in turn.
    pub closing_window: bool,
    /// Set when the window should close once the prompts are answered.
    pub exit_pending: bool,
    /// Set by `File ▸ New window`; the eframe application creates the window at
    /// the end of the frame and clears it (`NC-F003`).
    pub new_window_request: bool,
}

impl Shell {
    /// The shell at first launch: one empty, clean plain-text document.
    pub fn new() -> Self {
        Self {
            tabs: vec![Tab::plain()],
            active: 0,
            recent_files: Vec::new(),
            show_settings: false,
            status_bar_visible: true,
            word_wrap: true,
            formatted_view: true,
            zoom: 100,
            theme: AppTheme::Dark,
            font_family: FONT_FAMILIES[0],
            opening: OpenBehavior::NewTab,
            ai_enabled: false,
            line: 1,
            column: 1,
            caret: 0,
            pending_selection: None,
            notification: None,
            find: FindDialog::new(),
            go_to: GoToDialog::new(),
            file_dialog: None,
            prompt: None,
            close_after_save: None,
            closing_window: false,
            exit_pending: false,
            new_window_request: false,
        }
    }

    /// The active tab, if any.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    /// The active tab mutably, if any.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    /// The availability context derived from the current state.
    pub fn availability(&self) -> AvailabilityContext {
        let tab = self.active_tab();
        AvailabilityContext {
            document_present: tab.is_some(),
            dirty: tab.map(Tab::is_dirty).unwrap_or(false),
            has_selection: tab
                .map(|tab| tab.file.document().has_selection())
                .unwrap_or(false),
            markdown_enabled: tab
                .map(|tab| tab.kind == DocKind::Markdown)
                .unwrap_or(false),
            service_available: self.ai_enabled,
            tab_count: self.tabs.len(),
            status_bar_visible: self.status_bar_visible,
            word_wrap: self.word_wrap,
            formatted_view: self.formatted_view,
        }
    }

    /// Opens a new tab of the given kind and makes it active.
    pub fn open_tab(&mut self, kind: DocKind) {
        let tab = match kind {
            DocKind::Plain => Tab::plain(),
            DocKind::Markdown => Tab::markdown(),
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    /// Records `path` as the most recently opened document (`NC-F010`).
    ///
    /// Identity follows [`Tab::matches_path`]: two spellings of one file are one
    /// entry, so reopening a document moves it back to the front instead of
    /// adding a second line for it. Older entries past [`RECENT_LIMIT`] are
    /// dropped, which is why the newest entry leads.
    pub fn remember_recent(&mut self, path: &Path) {
        let wanted = canonical_path(path);
        self.recent_files
            .retain(|open| canonical_path(open) != wanted);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(RECENT_LIMIT);
    }

    /// Closes the tab at `index`. The last remaining tab is replaced by a fresh
    /// untitled document rather than leaving the window with no document.
    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.tabs.push(Tab::plain());
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if self.active > index {
            self.active -= 1;
        }
    }

    /// The number of characters in the active document.
    pub fn char_count(&self) -> usize {
        self.active_tab()
            .map(|tab| tab.text().chars().count())
            .unwrap_or(0)
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_the_last_tab_leaves_a_fresh_document() {
        let mut shell = Shell::new();
        shell.close_tab(0);
        assert_eq!(shell.tabs.len(), 1);
        assert_eq!(shell.active, 0);
        assert!(shell.tabs[0].text().is_empty());
    }

    #[test]
    fn active_follows_removal() {
        let mut shell = Shell::new();
        shell.open_tab(DocKind::Markdown);
        shell.open_tab(DocKind::Plain);
        assert_eq!(shell.active, 2);
        shell.close_tab(0);
        assert_eq!(shell.active, 1);
    }

    #[test]
    fn markdown_documents_enable_markdown_commands() {
        let mut shell = Shell::new();
        shell.open_tab(DocKind::Markdown);
        assert!(shell.availability().markdown_enabled);
    }

    #[test]
    fn tabs_recognize_two_spellings_of_one_file() {
        let directory =
            std::env::temp_dir().join(format!("notecraft-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let path = directory.join("note.txt");
        std::fs::write(&path, b"hello").expect("the file");

        let tab = Tab::open(&path).expect("the file opens");
        assert!(tab.matches_path(&path), "the same path matches");
        // A path through the containing folder is the same file.
        assert!(tab.matches_path(&directory.join(".").join("note.txt")));
        assert!(!tab.matches_path(&directory.join("other.txt")));

        // A new, untitled tab matches no path.
        assert!(!Tab::plain().matches_path(&path));

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A scratch directory, removed again by the caller.
    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("notecraft-recent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        directory
    }

    /// The file names of `paths`, so a test can read the list the way the menu
    /// shows it without depending on how each path was spelled on the way in.
    fn names(paths: &[PathBuf]) -> Vec<std::ffi::OsString> {
        paths
            .iter()
            .map(|path| path.file_name().expect("a named file").to_os_string())
            .collect()
    }

    /// One entry of a [`names`] comparison, spelled as the bare file name.
    fn name(text: &str) -> std::ffi::OsString {
        std::ffi::OsString::from(text)
    }

    #[test]
    fn recent_files_lead_with_the_most_recent() {
        let directory = scratch("order");
        let mut shell = Shell::new();
        shell.remember_recent(&directory.join("first.txt"));
        shell.remember_recent(&directory.join("second.txt"));

        assert_eq!(
            names(&shell.recent_files),
            vec![name("second.txt"), name("first.txt")],
            "the newest entry leads, because that is what the menu lists first"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn reopening_a_file_moves_it_to_the_front_instead_of_duplicating_it() {
        let directory = scratch("dedupe");
        let one = directory.join("one.txt");
        let two = directory.join("two.txt");
        std::fs::write(&one, b"one").expect("a file");
        std::fs::write(&two, b"two").expect("a file");

        let mut shell = Shell::new();
        shell.remember_recent(&one);
        shell.remember_recent(&two);
        shell.remember_recent(&one);
        assert_eq!(
            names(&shell.recent_files),
            vec![name("one.txt"), name("two.txt")]
        );

        // A second spelling of one file is the same entry, by the same identity
        // rule a tab uses (`NC-F012`).
        shell.remember_recent(&directory.join(".").join("two.txt"));
        assert_eq!(
            names(&shell.recent_files),
            vec![name("two.txt"), name("one.txt")],
            "the file moves to the front rather than appearing twice"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn recent_files_keep_the_newest_and_drop_the_oldest() {
        let directory = scratch("limit");
        let mut shell = Shell::new();
        for index in 0..=RECENT_LIMIT {
            shell.remember_recent(&directory.join(format!("note-{index}.txt")));
        }

        assert_eq!(shell.recent_files.len(), RECENT_LIMIT);
        assert_eq!(
            names(&shell.recent_files).first().cloned(),
            Some(name(&format!("note-{RECENT_LIMIT}.txt"))),
            "the newest entry survives"
        );
        assert!(
            names(&shell.recent_files)
                .iter()
                .all(|name| name != std::ffi::OsStr::new("note-0.txt")),
            "the oldest entry is the one dropped"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn opening_a_file_records_it_as_recent_and_reuses_its_tab() {
        let directory = scratch("open");
        let path = directory.join("note.txt");
        std::fs::write(&path, b"hello").expect("the file");

        let mut shell = Shell::new();
        crate::ui::actions::open_path(&mut shell, &path, None);

        assert_eq!(names(&shell.recent_files), vec![name("note.txt")]);
        assert_eq!(shell.active, 1, "the opened file is the tab shown");
        assert!(shell.tabs[1].matches_path(&path), "and it is the file");

        // Reopening it focuses that tab rather than opening the file again, and
        // leaves one Recent entry rather than adding a second.
        crate::ui::actions::open_path(&mut shell, &path, None);
        assert_eq!(shell.tabs.len(), 2, "the file is not opened twice");
        assert_eq!(shell.active, 1);
        assert_eq!(shell.recent_files.len(), 1);

        let _ = std::fs::remove_dir_all(&directory);
    }
}
