//! The text buffer: a `String` plus a line index, with edit primitives.
//!
//! Byte offsets into the buffer's text are the engine's canonical position
//! type. Every method that takes an offset clamps it to the nearest character
//! boundary, so no operation can panic on a stale offset or leave the buffer
//! holding invalid UTF-8.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// A line/column position within the buffer.
///
/// Both fields are zero-based; `column` counts *grapheme clusters*, not bytes.
/// The one-based form the reference shows in its status bar is a view concern
/// (`docs/reference-matrix.md` §6, milestone M5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct LineColumn {
    /// Zero-based line index.
    pub line: usize,
    /// Zero-based grapheme column within the line.
    pub column: usize,
}

impl LineColumn {
    /// The start of `line`.
    pub const fn at_line_start(line: usize) -> Self {
        Self { line, column: 0 }
    }
}

/// The editable text.
///
/// The buffer holds `\n` terminators only; the style a file was read in with
/// lives on the [`crate::Document`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
    line_starts: Vec<usize>,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self {
            text: String::new(),
            line_starts: vec![0],
        }
    }
}

impl TextBuffer {
    /// An empty buffer, which is one empty line.
    pub fn new() -> Self {
        Self::default()
    }

    /// A buffer over text that already uses `\n` terminators.
    pub fn from_normalized(text: &str) -> Self {
        Self {
            line_starts: Self::line_index(text),
            text: text.to_owned(),
        }
    }

