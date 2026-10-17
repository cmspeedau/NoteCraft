# The spelling dictionary

`words.txt` is the whole of NoteCraft's spelling knowledge. `crates/textproc/src/spell.rs`
embeds it at build time (`include_str!`), so a spell check needs no dictionary file at
run time and pulls in no dictionary crate. `Settings ▸ Spell check` and `Edit ▸ Spelling
suggestions` are the two places it reaches the screen; what "misspelled" means is written
up in `docs/ui-reference.md` §14.

## What it is

A few hundred common English words, **lowercase and whitespace-separated** (several to a
line). A `#` line is a comment and is skipped when the list is read — only tokens made
entirely of lowercase ASCII letters become entries.

Only **base forms** are listed. The engine also accepts regular inflections of a listed
word by stripping the ending back to a base — plurals, `-ed`, `-ing`, `-ly`,
comparatives, and the spelling changes English makes around them (`make`→`making`,
`run`→`running`, `baby`→`babies`) — so inflected forms are deliberately *not* written
out. The last block carries the **irregular forms** the stripping cannot reach (`sat`,
`went`, `said`, `wrote`) and the everyday function words that, left out, would sit one
edit from some listed word and be underlined in error.

## What it is not

It is not a full dictionary, and it is not meant to grow into one. A general spell
checker flags every word it does not hold; this one flags a word only when it is unknown
*and* within one edit of a word it does hold, so a flag means "this looks like a slip"
rather than "not in my list". That is what lets a list this small run without squiggling
correct-but-uncommon prose — see the limits recorded in §14.

## Where it came from

The list was authored by hand for this build, not copied from a word list. Every entry
is ordinary English, so no licence accompanies it; nothing here is reproduced from a
third-party dictionary. A word is worth adding when it appears in real prose that this
build would otherwise underline by mistake — the frequency of a word, not its
completeness in some master list, is the test for whether it belongs here.
