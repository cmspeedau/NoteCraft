//! What can go wrong reading and writing a file.

use std::fmt;
use std::path::PathBuf;

use crate::encoding::Encoding;

/// Why a byte string could not be decoded as the encoding it claimed.
///
/// A failed decode is never repaired by substituting characters: the caller
/// falls back to ANSI, which can read any byte string. These errors are
/// therefore reports about evidence, not failures a user ever sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The bytes are not valid UTF-8.
    NotUtf8,
    /// A wide encoding ended in half a code unit.
    Truncated,
    /// A UTF-16 code unit was an unpaired surrogate.
    UnpairedSurrogate,
    /// A UTF-32 code unit was not a Unicode scalar value.
    NotAScalarValue,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NotUtf8 => "not valid UTF-8",
            Self::Truncated => "a wide encoding ended in half a code unit",
            Self::UnpairedSurrogate => "an unpaired surrogate",
            Self::NotAScalarValue => "not a Unicode scalar value",
        };
        f.write_str(text)
    }
}

impl std::error::Error for DecodeError {}

/// Characters that cannot be written in the encoding a document is saved in.
///
/// This is the ANSI case: every character outside the code page has no byte.
/// The behaviour is a design decision, so NoteCraft asks instead
/// of guessing: [`crate::encode`] reports the characters, and
/// [`crate::encode_lossy`] writes them as `?` once the user has said to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeError {
    /// The encoding that could not represent the characters.
    pub encoding: Encoding,
    /// The characters with no representation, in the order they appear.
    pub characters: Vec<char>,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /// How many characters the message names before it stops.
        const NAMED: usize = 8;

        write!(
            f,
            "{} character(s) cannot be written in {}: ",
            self.characters.len(),
            self.encoding
        )?;
        for character in self.characters.iter().take(NAMED) {
            write!(f, "{character}")?;
        }
        if self.characters.len() > NAMED {
            write!(f, "… and {} more", self.characters.len() - NAMED)?;
        }
        Ok(())
    }
}

impl std::error::Error for EncodeError {}

/// The error type of this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be read.
    #[error("could not read {path}")]
    Read {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying reason.
        #[source]
        source: std::io::Error,
    },
    /// The file could not be written, and is as it was.
    #[error("could not write {path}")]
    Write {
        /// The file that could not be written.
        path: PathBuf,
        /// The underlying reason.
        #[source]
        source: std::io::Error,
    },
    /// The text cannot be written in the encoding it would be saved in.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// The document has never been saved, so there is nowhere to save it to.
    /// Save As is opened here instead.
    #[error("the document has no path yet")]
    NoPath,
}

/// The result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;
