//! Session persistence (`NC-F011`).
//!
//! On exit the shell records the documents it has open — their paths, encodings,
//! line endings, and, for anything with unsaved changes, its text — under the
//! user's configuration directory. On the next start, with no files named on the
//! command line, those documents are read back, so a restart does not lose open
//! tabs or unsaved work.
//!
//! The store is one small text file, written by hand rather than through a
//! serialization crate: a versioned header, a handful of `key value` lines per
//! tab, and values escaped so a document's own newlines cannot break the line
//! structure. Two things are deliberately not kept — the undo history and the
//! caret — because the shell does not track them per tab; a restored document
//! starts with a fresh history and its caret at the start (recorded
//! limitations). Only the main window's session is written; extra windows are
//! transient.
//!
//! Loading is best-effort: a missing or unreadable session file is simply no
//! session, and a clean tab whose file can no longer be read is left out rather
//! than failing the whole start.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::str::Lines;

use nc_core::LineEnding;
use nc_textproc::{Encoding, TextFile};

use crate::ui::state::{DocKind, Shell, Tab};

/// The session file's format marker, which also fixes its version.
const VERSION: &str = "notecraft-session 1";

/// The name of the session file inside the configuration directory.
const FILE: &str = "session";

/// One open document, as the session file records it.
///
/// `pub(crate)` rather than private so the lifecycle acceptance tests can drive a
/// save and a restart through these exact parts, not a reimplementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TabState {
    /// Whether the document is plain text or Markdown.
    kind: DocKind,
    /// Where the document lives, or `None` for one that has never been saved.
    path: Option<PathBuf>,
    /// The encoding it is written in.
    encoding: Encoding,
    /// Whether it begins with a byte order mark.
    bom: bool,
    /// The line endings it is written with.
    line_ending: LineEnding,
    /// Whether it has unsaved changes.
    dirty: bool,
    /// The document's text, present only when it has unsaved changes.
    ///
    /// A clean document can be read back from its file, so only unsaved work
    /// needs to be stored in full.
    text: Option<String>,
}

/// Every open document and which one is active.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Session {
    /// The tabs, in strip order.
    tabs: Vec<TabState>,
    /// The index of the active tab.
    active: usize,
}

/// The shell to start with when no files are named on the command line: the
/// previous session's documents, or one empty document if there is no session.
pub fn restore_shell() -> Shell {
    match session_path().and_then(|path| load_from(&path)) {
        Some(session) => session.into_shell(),
        None => Shell::new(),
    }
}

/// Records `shell` under the configuration directory.
///
/// Failure is returned rather than ignored so callers can log it, but losing a
/// session file must never be fatal: the write is a convenience on top of the
/// unsaved-changes prompts, not a replacement for them.
pub fn save(shell: &Shell) -> io::Result<()> {
    match session_path() {
        Some(path) => save_to(&path, &Session::of(shell)),
        // Nowhere to write is not an error; the session simply is not kept.
        None => Ok(()),
    }
}

impl Session {
    /// The session a shell's current documents describe.
    pub(crate) fn of(shell: &Shell) -> Self {
        Self {
            tabs: shell.tabs.iter().map(TabState::of).collect(),
            active: shell.active,
        }
    }

    /// A shell holding these documents, or `Shell::new()` if none could be
    /// restored.
    pub(crate) fn into_shell(self) -> Shell {
        let tabs: Vec<Tab> = self
            .tabs
            .into_iter()
            .filter_map(TabState::into_tab)
            .collect();
        let mut shell = Shell::new();
        if tabs.is_empty() {
            return shell;
        }
        shell.tabs = tabs;
        shell.active = self.active.min(shell.tabs.len() - 1);
        // A restored document has been opened, so it belongs in `File ▸ Recent`.
        // The session records no timestamps, so the strip order stands in for the
        // order they were opened in.
        let opened: Vec<PathBuf> = shell
            .tabs
            .iter()
            .filter(|tab| tab.file.has_path())
            .map(|tab| tab.file.path().to_path_buf())
            .collect();
        for path in &opened {
            shell.remember_recent(path);
        }
        shell
    }

