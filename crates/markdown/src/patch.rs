//! Applying a set of replacements to a text and carrying the selection with it.
//!
//! Every formatting command in this crate reduces to the same two steps: work out
//! which byte ranges the markers occupy, then rewrite them. Remapping the
//! selection is the half that decides whether the user can press the shortcut a
//! second time to undo it, so it is done in one place rather than re-derived at
//! each call site.

use crate::Range;

/// One replacement: the bytes at [`Patch::at`] become [`Patch::with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    /// The range of the original text this patch replaces.
    pub at: Range,
    /// What replaces it.
    pub with: String,
}

impl Patch {
    /// A patch that inserts `with` at `at` without removing anything.
    pub fn insert(at: usize, with: impl Into<String>) -> Self {
        Self {
            at: Range::caret(at),
            with: with.into(),
        }
    }

    /// A patch that removes `at`.
    pub fn delete(at: Range) -> Self {
        Self {
            at,
            with: String::new(),
        }
    }
}

/// The result of a formatting command: the new text, and where the selection went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The document text after the edits.
    pub text: String,
    /// The selection in terms of [`Outcome::text`].
    pub selection: Range,
}

impl Outcome {
    /// The edited text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Where the selection ended up.
    pub fn selection(&self) -> Range {
        self.selection
    }
}

/// Rewrites `text` with `patches`, mapping `selection` through the rewrite.
///
/// Patches may be given in any order but must not overlap. A position that falls
/// inside a replaced range collapses to the start of its replacement, which is
/// what keeps a collapsed selection from ending up stranded in deleted bytes.
///
/// The two ends are mapped by slightly different rules, and the difference is the
/// whole point: text inserted at the selection's *start* stays outside it, while
/// text inserted at its *end* is swallowed. That is what lets `Ctrl+B` hand back a
/// selection covering `**word**` — markers included — so pressing the shortcut
/// again finds markers inside the selection and unwraps them.
pub fn apply(text: &str, patches: &[Patch], selection: Range) -> Outcome {
    let mut ordered: Vec<&Patch> = patches.iter().collect();
    ordered.sort_by_key(|patch| patch.at.start);
    debug_assert!(
        ordered
            .windows(2)
            .all(|pair| pair[0].at.end <= pair[1].at.start),
        "patches must not overlap: {ordered:?}"
    );

    let mut rewritten = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut shifts: Vec<(usize, usize, i64)> = Vec::with_capacity(ordered.len());

    for patch in &ordered {
        rewritten.push_str(&text[cursor..patch.at.start]);
        rewritten.push_str(&patch.with);
        cursor = patch.at.end;
        shifts.push((
            patch.at.start,
            patch.at.end,
            patch.with.len() as i64 - patch.at.len() as i64,
        ));
    }
    rewritten.push_str(&text[cursor..]);

    Outcome {
        text: rewritten,
        selection: Range {
            start: map_one(selection.start, &shifts, Edge::Start),
            end: map_one(selection.end, &shifts, Edge::End),
        },
    }
}

/// Which edge of a selection is being mapped.
#[derive(Clone, Copy)]
enum Edge {
    /// Text added at this position belongs to the left of the selection.
    Start,
    /// Text added at this position belongs to the selection.
    End,
}

/// Where one old offset lands after the patches are applied.
fn map_one(position: usize, shifts: &[(usize, usize, i64)], edge: Edge) -> usize {
    let mut carried = 0i64;
    for &(start, end, delta) in shifts {
        let before = match edge {
            Edge::Start => position <= start,
            Edge::End => position < start,
        };
        if before {
            // Before this patch: nothing to its left remains, so the running
            // shift is the whole answer.
            break;
        }
        if position >= end {
            // Past this patch: carry its width change into everything after it.
            carried += delta;
            continue;
        }
        // Inside a replaced range: collapse onto the replacement's start.
        return (start as i64 + carried).max(0) as usize;
    }
    (position as i64 + carried).max(0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_are_applied_whatever_order_they_are_given_in() {
        let patches = vec![Patch::insert(5, "!"), Patch::insert(0, "y")];
        let outcome = apply("hello world", &patches, Range::new(0, 5).unwrap());
        assert_eq!(outcome.text, "yhello! world");
    }

    #[test]
    fn text_added_at_either_end_of_a_selection_is_covered_by_it() {
        let leading = [Patch::insert(0, "**")];
        let outcome = apply("hi", &leading, Range::new(0, 2).unwrap());
        assert_eq!(outcome.text, "**hi");
        assert_eq!(outcome.selection, Range::new(0, 4).unwrap());

        let trailing = [Patch::insert(2, "**")];
        let outcome = apply("hi", &trailing, Range::new(0, 2).unwrap());
        assert_eq!(outcome.text, "hi**");
        assert_eq!(outcome.selection, Range::new(0, 4).unwrap());
    }

    #[test]
    fn a_caret_before_an_insertion_stays_where_it_was() {
        let patches = [Patch::insert(2, "**")];
        let outcome = apply("hi", &patches, Range::caret(0));
        assert_eq!(outcome.selection, Range::caret(0));
    }

    #[test]
    fn a_selection_inside_a_deletion_collapses_to_where_it_lived() {
        let patches = [Patch::delete(Range::new(0, 2).unwrap())];
        let outcome = apply("**hi", &patches, Range::new(1, 4).unwrap());
        assert_eq!(outcome.text, "hi");
        // Byte 1 was inside the removed run, so it lands at the start of what
        // replaced it — here, the start of the text.
        assert_eq!(outcome.selection, Range::new(0, 2).unwrap());
    }

    #[test]
    fn a_position_at_the_end_of_a_replacement_is_past_it() {
        let patches = [Patch::delete(Range::new(0, 2).unwrap())];
        let outcome = apply("**hi", &patches, Range::new(2, 4).unwrap());
        assert_eq!(outcome.selection, Range::new(0, 2).unwrap());
    }
}
