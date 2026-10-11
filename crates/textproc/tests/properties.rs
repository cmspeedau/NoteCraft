//! Properties that must hold for every file and every edit.
//!
//! The tests in `file_io/` name particular files; these say what has to be true
//! of *any* file, over inputs no one would think to write down by hand. The
//! property that matters most is the one the whole stage is about: whatever
//! bytes go in, opening and saving an untouched file gives those same bytes
//! back — and the text the reader produces encodes back to exactly the bytes it
//! was read from. That second one is the "reading never substitutes" rule from
//! `crates/textproc/src/lib.rs` stated as something checkable.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use nc_textproc::encoding::encode;
use nc_textproc::{TextFile, interpret};
use proptest::prelude::*;

static CASE: AtomicUsize = AtomicUsize::new(0);

/// Text without terminators, so that what comes back out of a save is compared
/// with what went in rather than with a normalised line-ending style. The
/// byte-order mark is left out too: it is the encoding's marker, not text, and
/// a file whose only content is a BOM reads back as an empty one.
fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..64).prop_map(|characters| {
        characters
            .into_iter()
            .filter(|character| !matches!(*character, '\r' | '\n' | '\u{feff}'))
            .collect()
    })
}

/// A scratch directory of its own, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("notecraft-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        Self(directory)
    }

    /// A path of its own inside this directory, so cases cannot collide.
    fn file(&self) -> PathBuf {
        self.0
            .join(format!("case-{}.txt", CASE.fetch_add(1, Ordering::Relaxed)))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes `bytes` to a scratch file and opens it.
fn opened(path: &Path, bytes: &[u8]) -> TextFile {
    std::fs::write(path, bytes).expect("a scratch file");
    TextFile::open(path).expect("the file opens")
}

proptest! {
    /// The reader never substitutes: whatever it read, it writes back.
    #[test]
    fn reading_any_bytes_gives_text_that_encodes_to_those_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        let read = interpret(&bytes);
        let written = encode(&read.text, read.detection.encoding, read.detection.bom)
            .expect("text read from bytes is writable as those bytes");
        prop_assert_eq!(written, bytes);
    }

    /// Opening and saving an untouched file is the identity.
    #[test]
    fn saving_an_untouched_file_gives_the_same_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..2048),
    ) {
        let scratch = Scratch::new("prop-roundtrip");
        let path = scratch.file();
        let mut file = opened(&path, &bytes);

        prop_assert!(!file.is_dirty());
        let saved = file
            .save()
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(saved.bytes, bytes.len());
        prop_assert_eq!(std::fs::read(&path).expect("the file"), bytes);
    }

    /// What is on screen is what a reopen shows, for any text at all.
    #[test]
    fn text_survives_a_save_and_a_reopen(value in text()) {
        let scratch = Scratch::new("prop-text");
        let path = scratch.file();

        let mut typed = TextFile::untitled();
        typed.insert_str(&value);
        let encoding = typed.encoding();
        typed
            .save_as(&path, encoding, false)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        let reopened = TextFile::open(&path).expect("the file opens");
        prop_assert_eq!(reopened.text(), value.as_str());
    }

    /// Undo puts the document back exactly as it was, however long the run of
    /// edits was.
    #[test]
    fn undoing_every_edit_gives_the_text_back(
        before in text(),
        edits in prop::collection::vec(text(), 1..16),
    ) {
        let mut file = TextFile::untitled();
        file.insert_str(&before);
        file.document_mut().mark_saved();
        prop_assert!(!file.is_dirty());

        let mut count = 0;
        for edit in &edits {
            if file.document_mut().insert_uncoalesced(edit) {
                count += 1;
            }
        }

        for _ in 0..count {
            prop_assert!(file.undo());
        }
        prop_assert_eq!(file.text(), before.as_str());
        // The flag is not part of this: an undo puts the text back but the
        // document is still one the user has touched (`file_io/dirty.rs`
        // pins that).
    }

    /// Saving never writes part of a document: either the file is the old bytes
    /// or it is the whole new text.
    #[test]
    fn a_save_leaves_no_partial_file(
        first in text(),
        second in text(),
    ) {
        let scratch = Scratch::new("prop-atomic");
        let path = scratch.file();

        let mut file = TextFile::untitled();
        file.insert_str(&first);
        let encoding = file.encoding();
        file.save_as(&path, encoding, false).expect("a save");

        file.select_all();
        file.delete_selection();
        file.insert_str(&second);
        file.save().expect("a save");

        let on_disk = std::fs::read(&path).expect("the file");
        prop_assert_eq!(on_disk, second.clone().into_bytes());
        // Nothing of the old text is left over, and no temporary file was left
        // behind next to it.
        let leftovers: Vec<_> = std::fs::read_dir(&scratch.0)
            .expect("the directory")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        prop_assert_eq!(leftovers, vec![path.file_name().expect("a name").to_owned()]);
    }
}
