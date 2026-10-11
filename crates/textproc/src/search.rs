//! Finding text (`docs/architecture.md` §4, M5 in `ROADMAP.md`).
//!
//! This module is the engine behind Find, Find next and Replace
//! (`docs/feature-matrix.md` NC-S001–NC-S005). It answers one question — where
//! is this text — and `nc-core`'s `Document::replace_ranges` does the
//! replacing, so a search can be tested without a window and a replacement
//! without a dialog.
//!
//! A match is a byte range, the same offsets the rest of the engine uses: a
//! [`Match`] can be handed straight to `Document::replace_ranges`, and
//! [`Match::text`] reads it back out of the document's own `text()`.
//!
//! # Invariants
//!
//! * **A match never splits a character.** Everything here matches on
//!   character boundaries, so `&text[start..end]` can always be taken from a
//!   returned range.
//! * **Searching never changes the text.** Case-insensitive matching folds a
//!   *copy* of the document and keeps a map back to the original offsets; the
//!   document itself is only ever read. This is the M4 rule that reading never
//!   substitutes, applied to searching.
//! * **Matches are the shape Replace All needs.** [`find_all`] returns
//!   non-overlapping matches in order, which is exactly what
//!   `Document::replace_ranges` requires: one undo step for the whole run.
//!
//! # Evidence discipline
//!
//! The reference's exact behaviour is not captured for every corner, and where
//! it is not, the choice is named rather than asserted as parity
//! (`docs/reference-matrix.md` §5). In this module that means: matches are
//! non-overlapping and found left to right (so a needle that could match
//! inside itself, like `aa` in `aaa`, matches once from the left rather than
//! twice); a *whole word* edge is any character that is not a letter, a digit
//! or an underscore; and the wording of the "can't find text" and
//! "beginning/end of document reached" notices belongs to the shell, which
//! gets [`SearchResult::NoMatch`] and [`SearchResult::Boundary`] and spells
//! them its own way.

use std::ops::Range;

/// Which way a search travels from the caret.
///
/// The direction is one of the reference's Find options
/// (`docs/feature-matrix.md` NC-S004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Towards the end of the document.
    #[default]
    Forward,
    /// Towards the start of the document.
    Backward,
}

/// How a search looks (`docs/feature-matrix.md` NC-S004).
///
/// The reference's Find dialog carries *Match case* and *Match whole word
/// only*, both off when it opens, and both kept for the next Find.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SearchOptions {
    /// Compare case exactly. Off by default, as in the reference.
    pub match_case: bool,
    /// Match only whole words. Off by default, as in the reference.
    pub whole_word: bool,
}

impl SearchOptions {
    /// The reference's defaults: either case, anywhere in a word.
    pub const fn new() -> Self {
        Self {
            match_case: false,
            whole_word: false,
        }
    }

    /// The same options with *Match case* on.
    #[must_use]
    pub const fn matching_case(self) -> Self {
        Self {
            match_case: true,
            ..self
        }
    }

    /// The same options with *Match whole word only* on.
    #[must_use]
    pub const fn whole_words(self) -> Self {
        Self {
            whole_word: true,
            ..self
        }
    }
}

/// One occurrence of the needle: a byte range into the haystack it was found
/// in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    /// Where the match starts, in bytes from the start of the haystack.
    pub start: usize,
    /// Where the match ends: one past its last byte.
    pub end: usize,
}

impl Match {
    /// A match covering `start..end`.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// The text this match covers, read back out of the haystack it was found
    /// in.
    pub fn text<'a>(&self, haystack: &'a str) -> &'a str {
        &haystack[self.start..self.end]
    }

    /// The match as a range, for `Document::replace_ranges`.
    pub const fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// How many bytes the match covers. Never zero: an empty needle matches
    /// nothing.
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Whether the match covers no bytes. Always false for a search's own
    /// matches, which is what keeps `find_all`'s list valid input for
    /// `replace_ranges`.
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// What one search step found, or why it found nothing
/// (`docs/feature-matrix.md` NC-S002, NC-S005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchResult {
    /// The match the search was looking for.
    Found(Match),
    /// The match the search was looking for, reached by wrapping around one
    /// end of the document. The reference stops and says so before wrapping,
    /// which is why this is not just a [`SearchResult::Found`].
    Wrapped(Match),
    /// The search ran out of document without wrapping, and there *is* a match
    /// elsewhere: the reference's "Beginning/end of document reached" notice.
    Boundary,
    /// The text is not in the document at all: the reference's "Cannot find"
    /// message.
    NoMatch,
}

