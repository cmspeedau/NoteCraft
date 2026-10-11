//! Line-ending styles, and how the engine keeps them intact.

/// A line-ending style.
///
/// Source: `docs/reference-matrix.md` §4 — a new document saved by the
/// reference uses CRLF, and an existing file keeps whatever style it had.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineEnding {
    /// CRLF (`\r\n`).
    #[default]
    CrLf,
    /// LF (`\n`).
    Lf,
    /// CR (`\r`).
    Cr,
}

impl LineEnding {
    /// The style used for a new document, and for a document whose original
    /// bytes contained no terminator at all.
    pub const DEFAULT: Self = Self::CrLf;

    /// The terminator as text.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CrLf => "\r\n",
            Self::Lf => "\n",
            Self::Cr => "\r",
        }
    }

    /// The terminator as bytes.
    pub const fn as_bytes(self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    /// The terminator length in bytes.
    pub const fn byte_len(self) -> usize {
        self.as_str().len()
    }
}

/// What a byte string's line endings look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineEndingReport {
    /// The style taken to be *the file's* style. This is the first style that
    /// occurs in the file, which is what later milestones will re-check
    /// against the reference when `mixed` is true.
    pub style: LineEnding,
    /// Number of CRLF terminators seen.
    pub crlf: usize,
    /// Number of bare LF terminators seen.
    pub lf: usize,
    /// Number of bare CR terminators seen.
    pub cr: usize,
    /// Whether more than one style occurs in the same file.
    pub mixed: bool,
}

impl LineEndingReport {
    /// A report for text with no terminator at all.
    pub const fn uniform(style: LineEnding) -> Self {
        Self {
            style,
            crlf: 0,
            lf: 0,
            cr: 0,
            mixed: false,
        }
    }

    /// Total number of terminators seen.
    pub const fn total(&self) -> usize {
        self.crlf + self.lf + self.cr
    }

    /// Whether the file contained at least one terminator.
    pub const fn any(&self) -> bool {
        self.total() > 0
    }
}

impl Default for LineEndingReport {
    fn default() -> Self {
        Self::uniform(LineEnding::DEFAULT)
    }
}

/// Classifies the line endings in `text`.
pub fn detect(text: &str) -> LineEndingReport {
    let bytes = text.as_bytes();
    let (mut crlf, mut lf, mut cr) = (0usize, 0usize, 0usize);
    let mut first: Option<LineEnding> = None;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                crlf += 1;
                first.get_or_insert(LineEnding::CrLf);
                index += 2;
            }
            b'\r' => {
                cr += 1;
                first.get_or_insert(LineEnding::Cr);
                index += 1;
            }
            b'\n' => {
                lf += 1;
                first.get_or_insert(LineEnding::Lf);
                index += 1;
            }
            _ => index += 1,
        }
    }
    let distinct = [crlf > 0, lf > 0, cr > 0]
        .iter()
        .filter(|seen| **seen)
        .count();
    LineEndingReport {
        style: first.unwrap_or(LineEnding::DEFAULT),
        crlf,
        lf,
        cr,
        mixed: distinct > 1,
    }
}

/// Rewrites every terminator in `text` as a single `\n`.
///
/// `\r\n` collapses to one terminator, not two, so a CRLF file and an LF file
/// with the same lines produce the same buffer text.
pub fn to_lf(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                out.push('\n');
                index += 2;
            }
            b'\r' => {
                out.push('\n');
                index += 1;
            }
            _ => {
                let rest = &text[index..];
                let ch = rest.chars().next().expect("index is on a char boundary");
                out.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    out
}

/// Writes `text` (which must use `\n` terminators) out in `style`.
pub fn from_lf(text: &str, style: LineEnding) -> String {
    match style {
        LineEnding::Lf => text.to_owned(),
        _ => {
            let terminator = style.as_str();
            let mut out = String::with_capacity(text.len() + 8);
            let mut rest = text;
            while let Some(position) = rest.find('\n') {
                out.push_str(&rest[..position]);
                out.push_str(terminator);
                rest = &rest[position + 1..];
            }
            out.push_str(rest);
            out
        }
    }
}
