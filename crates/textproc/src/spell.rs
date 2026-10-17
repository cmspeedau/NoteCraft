//! Spell checking: which words are not the ones this build knows, and what to
//! offer instead (`NC-P004`).
//!
//! Nothing here draws or replaces anything — like [`crate::prompt`], the module
//! answers questions about a text and leaves the acting to the caller. It is the
//! shell that underlines a word and that swaps in the suggestion the user picks.
//!
//! What a *misspelling* means here is a design decision the reference capture
//! does not make (`docs/ui-reference.md`, spell check). A general spell checker
//! flags every word its dictionary does not hold, which needs a dictionary of
//! tens of thousands of inflected forms and an affix engine to keep up. This
//! build carries a small curated list of base words instead, so a flag means
//! something narrower and far less noisy: a word that is **not** in the list *and*
//! is a single edit (an insertion, deletion, substitution, or a transposition of
//! two neighbours) away from one that is. That is what a slip of the fingers
//! looks like — `teh`, `recieve`, `wrod` — while a correct word the list has
//! never met (`zebra`) is left alone rather than squiggled in error. Suggestions
//! widen that radius by one edit, so a correction is offered for two-edit slips
//! too once one has been noticed.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::OnceLock;

/// The words this build knows, embedded so a spell check needs no dictionary
/// file at run time and no dependency (`assets/words.txt`).
const WORDS: &str = include_str!("../assets/words.txt");

/// The most edits a word may be from a known one and still be called a
/// misspelling. One edit is a slip of the fingers; more is usually a different
/// word entirely, and flagging those would squiggle half of any unusual prose.
const FLAG_RADIUS: usize = 1;

/// How far to look when offering corrections — one edit further than the flag
/// radius, so a word noticed as a slip still has suggestions when its true
/// spelling is two edits off (a doubled letter and a wrong vowel, say).
const SUGGEST_RADIUS: usize = 2;

/// The dictionary: the known words, and a way to find the near ones quickly.
struct Dictionary {
    /// Every known word, lowercased, for the "do we know this word" test.
    known: BTreeSet<&'static str>,
    /// Known words bucketed by character length, each with its position in the
    /// list. An edit of one changes the length by at most one, so a near-miss
    /// scan touches only the buckets within the radius of the suspect word's
    /// length, not the whole list; the list position ranks suggestions so the
    /// commoner word (written earlier) is offered first.
    by_len: BTreeMap<usize, Vec<(usize, &'static str)>>,
}

impl Dictionary {
    fn build() -> Self {
        // The file's `#` comment lines are prose, not words: keep only the
        // lowercase-alphabetic entries and rank them by their order in the list.
        let entries: Vec<&'static str> = WORDS
            .split_whitespace()
            .filter(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase()))
            .collect();
        let known: BTreeSet<&'static str> = entries.iter().copied().collect();
        let mut by_len: BTreeMap<usize, Vec<(usize, &'static str)>> = BTreeMap::new();
        for (rank, word) in entries.into_iter().enumerate() {
            by_len
                .entry(word.chars().count())
                .or_default()
                .push((rank, word));
        }
        Self { known, by_len }
    }

    /// Whether `word` (already lowercased) is a word we know — the exact word, or
    /// a regular inflection of one. The list holds base forms, so plurals,
    /// `-ed`/`-ing`, `-ly`, and comparatives are recognised by stripping the
    /// ending back to a listed base. The rules are generous on purpose: a false
    /// "known" hides a misspelling, the kinder error than a correct word flagged
    /// red.
    fn knows(&self, word: &str) -> bool {
        if self.known.contains(word) {
            return true;
        }
        for (suffix, restore) in [
            ("ies", "y"),  // babies -> baby
            ("ied", "y"),  // carried -> carry
            ("ying", "y"), // carrying -> carry
        ] {
            if let Some(stem) = word.strip_suffix(suffix) {
                if self.known.contains(format!("{stem}{restore}").as_str()) {
                    return true;
                }
            }
        }
        for suffix in ["es", "s", "ed", "ing", "ly", "er", "est"] {
            if let Some(stem) = word.strip_suffix(suffix) {
                if self.is_inflected(stem, suffix) {
                    return true;
                }
            }
        }
        false
    }

    /// Whether `stem` (a word with `suffix` taken off) is a real base form,
    /// allowing for the spelling changes English makes when it adds an ending: a
    /// dropped silent *e* (`make` + `ing` = `making`) and a doubled final
    /// consonant (`run` + `ing` = `running`).
    fn is_inflected(&self, stem: &str, suffix: &str) -> bool {
        if stem.is_empty() {
            return false;
        }
        if self.known.contains(stem) {
            return true;
        }
        // A base ending in a silent e loses it before -ing/-ed/-er/-est/-es.
        if matches!(suffix, "ing" | "ed" | "er" | "est" | "es")
            && self.known.contains(format!("{stem}e").as_str())
        {
            return true;
        }
        // A short base doubles its final consonant before those same endings.
        let bytes = stem.as_bytes();
        let doubled = bytes.len() >= 2
            && bytes[bytes.len() - 1] == bytes[bytes.len() - 2]
            && is_consonant(bytes[bytes.len() - 1]);
        if matches!(suffix, "ing" | "ed" | "er" | "est") && doubled {
            let single = &stem[..stem.len() - 1];
            if self.known.contains(single) || self.known.contains(format!("{single}e").as_str()) {
                return true;
            }
        }
        false
    }

