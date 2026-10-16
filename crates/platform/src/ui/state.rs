//! The shell's observable state: open tabs, the active document, and the view
//! options the chrome reflects.

use nc_commands::AvailabilityContext;

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

/// A single open document.
pub struct Tab {
    /// The name shown on the tab and in the title bar.
    pub title: String,
    /// The document text.
    pub text: String,
    /// Whether the document has unsaved changes.
    pub dirty: bool,
    /// Whether the document is plain text or Markdown.
    pub kind: DocKind,
    /// The encoding the status bar reports.
    pub encoding: &'static str,
    /// The line-ending style the status bar reports.
    pub line_ending: &'static str,
    /// The paragraph style selected in the toolbar.
    pub paragraph: ParagraphStyle,
    /// Whether the paragraph style is being edited in formatted view.
    pub formatted: bool,
}

impl Tab {
    /// A new, empty plain-text document.
    pub fn plain() -> Self {
        Self {
            title: "Untitled".to_owned(),
            text: String::new(),
            dirty: false,
            kind: DocKind::Plain,
            encoding: "UTF-8",
            line_ending: "Windows (CRLF)",
            paragraph: ParagraphStyle::Body,
            formatted: false,
        }
    }

    /// A new, empty Markdown document.
    pub fn markdown() -> Self {
        Self {
            title: "Untitled".to_owned(),
            text: String::new(),
            dirty: false,
            kind: DocKind::Markdown,
            encoding: "UTF-8",
            line_ending: "Windows (CRLF)",
            paragraph: ParagraphStyle::Body,
            formatted: true,
        }
    }

    /// A document with the given title and text (used when opening a file).
    pub fn from_file(title: String, text: String) -> Self {
        let markdown = title.to_ascii_lowercase().ends_with(".md")
            || title.to_ascii_lowercase().ends_with(".markdown");
        Self {
            kind: if markdown {
                DocKind::Markdown
            } else {
                DocKind::Plain
            },
            title,
            text,
            formatted: markdown,
            ..Tab::plain()
        }
    }

    /// The label the status bar shows for the document kind.
    pub const fn kind_label(&self) -> &'static str {
        match self.kind {
            DocKind::Plain => "Plain text",
            DocKind::Markdown => "Markdown",
        }
    }
}

/// The editor font families offered in Settings.
pub const FONT_FAMILIES: [&str; 4] = ["Consolas", "Cascadia Mono", "Segoe UI", "Courier New"];

/// The whole shell state.
pub struct Shell {
    /// The open documents, in tab order.
    pub tabs: Vec<Tab>,
    /// The index of the active tab.
    pub active: usize,
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
}

impl Shell {
    /// The shell at first launch: one empty, clean plain-text document.
    pub fn new() -> Self {
        Self {
            tabs: vec![Tab::plain()],
            active: 0,
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
            dirty: tab.map(|tab| tab.dirty).unwrap_or(false),
            has_selection: false,
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
            .map(|tab| tab.text.chars().count())
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
        assert!(shell.tabs[0].text.is_empty());
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
}
