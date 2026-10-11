//! Tab stops and display-width arithmetic.
//!
//! The reference's tab-stop width and its Tab-key behaviour are not yet
//! captured (`docs/reference-matrix.md` §5), so the engine exposes the width as
//! a parameter with the conventional default of 8 columns and keeps the
//! arithmetic separate and testable (`docs/feature-matrix.md` NC-E007).

/// The distance between tab stops, in display columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabStop(u8);

impl TabStop {
    /// The default tab-stop width, in columns.
    pub const DEFAULT: Self = Self::new(8);

    /// A tab-stop width of `width` columns. Zero is treated as one column so
    /// the arithmetic can never loop.
    pub const fn new(width: u8) -> Self {
        Self(if width == 0 { 1 } else { width })
    }

    /// The width in columns.
    pub const fn width(self) -> u8 {
        self.0
    }

    /// The next tab stop strictly after `column`.
    pub const fn advance(self, column: usize) -> usize {
        let width = self.0 as usize;
        (column / width + 1) * width
    }
}

impl Default for TabStop {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The display width of `text`, with a tab advancing to the next tab stop.
///
/// Width is counted in grapheme clusters, so a combining sequence or an
/// emoji ZWJ sequence advances by one column. Characters that are wide in the
/// reference's font (CJK, for instance) are counted as one column too: the
/// reference's exact handling of them is a capture item, so the approximation
/// is named here rather than presented as parity.
pub fn display_width(text: &str, tab_stop: TabStop, start_column: usize) -> usize {
    use unicode_segmentation::UnicodeSegmentation;
    let mut column = start_column;
    for grapheme in text.graphemes(true) {
        if grapheme == "\t" {
            column = tab_stop.advance(column);
        } else {
            column += 1;
        }
    }
    column.saturating_sub(start_column)
}
