//! Formatting commands, expressed as edits to Markdown source.
//!
//! Each command takes the text and the current selection and returns an
//! [`Outcome`]. Three rules hold throughout:
//!
//! * **Only the markers change.** A command adds or removes the bytes of its own
//!   syntax and leaves every other byte alone, including bytes the parser does not
//!   understand. Nothing here re-flowes, re-wraps, or tidies the text.
//! * **Toggling is reversible.** Wrapping a selection leaves the selection covering
//!   the markers, so the same command run again unwraps it. One keystroke and one
//!   undo step are the same answer.
//! * **Invalid input does nothing.** A selection that is out of bounds or splits a
//!   `char` returns `None`, so the shell can skip the edit rather than perform a
//!   partial one.
//!
//! Block commands (headings, lists, indentation) act on every line the selection
//! touches, which is what a user means by selecting partway through a paragraph.

use crate::{Outcome, Patch, Range, apply};

/// How far one level of list nesting is indented.
pub const INDENT: &str = "  ";

/// Inline emphasis that a command toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inline {
    /// `**strong**`
    Bold,
    /// `*emphasis*`
    Italic,
    /// `~~strikethrough~~`
    Strike,
}

impl Inline {
    /// The two-byte marker this emphasis is written with.
    pub fn marker(self) -> &'static str {
        match self {
            Inline::Bold => "**",
            Inline::Italic => "*",
            Inline::Strike => "~~",
        }
    }
}

/// A paragraph's ATX heading level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heading {
    /// Unheadinged text.
    Body,
    /// `#` through `######`, so `1..=6`.
    Level(u8),
}

impl Heading {
    /// The heading level a line already carries.
    pub fn of(line: &str) -> Self {
        heading_prefix(line).map_or(Heading::Body, |(_, _, level)| Heading::Level(level))
    }

    /// The ATX prefix this heading writes, and `""` for body text.
    pub fn prefix(self) -> String {
        match self {
            Heading::Body => String::new(),
            Heading::Level(level) => format!("{} ", "#".repeat(level as usize)),
        }
    }

    /// A level the ATX syntax can express, so `1..=6`.
    pub fn from_level(level: u8) -> Option<Self> {
        if (1..=6).contains(&level) {
            Some(Heading::Level(level))
        } else {
            None
        }
    }
}

/// The list kind a command applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// `- item`
    Bullet,
    /// `1. item`, numbered from one across the selected lines.
    Numbered,
}

impl ListKind {
    /// The marker written before a line's content, given its position in the list.
    pub fn marker(self, index: usize) -> String {
        match self {
            ListKind::Bullet => "- ".to_string(),
            ListKind::Numbered => format!("{}. ", index + 1),
        }
    }
}

/// Wraps the selection in `kind`'s markers, or removes markers already there
/// (`NC-M003`, `NC-M004`).
///
/// Three situations are distinguished, because a user may mean any of them:
/// markers sitting just *outside* the selection (they selected the words, not the
/// stars), markers *inside* it (they selected the whole marked run), or neither
/// (wrap it). An empty selection gets both markers and the caret between them.
pub fn toggle_inline(text: &str, selection: Range, kind: Inline) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let marker = kind.marker();
    let width = marker.len();

    if selection.is_empty() {
        // Written out directly rather than through `apply`: a caret has no edges to
        // stretch, and what the user wants is to type *between* the markers.
        let mut edited = String::with_capacity(text.len() + width * 2);
        edited.push_str(&text[..selection.start]);
        edited.push_str(marker);
        edited.push_str(marker);
        edited.push_str(&text[selection.start..]);
        return Some(Outcome {
            text: edited,
            selection: Range::caret(selection.start + width),
        });
    }

    // Markers on the outside: the user selected the words they want unmarked.
    if preceded_by(text, selection.start, marker) && followed_by(text, selection.end, marker) {
        let outer = [
            Patch::delete(Range::new(selection.start - width, selection.start)?),
            Patch::delete(Range::new(selection.end, selection.end + width)?),
        ];
        return Some(apply(text, &outer, selection));
    }

    // Markers on the inside: the user selected the whole marked run.
    if selection.len() > width * 2
        && followed_by(text, selection.start, marker)
        && preceded_by(text, selection.end, marker)
    {
        let inner = [
            Patch::delete(Range::new(selection.start, selection.start + width)?),
            Patch::delete(Range::new(selection.end - width, selection.end)?),
        ];
        return Some(apply(text, &inner, selection));
    }

    // Otherwise wrap, and let the selection carry the markers so a second press
    // meets the "inside" case above and unwraps.
    let wrap = [
        Patch::insert(selection.start, marker),
        Patch::insert(selection.end, marker),
    ];
    Some(apply(text, &wrap, selection))
}

