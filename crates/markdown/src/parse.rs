//! A tolerant, loss-keeping read of Markdown source — `docs/architecture.md` §6.
//!
//! The parser describes the source; it never replaces it. Every [`Block`] carries
//! the exact bytes it was read from, so reconstructing a document from its blocks
//! is concatenation, and a construct the parser does not know — a footnote, raw
//! HTML, an unterminated fence — survives as ordinary text instead of being
//! tidied away (`NC-M011`).
//!
//! [`spans`] does the same for inline emphasis: a delimiter with no partner is
//! left as the character the user typed, and the returned spans never overlap or
//! cross a line boundary, which is what lets the formatted view highlight them
//! without guessing.

use crate::Range;
use crate::format::{heading_prefix, list_prefix, quote_prefix};

/// What a run of Markdown source turns out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// A line with nothing but whitespace on it.
    Blank,
    /// An ATX heading, at the level its `#` run shows.
    Heading(u8),
    /// A list item, indented by `indent` bytes and led by `marker`.
    ListItem {
        /// How many bytes of whitespace lead the marker.
        indent: usize,
        /// The marker that leads it.
        marker: ListMarker,
    },
    /// A `>` blockquote line.
    Quote,
    /// A row of a pipe table.
    TableRow,
    /// Ordinary text.
    Paragraph,
}

/// The marker that leads a list item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMarker {
    /// `-`
    Dash,
    /// `+`
    Plus,
    /// `*`
    Star,
    /// `1.`, `2.`, and so on, with the number as written.
    Number(u64),
}

/// A block of source, with the bytes it came from kept attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block<'a> {
    /// What the block turned out to be.
    pub kind: BlockKind,
    /// The exact bytes of source, newlines included, that this block was read
    /// from. Concatenating every block's source reproduces the document.
    pub source: &'a str,
    /// Where those bytes sit in the document.
    pub range: Range,
}

impl<'a> Block<'a> {
    /// Whether the block is a blank line.
    pub fn is_blank(&self) -> bool {
        self.kind == BlockKind::Blank
    }

    /// The block's text with its marker and indentation removed, which is what a
    /// rendered view shows. The bytes in [`Block::source`] are untouched.
    pub fn content(&self) -> &'a str {
        let body = trim_newlines(self.source);
        let prefix = match self.kind {
            BlockKind::Heading(_) => heading_prefix(self.line()).map(|(at, len, _)| (at, len)),
            BlockKind::ListItem { .. } => list_prefix(self.line()).map(|(at, len, _)| (at, len)),
            BlockKind::Quote => quote_prefix(self.line()),
            _ => None,
        };
        match prefix {
            // Offsets into a block's first line index into its body the same way,
            // whether or not the block runs on.
            Some((at, len)) => body.get(at + len..).unwrap_or(""),
            None => body,
        }
    }

    /// The block's first line without its trailing newline, which is the line a
    /// marker was read from.
    fn line(&self) -> &'a str {
        match self.source.find('\n') {
            Some(offset) => self.source.get(..offset).unwrap_or(""),
            None => self.source,
        }
    }
}

/// Reads `text` as a sequence of blocks whose sources tile it exactly.
///
/// Consecutive paragraph lines are grouped into one block, because that is the
/// unit a rendered view lays out; every other kind of line stands alone. Nothing
/// is reordered, merged across a blank line, or rewritten.
pub fn blocks(text: &str) -> Vec<Block<'_>> {
    let lines = line_kinds(text);
    let mut grouped: Vec<Block<'_>> = Vec::new();

    let mut index = 0usize;
    while index < lines.len() {
        let kind = lines[index].kind;
        let mut last = index;
        if kind == BlockKind::Paragraph {
            while last + 1 < lines.len() && lines[last + 1].kind == BlockKind::Paragraph {
                last += 1;
            }
        }

        let start = lines[index].start;
        // Take the newline that ends the group with it, so the blocks tile.
        let end = match lines.get(last + 1) {
            Some(next) => next.start,
            None => text.len(),
        };
        let Some(range) = Range::new(start, end) else {
            break;
        };
        grouped.push(Block {
            kind,
            source: range.slice(text),
            range,
        });
        index = last + 1;
    }

    grouped
}