    /// The known words within `radius` edits of `word` (lowercased), closest
    /// first, then earliest in the list — so the commoner of two equally-near
    /// words is offered before the rarer one.
    fn near(&self, word: &str, radius: usize) -> Vec<&'static str> {
        let length = word.chars().count();
        let mut found: Vec<(usize, usize, &'static str)> = Vec::new();
        for len in length.saturating_sub(radius)..=length + radius {
            let Some(bucket) = self.by_len.get(&len) else {
                continue;
            };
            for (rank, candidate) in bucket {
                let Some(distance) = within(candidate, word, radius) else {
                    continue;
                };
                found.push((distance, *rank, *candidate));
            }
        }
        found.sort();
        found.into_iter().map(|(_, _, word)| word).collect()
    }
}

/// The one dictionary, built on first use and shared.
fn dictionary() -> &'static Dictionary {
    static DICTIONARY: OnceLock<Dictionary> = OnceLock::new();
    DICTIONARY.get_or_init(Dictionary::build)
}

/// Whether `word` is a word this build knows, in any case.
pub fn is_known(word: &str) -> bool {
    dictionary().knows(&word.to_lowercase())
}

/// The byte ranges in `text` of the words to underline: unknown, but a single
/// edit from a known one. In order, as byte offsets into `text`.
pub fn misspellings(text: &str) -> Vec<Range<usize>> {
    words(text)
        .into_iter()
        .filter(|(_, word)| suspect(word) && !is_known(word) && near_one(word))
        .map(|(range, _)| range)
        .collect()
}

/// The known word `word` most likely meant, best first, capped at `limit`.
pub fn suggestions(word: &str, limit: usize) -> Vec<String> {
    dictionary()
        .near(&word.to_lowercase(), SUGGEST_RADIUS)
        .into_iter()
        .take(limit)
        .map(str::to_owned)
        .collect()
}

/// The misspelled word at `byte` — the one whose range contains it, so a caret
/// inside a word or resting just after it finds it. `None` when `byte` sits in a
/// clean word or in the space between words.
pub fn misspelled_word(text: &str, byte: usize) -> Option<Range<usize>> {
    misspellings(text)
        .into_iter()
        .find(|range| range.start <= byte && byte <= range.end)
}

/// Whether a token is a candidate for checking: long enough to mean something,
/// and not an all-caps acronym like `HTML` that a list of ordinary words is not
/// expected to hold.
fn suspect(word: &str) -> bool {
    let letters = word.chars().filter(|c| c.is_alphabetic()).count();
    if letters < 2 {
        return false;
    }
    let not_lowercase = word
        .chars()
        .filter(|c| c.is_alphabetic() && !c.is_lowercase())
        .count();
    not_lowercase != letters // every letter uppercase -> an acronym, skip
}

/// Whether a known word lies within the flag radius of `word` — what makes a
/// suspect word worth flagging rather than an unusual correct one.
fn near_one(word: &str) -> bool {
    !dictionary()
        .near(&word.to_lowercase(), FLAG_RADIUS)
        .is_empty()
}

/// The alphabetic tokens of `text`, each with its byte range: a run of letters,
/// allowing an apostrophe *between* letters so `don't` and `it's` stay one word.
fn words(text: &str) -> Vec<(Range<usize>, &str)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].1.is_alphabetic() {
            i += 1;
            continue;
        }
        let start = chars[i].0;
        let mut last = i; // index (in chars) of the token's final included char
        let mut j = i + 1;
        while j < chars.len() {
            let ch = chars[j].1;
            if ch.is_alphabetic() {
                last = j;
                j += 1;
            } else if (ch == '\'' || ch == '\u{2019}') && apostrophe_between(&chars, j) {
                last = j + 1; // include the apostrophe and the letter after it
                j += 2;
            } else {
                break;
            }
        }
        let (last_byte, last_char) = chars[last];
        let end = last_byte + last_char.len_utf8();
        found.push((start..end, &text[start..end]));
        i = j;
    }
    found
}

/// Whether the apostrophe at `chars[j]` sits between two letters.
fn apostrophe_between(chars: &[(usize, char)], j: usize) -> bool {
    let before = j > 0 && chars[j - 1].1.is_alphabetic();
    let after = chars.get(j + 1).is_some_and(|(_, c)| c.is_alphabetic());
    before && after
}