/// Sets every line the selection touches to `heading`, or back to body text when
/// the first line already carries that level (`NC-M005`).
pub fn set_heading(text: &str, selection: Range, heading: Heading) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let lines = lines_covering(text, selection);
    if lines.is_empty() {
        return None;
    }

    let current = Heading::of(lines[0].slice(text));
    let target = if current == heading {
        Heading::Body
    } else {
        heading
    };
    let prefix = target.prefix();

    let mut patches = Vec::new();
    for line in lines {
        let content = line.slice(text);
        // Whatever block syntax already leads the line gives way, so a bullet does
        // not survive the change as "- # Title".
        match (block_prefix(content), prefix.as_str()) {
            (None, "") => {}
            (None, replacement) => {
                patches.push(Patch::insert(line.start + indent_len(content), replacement));
            }
            (Some((at, len)), "") => {
                patches.push(Patch::delete(Range::new(
                    line.start + at,
                    line.start + at + len,
                )?));
            }
            (Some((at, len)), replacement) => {
                patches.push(Patch {
                    at: Range::new(line.start + at, line.start + at + len)?,
                    with: replacement.to_string(),
                });
            }
        }
    }

    Some(apply(text, &patches, selection))
}

/// Applies `kind` to every line the selection touches, or strips that kind when
/// every line already carries it (`NC-M006`).
///
/// Indentation is left where it is, so nesting survives: the list command changes
/// the marker, [`indent_lines`] changes the level.
pub fn set_list(text: &str, selection: Range, kind: ListKind) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let lines = lines_covering(text, selection);
    if lines.is_empty() {
        return None;
    }

    let stripping = lines
        .iter()
        .all(|line| carries_kind(line.slice(text), kind));

    let mut patches = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let content = line.slice(text);
        if stripping {
            let Some((at, len, _)) = list_prefix(content) else {
                continue;
            };
            patches.push(Patch::delete(Range::new(
                line.start + at,
                line.start + at + len,
            )?));
            continue;
        }
        let marker = kind.marker(index);
        match block_prefix(content) {
            None => patches.push(Patch::insert(line.start + indent_len(content), marker)),
            Some((at, len)) => patches.push(Patch {
                at: Range::new(line.start + at, line.start + at + len)?,
                with: marker,
            }),
        }
    }

    Some(apply(text, &patches, selection))
}

/// Indents every line the selection touches by one [`INDENT`] level, which is how
/// a list item becomes a nested one (`NC-M006`).
pub fn indent_lines(text: &str, selection: Range) -> Option<Outcome> {
    block_edit(text, selection, |line| {
        Some(Patch::insert(line.start, INDENT))
    })
}

/// Removes up to one [`INDENT`] level of leading whitespace from every line the
/// selection touches.
pub fn outdent_lines(text: &str, selection: Range) -> Option<Outcome> {
    block_edit(text, selection, |line| {
        let whitespace = indent_len(line.slice(text));
        if whitespace == 0 {
            return None;
        }
        let removed = whitespace.min(INDENT.len());
        Some(Patch::delete(Range::new(line.start, line.start + removed)?))
    })
}

