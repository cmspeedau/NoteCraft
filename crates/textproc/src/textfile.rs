//! One open file: the path it came from, the bytes it holds, and the document
//! those bytes say.
//!
//! [`TextFile`] is the whole of file I/O. It keeps three things in step: the
//! path, the bytes as they are on disk, and the [`Document`] the text was read
//! into. The encoding a file was read as is remembered and reused on save, so
//! ordinary save never changes how a file is written unless the user asks
//! (`docs/ui-reference.md`, NC-D004).

use std::ffi::OsString;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use nc_core::Document;

use crate::encoding::{self, Detection, Encoding, Interpreted};
use crate::error::{Error, Result};

/// Whether the file on disk still holds what NoteCraft last read or wrote.
///
/// Whether an outside change is *detected* at all is a design decision
/// (`docs/ui-reference.md`), so NoteCraft answers the question and
/// leaves the policy — prompt, reload, or ignore — to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalChange {
    /// The file is as it was when it was last read or written.
    None,
    /// The file now holds different bytes, or a file that did not exist does.
    Modified,
    /// The file was there and is gone.
    Removed,
}

/// What a save wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// The path that was written.
    pub path: PathBuf,
    /// How many bytes were written.
    pub bytes: usize,
}

/// One open file.
#[derive(Debug)]
pub struct TextFile {
    /// Where the file lives, or empty for a document that has never been saved.
    path: PathBuf,
    /// How the bytes were read, or how the next save will write them.
    detection: Detection,
    /// The bytes last read from or written to the disk.
    ///
    /// `None` means there was no file: a document that has never been saved, or
    /// one whose file has since been removed.
    on_disk: Option<Vec<u8>>,
    /// The document the text was read into.
    document: Document,
}

impl TextFile {
    /// A new, empty document with no file behind it yet.
    ///
    /// It is written as UTF-8 without a byte order mark (`docs/ui-reference.md`);
    /// the
    /// text it writes uses CRLF, which is `nc-core`'s rule for a new document.
    pub fn untitled() -> Self {
        Self {
            path: PathBuf::new(),
            detection: Detection::new_document(),
            on_disk: None,
            document: Document::new(),
        }
    }