/// The Damerau-Levenshtein (optimal string alignment) distance between two
/// words, or `None` once it is clear it exceeds `max` — the early exit that keeps
/// a scan of the dictionary cheap. A transposition of two adjacent characters
/// counts as one edit, which is what a swapped pair of fingers (`teh`) costs.
fn within(a: &str, b: &str, max: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let (m, n) = (a.len(), b.len());
    let mut d = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    (0..=n).for_each(|j| d[0][j] = j);
    for i in 1..=m {
        let mut row_best = usize::MAX;
        for j in 1..=n {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut value = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                value = value.min(d[i - 2][j - 2] + cost);
            }
            d[i][j] = value;
            row_best = row_best.min(value);
        }
        if row_best > max {
            return None;
        }
    }
    (d[m][n] <= max).then_some(d[m][n])
}

/// Whether a byte is an ASCII consonant, for the doubled-consonant rule.
fn is_consonant(byte: u8) -> bool {
    let lower = byte.to_ascii_lowercase();
    lower.is_ascii_alphabetic() && !matches!(lower, b'a' | b'e' | b'i' | b'o' | b'u')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flagged(text: &str) -> Vec<String> {
        misspellings(text)
            .into_iter()
            .map(|range| text[range].to_owned())
            .collect()
    }

    #[test]
    fn a_word_and_its_regular_inflections_are_known() {
        for word in ["word", "words", "worded", "wording"] {
            assert!(is_known(word), "{word}");
        }
        assert!(is_known("making")); // silent e
        assert!(is_known("running")); // doubled consonant
        assert!(is_known("stopped"));
        assert!(is_known("bigger"));
        assert!(is_known("babies")); // -ies -> -y
        assert!(is_known("carried")); // -ied -> -y
    }

    #[test]
    fn a_slip_one_edit_from_a_known_word_is_flagged() {
        assert_eq!(flagged("a teh cat"), vec!["teh"]);
        assert_eq!(flagged("plese fix"), vec!["plese"]);
        assert_eq!(flagged("the wrod"), vec!["wrod"]);
    }

    #[test]
    fn a_correct_word_the_list_has_never_met_is_left_alone() {
        // A small dictionary would light up under an uncommon-but-correct word if
        // a flag meant merely "unknown". It does not.
        assert!(flagged("a zebra quokka").is_empty());
    }

    #[test]
    fn known_words_are_never_flagged_whatever_their_case() {
        assert!(flagged("The Word Wrap").is_empty());
        assert!(flagged("document editor window").is_empty());
    }

    #[test]
    fn an_acronym_is_not_checked() {
        assert!(flagged("the HTML spec").is_empty(), "all-caps is skipped");
    }

    #[test]
    fn a_word_with_an_apostrophe_stays_one_word() {
        assert!(flagged("i don't know").is_empty());
    }

    #[test]
    fn ranges_point_at_the_flagged_word_in_the_text() {
        let text = "two teh cats";
        let ranges = misspellings(text);
        assert_eq!(ranges.len(), 1);
        assert_eq!(&text[ranges[0].clone()], "teh");
    }

    #[test]
    fn suggestions_are_closest_first_and_capped() {
        let for_teh = suggestions("teh", 3);
        assert!(for_teh.iter().any(|w| w == "the"), "got {for_teh:?}");
        assert!(for_teh.len() <= 3);
        // The ordering that matters: nearer words first. "to"/"ten"-class two-edit
        // words may follow, but no one-edit word comes after a two-edit one.
        let distances: Vec<usize> = for_teh
            .iter()
            .map(|w| within(w, "teh", 2).unwrap())
            .collect();
        assert!(
            distances.windows(2).all(|pair| pair[0] <= pair[1]),
            "got {for_teh:?}"
        );
    }

    #[test]
    fn suggestions_reach_a_two_edit_spelling() {
        // "wrods" is two edits from the listed base "word": not flagged (only
        // one-edit slips are), but a correction is still offered for it.
        assert!(
            misspellings("wrods").is_empty(),
            "a two-edit slip is not flagged"
        );
        assert!(suggestions("wrods", 8).iter().any(|w| w == "word"));
    }

    #[test]
    fn the_word_at_a_byte_is_the_misspelling_under_it() {
        let text = "say teh now";
        let inside = text.find("eh").expect("eh");
        let range = misspelled_word(text, inside).expect("caret inside a flagged word");
        assert_eq!(&text[range], "teh");
        assert!(misspelled_word(text, 1).is_none(), "inside a correct word");
    }

    #[test]
    fn the_edit_distance_counts_a_transposition_as_one() {
        assert_eq!(within("teh", "the", 1), Some(1));
        assert_eq!(within("a", "a", 2), Some(0));
        assert_eq!(within("abcd", "ab", 1), None, "two deletions is beyond one");
    }
}