/// Strips Markdown syntax from the selection, leaving the text it marked
/// (`NC-M009`).
///
/// Removed: heading, list, and blockquote markers, along with the indentation
/// ahead of them; then `**`, `~~`, and paired `*`. Kept: underscores, so
/// `snake_case_words` survives, and any marker with no partner, because an
/// unpaired asterisk is usually the user's own.
pub fn clear_formatting(text: &str, selection: Range) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let mut outcome = strip_block_prefixes(text, selection)?;
    for marker in ["**", "~~"] {
        outcome = remove_pairs(&outcome.text, outcome.selection, marker)?;
    }
    remove_star_pairs(&outcome.text, outcome.selection)
}

/// Wraps the selection as a Markdown link label (`NC-M007`).
///
/// With a selection the result is `[selected text](url)` and the selection covers
/// the whole link. With a caret an empty label and target are written and the caret
/// is left inside the brackets, ready to type.
pub fn insert_link(text: &str, selection: Range, url: &str) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let written = format!("[{}]({url})", selection.slice(text));
    if selection.is_empty() {
        let outcome = apply(text, &[Patch::insert(selection.start, written)], selection);
        return Some(Outcome {
            selection: Range::caret(outcome.selection.start + 1),
            ..outcome
        });
    }
    Some(apply(
        text,
        &[Patch {
            at: selection,
            with: written,
        }],
        selection,
    ))
}

/// Writes a GitHub-flavoured table: a header row, a separator row, and `rows` body
/// rows (`NC-M008`).
///
/// The block replaces the selection wherever it sits and the caret is left in the
/// first header cell, since an empty table is filled in by typing.
pub fn insert_table(text: &str, selection: Range, columns: usize, rows: usize) -> Option<Outcome> {
    if !selection.is_within(text) || columns == 0 || columns > 64 || rows > 64 {
        return None;
    }
    let row = |cell: &str| -> String {
        let mut line = String::from("|");
        for _ in 0..columns {
            line.push_str(cell);
            line.push('|');
        }
        line
    };
    let mut block = row("   ");
    block.push('\n');
    block.push_str(&row(" --- "));
    for _ in 0..rows {
        block.push('\n');
        block.push_str(&row("   "));
    }

    let outcome = apply(
        text,
        &[Patch {
            at: selection,
            with: block,
        }],
        selection,
    );
    Some(Outcome {
        selection: Range::caret(outcome.selection.start + 2),
        ..outcome
    })
}

/// The lines a selection covers, as ranges over each line's content with the
/// newline excluded.
///
/// A non-empty selection that ends exactly at the start of a line does not drag
/// that line in — that is where the caret sits after Shift+Down at the end of a
/// line, and the user means the line above. An empty one covers the line it sits
/// in.
fn lines_covering(text: &str, selection: Range) -> Vec<Range> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            push_line(&mut lines, text, selection, start, index);
            start = index + 1;
        }
    }
    push_line(&mut lines, text, selection, start, text.len());
    lines
}

fn push_line(lines: &mut Vec<Range>, _text: &str, selection: Range, start: usize, end: usize) {
    let Some(end) = Range::new(start, end.max(start)) else {
        return;
    };
    let touches = selection.start <= end.end && selection.end > end.start;
    let sits_here =
        selection.is_empty() && selection.start >= end.start && selection.start <= end.end;
    if touches || sits_here {
        lines.push(end);
    }
}

/// Rewrites each line a selection touches with a patch, dropping lines that
/// produce none.
fn block_edit(
    text: &str,
    selection: Range,
    mut per_line: impl FnMut(Range) -> Option<Patch>,
) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let patches: Vec<Patch> = lines_covering(text, selection)
        .iter()
        .filter_map(|line| per_line(*line))
        .collect();
    if patches.is_empty() {
        return Some(Outcome {
            text: text.to_string(),
            selection,
        });
    }
    Some(apply(text, &patches, selection))
}

/// Removes any heading, list, or blockquote marker from each line the selection
/// touches, taking the indentation ahead of it with it, so a nested item returns to
/// the margin. A line with no marker is left exactly as it is: indentation in
/// ordinary prose is the author's, not formatting to clear.
fn strip_block_prefixes(text: &str, selection: Range) -> Option<Outcome> {
    let patches: Vec<Patch> = lines_covering(text, selection)
        .iter()
        .filter_map(|line| {
            let (at, len) = block_prefix(line.slice(text))?;
            Some(Patch::delete(Range::new(
                line.start,
                line.start + at + len,
            )?))
        })
        .collect();
    if patches.is_empty() {
        return Some(Outcome {
            text: text.to_string(),
            selection,
        });
    }
    Some(apply(text, &patches, selection))
}