    /// Reads `path` and opens it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self::from_bytes(path, bytes))
    }

    /// Reads `path` as `encoding`, rather than detecting it
    /// (`docs/ui-reference.md` §7: the Open dialog's encoding list).
    ///
    /// `bom` is remembered, so saving reproduces the file's mark. Unlike
    /// [`open`](Self::open) this can fail: bytes that are not text in the
    /// chosen encoding are reported rather than silently read as ANSI.
    pub fn open_as(path: impl AsRef<Path>, encoding: Encoding, bom: bool) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let text = encoding::decode(&bytes, encoding).map_err(|source| Error::Decode {
            path: path.to_path_buf(),
            encoding,
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            detection: Detection::chosen(encoding, bom),
            on_disk: Some(bytes),
            document: Document::from_text(&text),
        })
    }

    /// A file over `bytes` that were already read from `path`.
    ///
    /// This never fails: bytes that are not UTF-8 are read as ANSI, which can
    /// read any byte string, and the exact bytes are remembered so that saving
    /// an unedited file reproduces them byte for byte.
    pub fn from_bytes(path: impl AsRef<Path>, bytes: impl Into<Vec<u8>>) -> Self {
        let bytes = bytes.into();
        let Interpreted { detection, text } = encoding::interpret(&bytes);
        Self {
            path: path.as_ref().to_path_buf(),
            detection,
            on_disk: Some(bytes),
            document: Document::from_text(&text),
        }
    }

    // ---- what the file is -----------------------------------------------

    /// The path this file came from, or empty if it has never been saved.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the file has a path to be saved to.
    pub fn has_path(&self) -> bool {
        !self.path.as_os_str().is_empty()
    }

    /// How the bytes were read, or how the next save will write them.
    pub fn detection(&self) -> Detection {
        self.detection
    }

    /// The encoding the bytes are in.
    pub fn encoding(&self) -> Encoding {
        self.detection.encoding
    }

    /// The encoding's name, as the status bar shows it.
    pub fn encoding_label(&self) -> &'static str {
        self.detection.encoding.label()
    }

    /// Whether the file begins with a byte order mark.
    pub fn has_bom(&self) -> bool {
        self.detection.bom
    }

    /// The bytes as they are on disk, or `None` if there is no file.
    ///
    /// This is what the file held when it was read or last written, not what it
    /// holds now; ask [`external_change`](Self::external_change) about that.
    pub fn on_disk(&self) -> Option<&[u8]> {
        self.on_disk.as_deref()
    }

    /// The number of bytes the file holds, or zero if there is no file.
    pub fn len(&self) -> usize {
        self.on_disk.as_ref().map_or(0, Vec::len)
    }

    /// Whether the file is empty, which it is if it does not exist.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the file on disk has changed since NoteCraft read or wrote it.
    pub fn external_change(&self) -> ExternalChange {
        let missing = |error: &std::io::Error| error.kind() == std::io::ErrorKind::NotFound;
        match fs::read(&self.path) {
            Ok(bytes) => match &self.on_disk {
                Some(known) if *known == bytes => ExternalChange::None,
                _ => ExternalChange::Modified,
            },
            Err(error) if missing(&error) => match self.on_disk {
                // A document that has never been saved has no file, and no file
                // is not a change.
                None => ExternalChange::None,
                Some(_) => ExternalChange::Removed,
            },
            // A file that is there but unreadable is not what we wrote.
            Err(_) => ExternalChange::Modified,
        }
    }

    // ---- the document ---------------------------------------------------

    /// The document the text was read into.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The document, to edit in place.
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// The text, with `\n` terminators.
    pub fn text(&self) -> &str {
        self.document.text()
    }

    /// The number of lines, which is at least one.
    pub fn line_count(&self) -> usize {
        self.document.line_count()
    }

    /// The style the file is written in.
    pub fn line_ending(&self) -> nc_core::LineEnding {
        self.document.line_ending()
    }

    /// Whether the text read in used more than one line-ending style.
    pub fn has_mixed_line_endings(&self) -> bool {
        self.document.has_mixed_line_endings()
    }

    /// Whether the document has unsaved changes.
    ///
    /// What is then asked is a design decision; this crate only
    /// reports the state the question is asked about.
    pub fn is_dirty(&self) -> bool {
        self.document.is_dirty()
    }

    /// Whether there is an edit to undo.
    pub fn can_undo(&self) -> bool {
        self.document.can_undo()
    }

    /// Whether there is an undone edit to redo.
    pub fn can_redo(&self) -> bool {
        self.document.can_redo()
    }

    // ---- editing --------------------------------------------------------

    /// Inserts `text` at the caret, replacing the selection if there is one.
    pub fn insert_str(&mut self, text: &str) -> bool {
        self.document.insert_str(text)
    }

    /// Inserts a line ending at the caret.
    pub fn insert_newline(&mut self) -> bool {
        self.document.insert_newline()
    }

    /// Deletes the character before the caret.
    pub fn backspace(&mut self) -> bool {
        self.document.backspace()
    }

    /// Deletes the character after the caret.
    pub fn delete_forward(&mut self) -> bool {
        self.document.delete_forward()
    }

    /// Deletes the selection.
    pub fn delete_selection(&mut self) -> bool {
        self.document.delete_selection()
    }

    /// Selects everything.
    pub fn select_all(&mut self) {
        self.document.select_all();
    }

    /// Selects from `anchor` to `head`, both byte offsets.
    pub fn select(&mut self, anchor: usize, head: usize) {
        self.document.select(anchor, head);
    }

    /// Undoes the last edit.
    pub fn undo(&mut self) -> bool {
        self.document.undo()
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) -> bool {
        self.document.redo()
    }

    // ---- reading and writing --------------------------------------------

    /// Writes the document to its file, in the encoding it was read in.
    ///
    /// Fails if the document has no path yet, which is where Save As is opened
    /// instead.
    pub fn save(&mut self) -> Result<Saved> {
        let text = self.document.save_text();
        let bytes = encoding::encode(&text, self.encoding(), self.has_bom())?;
        self.write(&bytes)
    }

    /// Writes the document to `path` as `encoding`, and follows it there.
    ///
    /// The encoding and the mark are the ones the user picked; the line-ending
    /// style stays the document's own.
    pub fn save_as(
        &mut self,
        path: impl AsRef<Path>,
        encoding: Encoding,
        bom: bool,
    ) -> Result<Saved> {
        let path = path.as_ref();
        let text = self.document.save_text();
        let bytes = encoding::encode(&text, encoding, bom)?;
        let saved = write_atomic(path, &bytes)?;
        self.path = path.to_path_buf();
        self.detection = Detection::chosen(encoding, bom);
        self.write_bookkeeping(bytes);
        Ok(saved)
    }

    /// Writes the document as ANSI, replacing characters the code page cannot
    /// represent with `?`, and reports the characters that were replaced.
    ///
    /// This is the second half of the ANSI question: [`save`](Self::save) fails
    /// and names the characters, and this is what the user's answer calls.
    pub fn save_lossy(&mut self) -> Result<(Saved, Vec<char>)> {
        if !self.has_path() {
            return Err(Error::NoPath);
        }
        let text = self.document.save_text();
        let (bytes, characters) = encoding::encode_lossy(&text, self.encoding(), self.has_bom());
        let saved = self.write(&bytes)?;
        Ok((saved, characters))
    }

    /// Reads the file again, discarding the document and its history.
    ///
    /// This is what answering "reload" to an outside change does; the encoding
    /// is detected afresh, since the new bytes may be in another one.
    pub fn reload(&mut self) -> Result<()> {
        let bytes = fs::read(&self.path).map_err(|source| Error::Read {
            path: self.path.clone(),
            source,
        })?;
        let Interpreted { detection, text } = encoding::interpret(&bytes);
        self.detection = detection;
        self.on_disk = Some(bytes);
        self.document = Document::from_text(&text);
        Ok(())
    }

    /// Writes `bytes` to the file this document already has.
    fn write(&mut self, bytes: &[u8]) -> Result<Saved> {
        if !self.has_path() {
            return Err(Error::NoPath);
        }
        let saved = write_atomic(&self.path, bytes)?;
        self.write_bookkeeping(bytes.to_vec());
        Ok(saved)
    }

    /// Records a successful write: the bytes are what is on disk now.
    fn write_bookkeeping(&mut self, bytes: Vec<u8>) {
        self.on_disk = Some(bytes);
        self.document.mark_saved();
    }
}