impl SearchResult {
    /// The match, if the search found one.
    pub const fn matched(self) -> Option<Match> {
        match self {
            Self::Found(found) | Self::Wrapped(found) => Some(found),
            Self::Boundary | Self::NoMatch => None,
        }
    }

    /// Whether the search found text.
    pub const fn found(self) -> bool {
        self.matched().is_some()
    }
}

/// Finds `needle` in `haystack`, travelling `direction` from `from`
/// (`docs/feature-matrix.md` NC-S001, NC-S002).
///
/// `from` is the caret — or the end of the selection, since that is where the
/// reference's caret is — and is clamped to the text and down to a character
/// boundary. A match *at* `from` counts, travelling either way.
///
/// With `wraps` set the search carries on from the other end of the document
/// and says so with [`SearchResult::Wrapped`]; without it, the search stops at
/// the end of the document it was travelling towards and reports
/// [`SearchResult::Boundary`] when a match exists elsewhere, so the shell can
/// tell "wrapped" apart from "there is nothing to find".
pub fn find(
    haystack: &str,
    needle: &str,
    from: usize,
    direction: Direction,
    options: SearchOptions,
    wraps: bool,
) -> SearchResult {
    let matches = find_all(haystack, needle, options);
    let Some(first) = matches.first().copied() else {
        return SearchResult::NoMatch;
    };
    let from = clamp(haystack, from);
    match direction {
        Direction::Forward => match matches.iter().find(|found| found.start >= from) {
            Some(found) => SearchResult::Found(*found),
            None if wraps => SearchResult::Wrapped(first),
            None => SearchResult::Boundary,
        },
        Direction::Backward => {
            let last = matches[matches.len() - 1];
            match matches.iter().rev().find(|found| found.end <= from) {
                Some(found) => SearchResult::Found(*found),
                None if wraps => SearchResult::Wrapped(last),
                None => SearchResult::Boundary,
            }
        }
    }
}

/// Finds the next occurrence at or after `from` (Find next, F3).
pub fn find_next(
    haystack: &str,
    needle: &str,
    from: usize,
    options: SearchOptions,
    wraps: bool,
) -> SearchResult {
    find(haystack, needle, from, Direction::Forward, options, wraps)
}

/// Finds the last occurrence at or before `from` (Find previous, Shift+F3).
pub fn find_previous(
    haystack: &str,
    needle: &str,
    from: usize,
    options: SearchOptions,
    wraps: bool,
) -> SearchResult {
    find(haystack, needle, from, Direction::Backward, options, wraps)
}

/// Every occurrence of `needle`, left to right and not overlapping
/// (`docs/feature-matrix.md` NC-S003: Replace All walks this list).
///
/// The result is exactly the shape `Document::replace_ranges` wants, so
/// Replace All is `find_all` followed by one call, and the whole run is one
/// undo step. An empty needle finds nothing rather than matching at every
/// position, which is what the reference's disabled Find button amounts to.
pub fn find_all(haystack: &str, needle: &str, options: SearchOptions) -> Vec<Match> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    if options.match_case {
        return matches_exactly(haystack, needle, options.whole_word);
    }

    // Case-insensitively: search a folded copy, then translate the offsets
    // back. The document itself is never touched.
    let folded = Folded::new(haystack);
    let lowered = needle.to_lowercase();
    let mut found = Vec::new();
    let mut at = 0usize;
    while at <= folded.text.len() {
        let Some(offset) = folded.text[at..].find(&lowered) else {
            break;
        };
        let start = at + offset;
        at = start + lowered.len();
        let Some(candidate) = folded.to_original(start, lowered.len()) else {
            continue;
        };
        if !options.whole_word || is_whole_word(haystack, candidate) {
            found.push(candidate);
        }
    }
    found
}

/// The offset *Go to line* moves the caret to
/// (`docs/feature-matrix.md` NC-S006).
///
/// Lines are counted from 1, the way the status bar counts them. A line number
/// that is not in the document clamps rather than failing: 0 and 1 are the
/// start of the text, and a number past the last line is the start of the last
/// line — which is the behaviour the reference's dialog has, and the reason
/// this returns an offset instead of an error for the shell to report.
pub fn go_to_line(text: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut start = 0usize;
    let mut number = 1usize;
    for (offset, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            number += 1;
            start = offset + 1;
            if number == line {
                return start;
            }
        }
    }
    start
}