/// Removes every non-overlapping occurrence of `needle` inside the selection.
fn remove_pairs(text: &str, selection: Range, needle: &str) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let window = selection.slice(text);
    let mut patches = Vec::new();
    let mut cursor = 0usize;
    while let Some(offset) = window[cursor..].find(needle) {
        let at = selection.start + cursor + offset;
        patches.push(Patch::delete(Range::new(at, at + needle.len())?));
        cursor += offset + needle.len();
    }
    if patches.is_empty() {
        return Some(Outcome {
            text: text.to_string(),
            selection,
        });
    }
    Some(apply(text, &patches, selection))
}

/// Removes `*` that wrap something, and leaves the rest.
///
/// An asterisk opens only when a non-blank, non-asterisk character follows it and
/// closes only when one precedes it, so the `*` in `a * b * c` — a multiplication,
/// not emphasis — stays where the user put it.
fn remove_star_pairs(text: &str, selection: Range) -> Option<Outcome> {
    if !selection.is_within(text) {
        return None;
    }
    let bytes = text.as_bytes();
    let blank = |byte: u8| byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r';
    let opens_at = |index: usize| {
        bytes
            .get(index + 1)
            .is_some_and(|next| *next != b'*' && !blank(*next))
    };
    let closes_at = |index: usize| {
        index > selection.start && bytes[index - 1] != b'*' && !blank(bytes[index - 1])
    };

    let mut patches = Vec::new();
    let mut opener: Option<usize> = None;
    for (index, byte) in bytes
        .iter()
        .enumerate()
        .take(selection.end)
        .skip(selection.start)
    {
        if *byte != b'*' {
            continue;
        }
        if closes_at(index) {
            if let Some(at) = opener.take() {
                patches.push(Patch::delete(Range::new(at, at + 1)?));
                patches.push(Patch::delete(Range::new(index, index + 1)?));
                continue;
            }
        }
        if opens_at(index) {
            opener = Some(index);
        }
    }
    if patches.is_empty() {
        return Some(Outcome {
            text: text.to_string(),
            selection,
        });
    }
    Some(apply(text, &patches, selection))
}

/// Whether `line` is already a list item of `kind`.
fn carries_kind(line: &str, kind: ListKind) -> bool {
    use crate::ListMarker::{Dash, Number, Plus, Star};
    list_prefix(line).is_some_and(|(_, _, marker)| match kind {
        ListKind::Bullet => matches!(marker, Dash | Plus | Star),
        ListKind::Numbered => matches!(marker, Number(_)),
    })
}

/// How many bytes of spaces and tabs lead a line.
pub(crate) fn indent_len(line: &str) -> usize {
    line.bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count()
}