/// An inline construct the formatted view can render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// `**strong**`
    Strong,
    /// `*emphasis*` or `_emphasis_`
    Emphasis,
    /// `~~strikethrough~~`
    Strike,
    /// A backtick code span.
    InlineCode,
    /// `[label](target)`
    Link,
    /// `![alt](target)`
    Image,
    /// A bare `<https://example.com>`.
    AutoLink,
}

/// A span of inline markup, and the text inside its delimiters.
///
/// [`Span::range`] includes the delimiters — the bytes the user typed — and
/// [`Span::content`] is what lies between them. Both are offsets into the text
/// given to [`spans`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// What kind of markup this is.
    pub kind: SpanKind,
    /// The whole construct, delimiters included.
    pub range: Range,
    /// The text between the delimiters.
    pub content: Range,
}

/// Finds the inline constructs in `text`, left to right and never overlapping.
///
/// Delimiters are matched at most one level deep: the first unresolved opener looks
/// for its first matching closer, and everything between them is the span's
/// content. An opener with no closer in its line is skipped and stays as the
/// character it is, which is what keeps a stray `**` from swallowing the rest of a
/// paragraph.
pub fn spans(text: &str) -> Vec<Span> {
    let mut found = Vec::new();
    let mut start = 0usize;
    while start <= text.len() {
        let end = match text[start..].find('\n') {
            Some(offset) => start + offset,
            None => text.len(),
        };
        scan_line(text, start, end, &mut found);
        start = end + 1;
    }
    found
}

/// The byte offsets at which `text`'s lines begin, with each line's kind.
fn line_kinds(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    while start <= text.len() {
        if start == text.len() && !lines.is_empty() {
            // A trailing newline ends the last line; it does not start another one.
            break;
        }
        let end = match text[start..].find('\n') {
            Some(offset) => start + offset,
            None => text.len(),
        };
        let Some(range) = Range::new(start, end) else {
            break;
        };
        lines.push(Line {
            start,
            kind: classify(range.slice(text)),
        });
        if end == text.len() {
            break;
        }
        start = end + 1;
    }
    lines
}

struct Line {
    start: usize,
    kind: BlockKind,
}

/// What a single line of source is.
fn classify(line: &str) -> BlockKind {
    if line.trim().is_empty() {
        return BlockKind::Blank;
    }
    if let Some((_, _, level)) = heading_prefix(line) {
        return BlockKind::Heading(level);
    }
    if let Some((indent, _, marker)) = list_prefix(line) {
        return BlockKind::ListItem { indent, marker };
    }
    if quote_prefix(line).is_some() {
        return BlockKind::Quote;
    }
    let trimmed = line.trim();
    if trimmed.len() > 1 && trimmed.starts_with('|') && trimmed.ends_with('|') {
        return BlockKind::TableRow;
    }
    BlockKind::Paragraph
}

/// Walks one line, appending spans and skipping over what they consume.
fn scan_line(text: &str, from: usize, to: usize, found: &mut Vec<Span>) {
    let mut index = from;
    while index < to {
        let Some(closed) = span_at(text, index, to) else {
            index = next_char(text, index);
            continue;
        };
        if closed.range.end <= index {
            // Defensive: a scanner bug must not loop forever.
            index = next_char(text, index);
            continue;
        }
        found.push(closed);
        index = closed.range.end;
    }
}

/// The construct starting exactly at `at`, if there is one that closes.
fn span_at(text: &str, at: usize, to: usize) -> Option<Span> {
    let bytes = text.as_bytes();
    match bytes[at] {
        b'`' => code_span(text, at, to),
        b'!' => image(text, at, to),
        b'[' => link(text, at, at + 1, to, SpanKind::Link),
        b'<' => auto_link(text, at, to),
        b'~' if bytes.get(at + 1) == Some(&b'~') => paired(text, at, to, "~~", SpanKind::Strike),
        b'*' if bytes.get(at + 1) == Some(&b'*') => paired(text, at, to, "**", SpanKind::Strong),
        b'*' => emphasised(text, at, to, b'*'),
        b'_' if starts_a_word(bytes, at) => emphasised(text, at, to, b'_'),
        _ => None,
    }
}