impl Default for TextFile {
    fn default() -> Self {
        Self::untitled()
    }
}

/// Writes `bytes` to `path`, either wholly or not at all.
///
/// The bytes go to a sibling temporary file and are renamed over `path`, so an
/// interrupted write cannot leave a half-written file behind — and a file that
/// cannot be replaced (read-only, or a directory that is not there) is refused
/// before anything is written, leaving it exactly as it was.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<Saved> {
    let error = |source: std::io::Error| Error::Write {
        path: path.to_path_buf(),
        source,
    };

    if path.exists() {
        fs::OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(error)?;
    }

    let temporary = temporary_path(path);
    {
        let mut file = fs::File::create(&temporary).map_err(error)?;
        file.write_all(bytes).map_err(error)?;
        file.sync_all().map_err(error)?;
    }
    fs::rename(&temporary, path).map_err(|source| {
        let _ = fs::remove_file(&temporary);
        error(source)
    })?;

    Ok(Saved {
        path: path.to_path_buf(),
        bytes: bytes.len(),
    })
}

/// The sibling file a write goes through, next to the file it will replace.
fn temporary_path(path: &Path) -> PathBuf {
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut name = OsString::from(".");
    name.push(path.file_name().unwrap_or_else(|| "file".as_ref()));
    name.push(".notecraft");
    directory.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_temporary_file_is_a_sibling_with_a_hidden_name() {
        assert_eq!(
            temporary_path(Path::new("/notes/todo.txt")),
            PathBuf::from("/notes/.todo.txt.notecraft")
        );
        assert_eq!(
            temporary_path(Path::new("todo.txt")),
            PathBuf::from("./.todo.txt.notecraft")
        );
    }

    #[test]
    fn a_new_document_has_no_file_and_is_utf8_without_a_mark() {
        let mut file = TextFile::untitled();
        assert!(!file.has_path());
        assert_eq!(file.encoding(), Encoding::Utf8);
        assert!(!file.has_bom());
        assert!(!file.is_dirty());
        assert_eq!(
            file.save().unwrap_err().to_string(),
            "the document has no path yet"
        );
    }
}