    fn line_index(text: &str) -> Vec<usize> {
        let mut starts = Vec::with_capacity(1 + text.len() / 24);
        starts.push(0);
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(index + 1);
            }
        }
        starts
    }

    /// The buffer's text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The number of lines, which is at least one.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// The byte offset at which `line` starts.
    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.line_starts.get(line).copied()
    }

    /// The byte offset just past the last character of `line`, excluding the
    /// terminator.
    pub fn line_end(&self, line: usize) -> Option<usize> {
        self.line_start(line)?;
        Some(
            self.line_starts
                .get(line + 1)
                .map_or(self.text.len(), |&next| next - 1),
        )
    }

    /// `line`'s text, without its terminator.
    pub fn line(&self, line: usize) -> Option<&str> {
        let start = self.line_start(line)?;
        let end = self.line_end(line)?;
        self.text.get(start..end)
    }

    /// Every line's text, without terminators.
    pub fn lines(&self) -> std::str::Split<'_, char> {
        self.text.split('\n')
    }

    /// The text between two byte offsets, or `None` if the range is out of
    /// bounds or does not fall on character boundaries.
    pub fn slice(&self, range: Range<usize>) -> Option<&str> {
        if range.start > range.end || range.end > self.text.len() {
            return None;
        }
        if !self.text.is_char_boundary(range.start) || !self.text.is_char_boundary(range.end) {
            return None;
        }
        self.text.get(range)
    }

    /// The nearest character boundary at or before `byte`.
    pub fn clamp(&self, byte: usize) -> usize {
        let mut at = byte.min(self.text.len());
        while at > 0 && !self.text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }

    /// The line containing `byte`.
    pub fn line_of(&self, byte: usize) -> usize {
        let at = self.clamp(byte);
        self.line_starts
            .partition_point(|&start| start <= at)
            .saturating_sub(1)
    }

    /// The line/column position of `byte`.
    pub fn line_column(&self, byte: usize) -> LineColumn {
        let at = self.clamp(byte);
        let line = self.line_of(at);
        let start = self.line_starts.get(line).copied().unwrap_or(0);
        LineColumn {
            line,
            column: self.text[start..at].graphemes(true).count(),
        }
    }

    /// The byte offset of a line/column position, clamped to the line's end.
    pub fn byte_of_line_column(&self, position: LineColumn) -> usize {
        let Some(start) = self.line_start(position.line) else {
            return self.text.len();
        };
        let end = self.line_end(position.line).unwrap_or(self.text.len());
        let line = self.text.get(start..end).unwrap_or("");
        let mut at = start;
        for (column, grapheme) in line.graphemes(true).enumerate() {
            if column == position.column {
                break;
            }
            at += grapheme.len();
        }
        at
    }

    /// The number of grapheme clusters in `line`.
    pub fn grapheme_count(&self, line: usize) -> usize {
        self.line(line)
            .map_or(0, |text| text.graphemes(true).count())
    }

    /// The boundary one grapheme to the right of `byte`, saturating at the
    /// end of the buffer.
    pub fn next_grapheme(&self, byte: usize) -> usize {
        let at = self.clamp(byte);
        match self.text[at..].graphemes(true).next() {
            Some(grapheme) => at + grapheme.len(),
            None => at,
        }
    }

    /// The boundary one grapheme to the left of `byte`, saturating at the
    /// start of the buffer. Stepping left over a `\n` deletes the line break
    /// rather than the previous line's last character, which is what makes
    /// Backspace at the start of a line join it to the line above.
    pub fn prev_grapheme(&self, byte: usize) -> usize {
        let at = self.clamp(byte);
        match self.text[..at].graphemes(true).next_back() {
            Some(grapheme) => at - grapheme.len(),
            None => at,
        }
    }

    /// The start of the word at or before `byte`.
    ///
    /// Word boundaries follow UAX #29 (`unicode-segmentation`'s
    /// `split_word_bounds`), with whitespace skipped, which is the closest
    /// published match to a Windows edit control. The reference's exact word
    /// definition is not captured, so this is an approximation recorded in
    /// `docs/feature-matrix.md` NC-E004.
    pub fn prev_word_bound(&self, byte: usize) -> usize {
        let at = self.clamp(byte);
        if at == 0 {
            return 0;
        }
        let bounds = self.word_boundaries();
        let mut index = bounds.partition_point(|&bound| bound < at);
        while index > 0 {
            let candidate = bounds[index - 1];
            let end = bounds.get(index).copied().unwrap_or(self.text.len());
            let segment = self.text.get(candidate..end).unwrap_or("");
            if candidate > 0 && segment.chars().all(char::is_whitespace) {
                index -= 1;
            } else {
                return candidate;
            }
        }
        0
    }

    /// The end of the next word at or after `byte`.
    ///
    /// See [`TextBuffer::prev_word_bound`] for the approximation this makes.
    pub fn next_word_bound(&self, byte: usize) -> usize {
        let at = self.clamp(byte);
        if at >= self.text.len() {
            return self.text.len();
        }
        let bounds = self.word_boundaries();
        let mut index = bounds.partition_point(|&bound| bound <= at);
        while index < bounds.len() {
            let candidate = bounds[index];
            let previous = if index == 0 { 0 } else { bounds[index - 1] };
            if candidate >= self.text.len() {
                return self.text.len();
            }
            let segment = self.text.get(previous..candidate).unwrap_or("");
            if segment.chars().all(char::is_whitespace) {
                index += 1;
            } else {
                return candidate;
            }
        }
        self.text.len()
    }

    fn word_boundaries(&self) -> Vec<usize> {
        let mut bounds: Vec<usize> = self
            .text
            .split_word_bound_indices()
            .map(|(index, _)| index)
            .collect();
        bounds.push(self.text.len());
        bounds.sort_unstable();
        bounds.dedup();
        bounds
    }

    /// Replaces `range` with `with`, returning the text that was removed.
    ///
    /// The line index is rebuilt after every edit. That is deliberate for now:
    /// an incremental index is a later optimisation, and a correct one is
    /// worth more than an unmeasured one (`docs/architecture.md` §4).
    pub fn replace(&mut self, range: Range<usize>, with: &str) -> String {
        let start = self.clamp(range.start);
        let end = self.clamp(range.end.max(start));
        let removed = self.text.get(start..end).unwrap_or("").to_owned();
        self.text.replace_range(start..end, with);
        self.line_starts = Self::line_index(&self.text);
        removed
    }

    /// Inserts `text` at `byte`.
    pub fn insert(&mut self, byte: usize, text: &str) {
        let at = self.clamp(byte);
        self.replace(at..at, text);
    }

    /// Removes `range`, returning the text that was removed.
    pub fn remove(&mut self, range: Range<usize>) -> String {
        self.replace(range, "")
    }

    /// Removes everything, leaving one empty line.
    pub fn clear(&mut self) {
        self.text.clear();
        self.line_starts.clear();
        self.line_starts.push(0);
    }
}