/// The `#` run that leads a line, as (offset, length, level).
pub(crate) fn heading_prefix(line: &str) -> Option<(usize, usize, u8)> {
    let indent = indent_len(line);
    let rest = line.get(indent..)?;
    let hashes = rest.bytes().take_while(|byte| *byte == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    // Safe to index: every counted byte is an ASCII `#`.
    let after = &rest[hashes..];
    let spaces = after
        .bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count();
    if spaces == 0 && !after.is_empty() {
        // `#hashtag` is a word, not a heading.
        return None;
    }
    Some((indent, hashes + spaces, hashes as u8))
}

/// The list marker that leads a line, as (offset, length, marker).
pub(crate) fn list_prefix(line: &str) -> Option<(usize, usize, crate::ListMarker)> {
    use crate::ListMarker;
    let indent = indent_len(line);
    let rest = line.get(indent..)?;
    let bullet = match rest.as_bytes().first()? {
        b'-' => Some(ListMarker::Dash),
        b'+' => Some(ListMarker::Plus),
        b'*' => Some(ListMarker::Star),
        _ => None,
    };
    if let Some(marker) = bullet {
        // The `*` of `*emphasis*` is emphasis, not a bullet: a marker is followed
        // by a space or by nothing at all.
        let after = &rest[1..];
        if !(after.is_empty() || after.starts_with(' ') || after.starts_with('\t')) {
            return None;
        }
        let spaces = after
            .bytes()
            .take_while(|byte| *byte == b' ' || *byte == b'\t')
            .count();
        return Some((indent, 1 + spaces, marker));
    }

    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    // Safe to index: every counted byte is an ASCII digit.
    let tail = &rest[digits..];
    let tail = tail.strip_prefix('.')?;
    if !(tail.is_empty() || tail.starts_with(' ') || tail.starts_with('\t')) {
        return None;
    }
    let spaces = tail
        .bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count();
    let number = rest[..digits].parse::<u64>().ok()?;
    Some((indent, digits + 1 + spaces, ListMarker::Number(number)))
}

/// The blockquote marker that leads a line, if any.
pub(crate) fn quote_prefix(line: &str) -> Option<(usize, usize)> {
    let indent = indent_len(line);
    let rest = line.get(indent..)?;
    if !rest.starts_with('>') {
        return None;
    }
    let after = &rest[1..];
    let spaces = after
        .bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count();
    Some((indent, 1 + spaces))
}

/// The block-level marker a line leads with, whichever kind it is.
fn block_prefix(line: &str) -> Option<(usize, usize)> {
    if let Some((at, len, _)) = heading_prefix(line) {
        return Some((at, len));
    }
    if let Some((at, len, _)) = list_prefix(line) {
        return Some((at, len));
    }
    quote_prefix(line)
}

/// Whether `marker` sits immediately before `at`.
fn preceded_by(text: &str, at: usize, marker: &str) -> bool {
    let width = marker.len();
    if at < width {
        return false;
    }
    text.get(at - width..at)
        .is_some_and(|bytes| bytes == marker)
}

/// Whether `marker` sits immediately after `at`.
fn followed_by(text: &str, at: usize, marker: &str) -> bool {
    text.get(at..at + marker.len())
        .is_some_and(|bytes| bytes == marker)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranges(text: &str, selection: Range) -> Vec<(usize, usize)> {
        lines_covering(text, selection)
            .iter()
            .map(|range| (range.start, range.end))
            .collect()
    }

    #[test]
    fn a_caret_covers_only_its_own_line() {
        assert_eq!(ranges("one\ntwo", Range::caret(1)), vec![(0, 3)]);
        assert_eq!(ranges("one\ntwo", Range::caret(4)), vec![(4, 7)]);
    }

    #[test]
    fn a_selection_ending_at_a_line_start_does_not_drag_that_line_in() {
        assert_eq!(
            ranges("one\ntwo\nthree", Range::new(0, 4).unwrap()),
            vec![(0, 3)]
        );
        assert_eq!(
            ranges("one\ntwo\nthree", Range::new(0, 8).unwrap()),
            vec![(0, 3), (4, 7)]
        );
    }

    #[test]
    fn a_word_is_not_mistaken_for_a_heading_or_a_list() {
        assert_eq!(heading_prefix("#hashtag"), None);
        assert_eq!(heading_prefix("# Title"), Some((0, 2, 1)));
        assert_eq!(heading_prefix("####### seven"), None, "ATX stops at six");
        assert!(
            list_prefix("*emphasis*").is_none(),
            "emphasis, not a bullet"
        );
        assert!(list_prefix("* item").is_some());
        assert!(list_prefix("12. item").is_some());
        assert!(list_prefix("12nope").is_none());
    }

    #[test]
    fn a_star_that_marks_nothing_is_left_alone() {
        let outcome = remove_star_pairs("a * b * c", Range::new(0, 9).unwrap()).unwrap();
        assert_eq!(outcome.text, "a * b * c", "multiplication, not emphasis");
    }

    #[test]
    fn a_star_that_wraps_something_is_removed() {
        let outcome = remove_star_pairs("*emphasis*", Range::new(0, 10).unwrap()).unwrap();
        assert_eq!(outcome.text, "emphasis");
    }
}