/// A backtick run and its matching run of the same length.
fn code_span(text: &str, from: usize, to: usize) -> Option<Span> {
    let run = run_len(text, from, b'`');
    let content_start = from + run;
    let tail = text.get(content_start..to)?;
    let fence = "`".repeat(run);
    let offset = tail.find(&fence)?;
    let content_end = content_start + offset;
    let end = content_end + run;
    Some(Span {
        kind: SpanKind::InlineCode,
        range: Range::new(from, end)?,
        content: Range::new(content_start, content_end)?,
    })
}

/// `**bold**` and `~~struck~~`: an opener run and its first matching closer.
fn paired(text: &str, from: usize, to: usize, marker: &str, kind: SpanKind) -> Option<Span> {
    let width = marker.len();
    let content_start = from + width;
    if content_start >= to {
        return None;
    }
    let tail = text.get(content_start..to)?;
    let offset = tail.find(marker)?;
    let content_end = content_start + offset;
    if content_end == content_start {
        // `****` marks nothing; leave the characters to the user.
        return None;
    }
    let end = content_end + width;
    Some(Span {
        kind,
        range: Range::new(from, end)?,
        content: Range::new(content_start, content_end)?,
    })
}

/// `*emphasis*` and `_emphasis_`, where the closing delimiter is a lone one.
fn emphasised(text: &str, from: usize, to: usize, marker: u8) -> Option<Span> {
    let bytes = text.as_bytes();
    let content_start = from + 1;
    if content_start >= to || bytes.get(content_start) == Some(&marker) {
        return None;
    }
    // Emphasis hugs its content: `* not emphasis *` is a pair of asterisks.
    if bytes[content_start].is_ascii_whitespace() {
        return None;
    }
    let mut index = content_start;
    while index < to {
        if bytes[index] == marker
            && bytes.get(index + 1) != Some(&marker)
            && bytes.get(index.wrapping_sub(1)) != Some(&marker)
            && !bytes[index - 1].is_ascii_whitespace()
        {
            let end = index + 1;
            return Some(Span {
                kind: SpanKind::Emphasis,
                range: Range::new(from, end)?,
                content: Range::new(content_start, index)?,
            });
        }
        index = next_char(text, index);
    }
    None
}

/// `[label](target)` and `![alt](target)`, where `from` is the delimiter that
/// opens the construct and `label_open` is the byte after its opening bracket.
fn link(text: &str, from: usize, label_open: usize, to: usize, kind: SpanKind) -> Option<Span> {
    let tail = text.get(label_open..to)?;
    let close = tail.find(']')?;
    let label_end = label_open + close;
    let after = text.get(label_end + 1..to)?;
    if !after.starts_with('(') {
        return None;
    }
    // Safe to index: the counted byte is an ASCII `(`.
    let target = &after[1..];
    let target_end = target.find(')')?;
    let end = label_end + 2 + target_end + 1;
    Some(Span {
        kind,
        range: Range::new(from, end)?,
        content: Range::new(label_open, label_end)?,
    })
}

/// `![alt](target)`, if the `!` is followed by a bracketed link.
fn image(text: &str, from: usize, to: usize) -> Option<Span> {
    if text.get(from..from + 1)? != "!" {
        return None;
    }
    link(text, from, from + 2, to, SpanKind::Image)
}

