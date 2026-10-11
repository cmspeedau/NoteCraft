//! The clipboard interface the document copies through.

/// A system clipboard, or a stand-in for one.
///
/// The engine never touches the platform clipboard directly: the Win32
/// clipboard (`CF_UNICODETEXT`) and the headless stand-in both implement this
/// trait, which is what lets Copy/Cut/Paste be tested off Windows
/// (`docs/architecture.md` §4).
pub trait Clipboard {
    /// The clipboard's current text, if it holds any.
    fn get_text(&self) -> Option<String>;

    /// Replaces the clipboard's contents.
    fn set_text(&mut self, text: String);
}

/// An in-memory clipboard.
///
/// Used by the tests and by the headless backend; on Windows the real
/// clipboard is behind the same trait.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryClipboard {
    text: Option<String>,
}

impl MemoryClipboard {
    /// An empty clipboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// The clipboard's contents, if any.
    pub fn contents(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Empties the clipboard.
    pub fn clear(&mut self) {
        self.text = None;
    }
}

impl Clipboard for MemoryClipboard {
    fn get_text(&self) -> Option<String> {
        self.text.clone()
    }

    fn set_text(&mut self, text: String) {
        self.text = Some(text);
    }
}