    /// Serializes the session to its file form.
    fn encode(&self) -> String {
        let mut out = String::new();
        out.push_str(VERSION);
        out.push('\n');
        out.push_str(&format!("active {}\n", self.active));
        out.push_str(&format!("tabs {}\n", self.tabs.len()));
        for tab in &self.tabs {
            out.push_str("tab\n");
            out.push_str("kind ");
            out.push_str(kind_token(tab.kind));
            out.push('\n');
            out.push_str("path ");
            if let Some(path) = &tab.path {
                out.push_str(&escape(&path.to_string_lossy()));
            }
            out.push('\n');
            out.push_str("encoding ");
            out.push_str(encoding_token(tab.encoding));
            out.push('\n');
            out.push_str(&format!("bom {}\n", u8::from(tab.bom)));
            out.push_str("eol ");
            out.push_str(eol_token(tab.line_ending));
            out.push('\n');
            out.push_str(&format!("dirty {}\n", u8::from(tab.dirty)));
            if let Some(text) = &tab.text {
                out.push_str("text ");
                out.push_str(&escape(text));
                out.push('\n');
            }
        }
        out
    }

    /// Reads a session back from its file form.
    fn decode(text: &str) -> Result<Self, SessionError> {
        let mut lines = text.lines().peekable();
        if lines.next() != Some(VERSION) {
            return Err(SessionError::Header);
        }

        let mut session = Self::default();
        while let Some(line) = lines.next() {
            if line == "tab" {
                session.tabs.push(TabState::decode(&mut lines)?);
            } else if let Some(value) = line.strip_prefix("active ") {
                session.active = value.parse().map_err(|_| SessionError::Line)?;
            } else if let Some(value) = line.strip_prefix("tabs ") {
                // The count is a convenience for readers; the tab blocks are
                // authoritative, so a stale count is not fatal.
                let _ = value.parse::<usize>().map_err(|_| SessionError::Line)?;
            } else if !line.is_empty() {
                return Err(SessionError::Line);
            }
        }
        Ok(session)
    }
}

impl TabState {
    /// The stored form of one shell tab.
    fn of(tab: &Tab) -> Self {
        let dirty = tab.is_dirty();
        let path = tab.file.has_path().then(|| tab.file.path().to_path_buf());
        // A path that does not survive the store's text form would restore to a
        // different file, so it is not kept; the tab is remembered as an
        // unsaved document instead, losing neither its text nor aiming a later
        // save at the wrong path.
        let path = path.filter(|path| round_trips(path));
        let unsaved = dirty || (tab.file.has_path() && path.is_none());
        Self {
            kind: tab.kind,
            path,
            encoding: tab.file.encoding(),
            bom: tab.file.has_bom(),
            line_ending: tab.file.line_ending(),
            dirty: unsaved,
            // Only unsaved work needs its text stored; a clean document is read
            // back from its file.
            text: unsaved.then(|| tab.text().to_owned()),
        }
    }

    /// The shell tab this describes, or `None` if a clean document's file can
    /// no longer be read.
    fn into_tab(self) -> Option<Tab> {
        if self.dirty {
            let file = TextFile::restore(
                self.path.unwrap_or_default(),
                self.encoding,
                self.bom,
                self.line_ending,
                self.text.as_deref().unwrap_or_default(),
                true,
            );
            return Some(Tab::restore(file, self.kind));
        }
        match self.path {
            Some(path) => Tab::open(path).ok(),
            None => Some(match self.kind {
                DocKind::Plain => Tab::plain(),
                DocKind::Markdown => Tab::markdown(),
            }),
        }
    }