/// A `<https://…>` autolink.
fn auto_link(text: &str, from: usize, to: usize) -> Option<Span> {
    let tail = text.get(from + 1..to)?;
    let close = tail.find('>')?;
    let inside = tail.get(..close)?;
    let schemeless = inside
        .strip_prefix("http://")
        .or(inside.strip_prefix("https://"))
        .or(inside.strip_prefix("mailto:"))?;
    if schemeless.is_empty() || schemeless.contains(' ') || schemeless.contains('>') {
        return None;
    }
    let end = from + 1 + close + 1;
    Some(Span {
        kind: SpanKind::AutoLink,
        range: Range::new(from, end)?,
        content: Range::new(from + 1, from + 1 + close)?,
    })
}

/// How many times `marker` repeats from `at`.
fn run_len(text: &str, at: usize, marker: u8) -> usize {
    text.as_bytes()
        .iter()
        .skip(at)
        .take_while(|byte| **byte == marker)
        .count()
}

/// Whether a `_` at `at` opens a word, rather than sitting inside one.
///
/// Without this, `snake_case_word` would come back emphasised, which is the
/// oldest Markdown parser mistake there is.
fn starts_a_word(bytes: &[u8], at: usize) -> bool {
    match bytes.get(at.wrapping_sub(1)) {
        None => true,
        Some(previous) => !(previous.is_ascii_alphanumeric() || *previous == b'_'),
    }
}

/// The offset of the next character at or after `at`, never the same one twice.
fn next_char(text: &str, at: usize) -> usize {
    let width = text[at..].chars().next().map_or(1, |c| c.len_utf8());
    (at + width).min(text.len())
}

/// A slice with its trailing newlines removed.
fn trim_newlines(text: &str) -> &str {
    let end = text.len() - text.bytes().rev().take_while(|byte| *byte == b'\n').count();
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_tile_the_source_exactly() {
        for text in [
            "",
            "\n",
            "one\n",
            "# Title\n\nSome words.\n",
            "- one\n- two\n\n12. three\n",
            "a\n\n\nb",
            "```\nliteral\n```",
        ] {
            let rebuilt: String = blocks(text).iter().map(|block| block.source).collect();
            assert_eq!(rebuilt, text, "blocks of {text:?} did not tile");
        }
    }

    #[test]
    fn an_unknown_construct_is_a_paragraph_not_an_error() {
        let source = "<div class=x>\n[^1]: footnote\n";
        let read = blocks(source);
        assert_eq!(read.len(), 1, "two lines of prose, one block");
        assert_eq!(read[0].kind, BlockKind::Paragraph);
        assert_eq!(
            read[0].content(),
            "<div class=x>\n[^1]: footnote",
            "the markup is carried as written, not parsed away"
        );
    }

    #[test]
    fn a_blocks_text_drops_its_marker_but_keeps_its_bytes() {
        let read = blocks("# Title\n- one\n");
        assert_eq!(read[0].kind, BlockKind::Heading(1));
        assert_eq!(read[0].content(), "Title");
        assert_eq!(read[0].source, "# Title\n", "the source is what was read");
        assert_eq!(read[1].content(), "one");
    }

    #[test]
    fn spans_do_not_cross_lines_or_overlap() {
        let text = "**one** and *two*\n*three\n";
        let found = spans(text);
        assert_eq!(
            found.len(),
            2,
            "the `*three` opener has no closer: {found:?}"
        );
        assert_eq!(found[0].kind, SpanKind::Strong);
        assert_eq!(found[0].content.slice(text), "one");
        assert_eq!(found[1].kind, SpanKind::Emphasis);
        assert_eq!(found[1].content.slice(text), "two");
        for pair in found.windows(2) {
            assert!(pair[0].range.end <= pair[1].range.start, "{found:?}");
        }
        assert!(
            found.iter().all(|span| span.range.end <= 17),
            "no span reaches past the newline: {found:?}"
        );
    }

    #[test]
    fn an_unpaired_delimiter_is_left_alone() {
        assert!(
            spans("a **b c")
                .iter()
                .all(|span| span.kind != SpanKind::Strong)
        );
    }

    #[test]
    fn underscores_inside_a_word_are_not_emphasis() {
        assert!(
            spans("snake_case_word").is_empty(),
            "{:?}",
            spans("snake_case_word")
        );
    }
}