/// Reads a *Go to line* entry: the digits of a line number, or nothing
/// (`docs/feature-matrix.md` NC-S006).
///
/// The field takes digits only, with blanks around them. Anything else —
/// empty, letters, a sign, or a number too large for a byte offset on this
/// platform — is [`None`], so the shell can leave the dialog as the user typed
/// it instead of moving the caret somewhere they did not ask for.
pub fn parse_line_number(input: &str) -> Option<usize> {
    let digits = input.trim();
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Whether `character` counts as part of a word for *Match whole word only*.
///
/// Letters and digits by Unicode's reckoning, plus the underscore.
pub fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Finds matches byte for byte, without folding case.
fn matches_exactly(haystack: &str, needle: &str, whole_word: bool) -> Vec<Match> {
    let mut found = Vec::new();
    let mut at = 0usize;
    while at <= haystack.len() {
        let Some(offset) = haystack[at..].find(needle) else {
            break;
        };
        let candidate = Match::new(at + offset, at + offset + needle.len());
        at = candidate.end;
        if !whole_word || is_whole_word(haystack, candidate) {
            found.push(candidate);
        }
    }
    found
}

/// Whether the characters on either side of `found` are outside a word.
fn is_whole_word(haystack: &str, found: Match) -> bool {
    let before = haystack[..found.start].chars().next_back();
    let after = haystack[found.end..].chars().next();
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

/// `byte`, clamped to `text` and moved down to a character boundary.
fn clamp(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

/// The haystack lowercased, with the place every character came from.
///
/// Folding is not one character to one character — `İ` lowercases to `i`
/// followed by a combining dot — so a lookup table is kept: entry *n* is where
/// the *n*-th character of the original starts, and where its lowercase form
/// starts in the copy. A sentinel entry at the end gives the map an answer for
/// the offset one past the last character.
struct Folded {
    text: String,
    origin: Vec<(usize, usize)>,
}

impl Folded {
    fn new(haystack: &str) -> Self {
        let mut text = String::with_capacity(haystack.len());
        let mut origin = Vec::with_capacity(haystack.len() + 1);
        for (offset, character) in haystack.char_indices() {
            origin.push((offset, text.len()));
            text.extend(character.to_lowercase());
        }
        origin.push((haystack.len(), text.len()));
        Self { text, origin }
    }

    /// The original range that a match of `length` bytes at `start` in the
    /// folded text covers, or [`None`] when the match does not line up with
    /// whole characters of the original.
    fn to_original(&self, start: usize, length: usize) -> Option<Match> {
        let first = self.unit_at(start)?;
        let past_last = self.unit_at(start + length)?;
        if past_last <= first {
            return None;
        }
        let (begin, _) = self.origin[first];
        let (end, _) = self.origin[past_last];
        Some(Match::new(begin, end))
    }

    /// The index of the character whose lowercase form begins exactly at
    /// `folded`.
    fn unit_at(&self, folded: usize) -> Option<usize> {
        let index = self
            .origin
            .partition_point(|&(_, begins)| begins <= folded)
            .checked_sub(1)?;
        (self.origin[index].1 == folded).then_some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_reference_s_checked_state() {
        let options = SearchOptions::new();
        assert_eq!(options, SearchOptions::default());
        assert!(!options.match_case);
        assert!(!options.whole_word);
        assert_eq!(Direction::default(), Direction::Forward);
    }

    #[test]
    fn finding_is_case_insensitive_until_asked_otherwise() {
        let found = find_next("Alpha", "alpha", 0, SearchOptions::new(), false);
        assert_eq!(found.matched().map(|found| found.range()), Some(0..5));
        let found = find_next(
            "Alpha",
            "alpha",
            0,
            SearchOptions::new().matching_case(),
            false,
        );
        assert_eq!(found, SearchResult::NoMatch);
    }

    #[test]
    fn an_empty_needle_matches_nothing() {
        assert!(find_all("text", "", SearchOptions::new()).is_empty());
        assert_eq!(
            find_next("text", "", 0, SearchOptions::new(), true),
            SearchResult::NoMatch
        );
    }
}