    /// Reads one tab block: the field lines up to the next `tab`.
    fn decode(lines: &mut Peekable<Lines<'_>>) -> Result<Self, SessionError> {
        let mut kind = None;
        let mut path = None;
        let mut encoding = None;
        let mut bom = None;
        let mut line_ending = None;
        let mut dirty = None;
        let mut text = None;

        while let Some(&line) = lines.peek() {
            if line == "tab" {
                break;
            }
            let line = lines.next().expect("peeked a line");
            if let Some(value) = line.strip_prefix("kind ") {
                kind = Some(kind_from(value)?);
            } else if let Some(value) = line.strip_prefix("path ") {
                path = Some(if value.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(unescape(value)?))
                });
            } else if let Some(value) = line.strip_prefix("encoding ") {
                encoding = Some(encoding_from(value)?);
            } else if let Some(value) = line.strip_prefix("bom ") {
                bom = Some(parse_bool(value)?);
            } else if let Some(value) = line.strip_prefix("eol ") {
                line_ending = Some(eol_from(value)?);
            } else if let Some(value) = line.strip_prefix("dirty ") {
                dirty = Some(parse_bool(value)?);
            } else if let Some(value) = line.strip_prefix("text ") {
                text = Some(unescape(value)?);
            } else {
                return Err(SessionError::Line);
            }
        }

        Ok(Self {
            kind: kind.ok_or(SessionError::Line)?,
            path: path.ok_or(SessionError::Line)?,
            encoding: encoding.ok_or(SessionError::Line)?,
            bom: bom.ok_or(SessionError::Line)?,
            line_ending: line_ending.ok_or(SessionError::Line)?,
            dirty: dirty.ok_or(SessionError::Line)?,
            text,
        })
    }
}

/// Why a session file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionError {
    /// The file does not start with the expected format marker.
    Header,
    /// A line is not one the format defines.
    Line,
}

/// Writes `session` to `path`, through a sibling temporary file so an
/// interrupted write cannot leave a half-written session behind.
pub(crate) fn save_to(path: &Path, session: &Session) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, session.encode())?;
    fs::rename(&temporary, path)
}

/// Reads the session at `path`, or `None` if it is missing, unreadable, or in a
/// form this build does not understand.
pub(crate) fn load_from(path: &Path) -> Option<Session> {
    let text = fs::read_to_string(path).ok()?;
    Session::decode(&text).ok()
}

/// The session file, or `None` when there is no configuration directory to put
/// it in.
fn session_path() -> Option<PathBuf> {
    config_dir().map(|directory| directory.join(FILE))
}

/// The directory NoteCraft keeps its session in.
///
/// `NOTECRAFT_CONFIG_DIR` overrides it, which is what the tests use; otherwise
/// the platform's usual per-user configuration directory, and nothing when none
/// can be found.
fn config_dir() -> Option<PathBuf> {
    if let Some(directory) = non_empty(std::env::var_os("NOTECRAFT_CONFIG_DIR")) {
        return Some(PathBuf::from(directory));
    }
    if cfg!(windows) {
        if let Some(appdata) = non_empty(std::env::var_os("APPDATA")) {
            return Some(PathBuf::from(appdata).join("NoteCraft"));
        }
    }
    if let Some(xdg) = non_empty(std::env::var_os("XDG_CONFIG_HOME")) {
        return Some(PathBuf::from(xdg).join("notecraft"));
    }
    non_empty(std::env::var_os("HOME"))
        .map(|home| PathBuf::from(home).join(".config").join("notecraft"))
}

/// The value, or `None` when it is unset or empty.
fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

/// Whether `path` can be stored as text and read back as the very same path.
///
/// [`Path::to_string_lossy`] replaces bytes that are not UTF-8 with U+FFFD, so
/// on a filesystem that allows such a path the round trip would name a
/// different file. Rebuilding the path from the text and comparing catches that.
fn round_trips(path: &Path) -> bool {
    PathBuf::from(path.to_string_lossy().into_owned()) == path
}

/// Escapes a value so it fits on one line, whatever characters it holds.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            _ => out.push(character),
        }
    }
    out
}

/// Reverses [`escape`].
fn unescape(value: &str) -> Result<String, SessionError> {
    let mut out = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            _ => return Err(SessionError::Line),
        }
    }
    Ok(out)
}

/// `0`/`1` as a boolean.
fn parse_bool(value: &str) -> Result<bool, SessionError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(SessionError::Line),
    }
}

/// The stored name of a document kind.
fn kind_token(kind: DocKind) -> &'static str {
    match kind {
        DocKind::Plain => "plain",
        DocKind::Markdown => "markdown",
    }
}

/// Reads a document kind back.
fn kind_from(value: &str) -> Result<DocKind, SessionError> {
    match value {
        "plain" => Ok(DocKind::Plain),
        "markdown" => Ok(DocKind::Markdown),
        _ => Err(SessionError::Line),
    }
}

