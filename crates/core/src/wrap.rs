//! Soft wrap.
//!
//! When word wrap is on, a logical line is laid out as one or more *wrapped*
//! lines. The layout is a pure function of the line, the viewport width, and
//! the tab-stop width, so it can be tested without drawing anything — the
//! viewport that consumes it arrives in milestone M2/M5.
//!
//! The reference's own wrap points are not yet captured in detail, so the
//! break rule here is stated plainly rather than presented as parity: break at
//! the last whitespace that fits, and if a single run of non-whitespace is
//! wider than the viewport, break inside it rather than overflow
//! (`docs/feature-matrix.md` NC-E006).

use unicode_segmentation::UnicodeSegmentation;

use crate::buffer::TextBuffer;
use crate::tabs::TabStop;

/// A run of one logical line as it appears on one screen line.
///
/// Offsets are byte offsets into the buffer, with `start` inclusive and `end`
/// exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrappedLine {
    /// Offset of the first byte on this screen line.
    pub start: usize,
    /// Offset just past the last byte on this screen line.
    pub end: usize,
}

impl WrappedLine {
    /// The number of bytes on this screen line.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Whether the screen line is empty.
    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }
}

/// Lays out one logical line, whose text begins at `offset` in the buffer.
pub fn wrap_line(line: &str, offset: usize, columns: usize, tab_stop: TabStop) -> Vec<WrappedLine> {
    if line.is_empty() {
        return vec![WrappedLine {
            start: offset,
            end: offset,
        }];
    }
    let columns = columns.max(1);
    // The line as graphemes: byte offset, byte length, and whether the grapheme
    // is whitespace. Working in graphemes rather than bytes keeps a multi-byte
    // character or a combining sequence on one screen line.
    let graphemes: Vec<(usize, usize, bool)> = line
        .grapheme_indices(true)
        .map(|(byte, grapheme)| {
            (
                byte,
                grapheme.len(),
                grapheme.chars().all(char::is_whitespace),
            )
        })
        .collect();

    let mut out = Vec::new();
    let mut start = 0usize;
    while start < graphemes.len() {
        // Lay out one screen line: take graphemes while they fit, remembering
        // the last whitespace boundary seen, so a break can be placed there.
        let mut column = 0usize;
        let mut index = start;
        let mut last_break: Option<usize> = None;
        while index < graphemes.len() {
            let (byte, len, whitespace) = graphemes[index];
            let grapheme = line.get(byte..byte + len).unwrap_or("");
            let after = if grapheme == "\t" {
                tab_stop.advance(column)
            } else {
                column + 1
            };
            if after > columns && index > start {
                break;
            }
            column = after;
            index += 1;
            if whitespace {
                last_break = Some(index);
            }
        }
        // `index` is one grapheme past the last one that fits. When the line
        // continues, break at the last whitespace that fits; when the run
        // itself is wider than the viewport there is none, so break inside it
        // rather than overflow.
        let end = if index < graphemes.len() {
            last_break.unwrap_or(index)
        } else {
            index
        };
        let end_byte = graphemes.get(end).map_or(line.len(), |&(byte, _, _)| byte);
        out.push(WrappedLine {
            start: offset + graphemes[start].0,
            end: offset + end_byte,
        });
        start = end;
    }
    out
}

/// Lays out every line of `buffer`.
pub fn wrap_document(buffer: &TextBuffer, columns: usize, tab_stop: TabStop) -> Vec<WrappedLine> {
    let mut out = Vec::new();
    for line in 0..buffer.line_count() {
        let start = buffer.line_start(line).unwrap_or(0);
        let text = buffer.line(line).unwrap_or("");
        out.extend(wrap_line(text, start, columns, tab_stop));
    }
    out
}