/// The stored name of an encoding.
fn encoding_token(encoding: Encoding) -> &'static str {
    match encoding {
        Encoding::Ansi => "ansi",
        Encoding::Utf8 => "utf8",
        Encoding::Utf16Le => "utf16le",
        Encoding::Utf16Be => "utf16be",
        Encoding::Utf32Le => "utf32le",
        Encoding::Utf32Be => "utf32be",
    }
}

/// Reads an encoding back.
fn encoding_from(value: &str) -> Result<Encoding, SessionError> {
    match value {
        "ansi" => Ok(Encoding::Ansi),
        "utf8" => Ok(Encoding::Utf8),
        "utf16le" => Ok(Encoding::Utf16Le),
        "utf16be" => Ok(Encoding::Utf16Be),
        "utf32le" => Ok(Encoding::Utf32Le),
        "utf32be" => Ok(Encoding::Utf32Be),
        _ => Err(SessionError::Line),
    }
}

/// The stored name of a line-ending style.
fn eol_token(line_ending: LineEnding) -> &'static str {
    match line_ending {
        LineEnding::CrLf => "crlf",
        LineEnding::Lf => "lf",
        LineEnding::Cr => "cr",
    }
}

/// Reads a line-ending style back.
fn eol_from(value: &str) -> Result<LineEnding, SessionError> {
    match value {
        "crlf" => Ok(LineEnding::CrLf),
        "lf" => Ok(LineEnding::Lf),
        "cr" => Ok(LineEnding::Cr),
        _ => Err(SessionError::Line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_textproc::Encoding;

    /// A scratch directory of its own, holding a session file.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let directory = std::env::temp_dir()
                .join(format!("notecraft-session-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("a scratch directory");
            Self(directory)
        }

        fn session(&self) -> PathBuf {
            self.0.join(FILE)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn escaping_round_trips_every_awkward_character() {
        for value in [
            "",
            "plain",
            "back\\slash",
            "line\nbreak\r\n",
            "tab\tand\u{0}nul",
            "unicode: héllo 😀",
        ] {
            assert_eq!(unescape(&escape(value)).unwrap(), value, "{value:?}");
        }
        assert!(!escape("a\nb").contains('\n'), "no real newline survives");
    }

    #[test]
    fn a_session_survives_a_round_trip() {
        let session = Session {
            active: 1,
            tabs: vec![
                TabState {
                    kind: DocKind::Plain,
                    path: None,
                    encoding: Encoding::Utf8,
                    bom: false,
                    line_ending: LineEnding::CrLf,
                    dirty: true,
                    text: Some("unsaved\nwork".to_owned()),
                },
                TabState {
                    kind: DocKind::Markdown,
                    path: Some(PathBuf::from("/notes/a b.md")),
                    encoding: Encoding::Utf16Le,
                    bom: true,
                    line_ending: LineEnding::Lf,
                    dirty: false,
                    text: None,
                },
            ],
        };

        let decoded = Session::decode(&session.encode()).expect("the session decodes");
        assert_eq!(decoded, session);
    }

    #[test]
    fn an_unknown_header_is_rejected() {
        assert_eq!(
            Session::decode("notecraft-session 99\n").unwrap_err(),
            SessionError::Header
        );
        assert_eq!(Session::decode("").unwrap_err(), SessionError::Header);
    }

    #[test]
    fn a_shell_round_trips_through_the_file() {
        let scratch = Scratch::new("round-trip");
        let path = scratch.session();

        let mut shell = Shell::new();
        shell.tabs[0]
            .document_mut()
            .insert_str("typed but not saved");
        assert!(shell.tabs[0].is_dirty());
        shell.open_tab(DocKind::Markdown);
        shell.tabs[1].document_mut().insert_str("# Title");
        shell.active = 1;

        save_to(&path, &Session::of(&shell)).expect("the session is written");
        let restored = load_from(&path).expect("the session is read").into_shell();

        assert_eq!(restored.tabs.len(), 2);
        assert_eq!(restored.active, 1);
        assert_eq!(restored.tabs[0].text(), "typed but not saved");
        assert!(restored.tabs[0].is_dirty(), "unsaved work is still unsaved");
        assert_eq!(restored.tabs[1].kind, DocKind::Markdown);
        assert_eq!(restored.tabs[1].text(), "# Title");
        assert!(restored.tabs[1].is_dirty());
    }

    #[test]
    fn a_clean_path_backed_tab_is_read_back_from_its_file() {
        let scratch = Scratch::new("clean");
        let file = scratch.0.join("note.txt");
        std::fs::write(&file, b"on disk\r\n").expect("the file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&file).expect("the file opens");
        assert!(!shell.tabs[0].is_dirty());

        save_to(&scratch.session(), &Session::of(&shell)).expect("written");
        let restored = load_from(&scratch.session()).unwrap().into_shell();

        assert_eq!(restored.tabs.len(), 1);
        assert_eq!(restored.tabs[0].text(), "on disk\n");
        assert!(!restored.tabs[0].is_dirty());
        assert_eq!(restored.tabs[0].file.line_ending(), LineEnding::CrLf);
    }

    #[test]
    fn a_session_with_no_readable_documents_falls_back_to_one_empty_tab() {
        let scratch = Scratch::new("missing-path");
        let file = scratch.0.join("gone.txt");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&file).unwrap_or_else(|_| Tab::plain());
        // The file is named but does not exist, so the clean tab cannot be read
        // back and the session falls back to a fresh document.
        let session = Session {
            active: 0,
            tabs: vec![TabState {
                kind: DocKind::Plain,
                path: Some(file),
                encoding: Encoding::Utf8,
                bom: false,
                line_ending: LineEnding::CrLf,
                dirty: false,
                text: None,
            }],
        };
        save_to(&scratch.session(), &session).expect("written");

        let restored = load_from(&scratch.session()).unwrap().into_shell();
        assert_eq!(restored.tabs.len(), 1);
        assert!(restored.tabs[0].text().is_empty());
        assert_eq!(restored.active, 0);
    }

    #[test]
    fn a_missing_session_file_is_no_session() {
        let scratch = Scratch::new("absent");
        assert!(load_from(&scratch.session()).is_none());
    }

    #[test]
    fn a_restored_session_lists_its_documents_as_recent() {
        let scratch = Scratch::new("recent");
        let file = scratch.0.join("note.txt");
        std::fs::write(&file, b"on disk\r\n").expect("the file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&file).expect("the file opens");

        save_to(&scratch.session(), &Session::of(&shell)).expect("written");
        let restored = load_from(&scratch.session()).unwrap().into_shell();

        assert_eq!(
            restored.recent_files,
            vec![file],
            "a restored document is one the user has open, so Recent lists it"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_cannot_be_stored_becomes_an_unsaved_document() {
        use std::os::unix::ffi::OsStrExt;

        let scratch = Scratch::new("lossy-path");
        let file = scratch.0.join(std::ffi::OsStr::from_bytes(b"bad\xFF.txt"));
        std::fs::write(&file, b"content").expect("the file");

        let mut shell = Shell::new();
        shell.tabs[0] = Tab::open(&file).expect("the file opens");
        assert!(!shell.tabs[0].is_dirty());

        save_to(&scratch.session(), &Session::of(&shell)).expect("written");
        let restored = load_from(&scratch.session()).unwrap().into_shell();

        assert_eq!(restored.tabs.len(), 1);
        assert_eq!(restored.tabs[0].text(), "content");
        assert!(!restored.tabs[0].file.has_path(), "the path is not kept");
        assert!(
            restored.tabs[0].is_dirty(),
            "it is remembered as an unsaved document"
        );
    }

    #[test]
    fn the_active_index_is_clamped_to_the_restored_tabs() {
        let session = Session {
            active: 9,
            tabs: vec![TabState {
                kind: DocKind::Markdown,
                path: None,
                encoding: Encoding::Utf8,
                bom: false,
                line_ending: LineEnding::CrLf,
                dirty: false,
                text: None,
            }],
        };
        let shell = session.into_shell();
        assert_eq!(shell.tabs.len(), 1);
        assert_eq!(shell.active, 0);
        assert_eq!(shell.tabs[0].kind, DocKind::Markdown);
    }
}
