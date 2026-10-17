# The editor's fonts

These four monospaced families are what `Settings ▸ Font` offers, and they are the
files that make those names mean something: the dropdown lists them, the choice picks
the face the document is drawn in, and `crates/platform/src/ui/fonts.rs` embeds the
files below.

| Family (what the dropdown shows) | File | Licence | Upstream |
| --- | --- | --- | --- |
| JetBrains Mono | `JetBrainsMono[wght].ttf` | OFL 1.1 | JetBrains — www.jetbrains.com/mono |
| IBM Plex Mono | `IBMPlexMono-Regular.ttf` | OFL 1.1 | IBM — www.github.com/IBM/plex |
| Atkinson Hyperlegible Mono | `AtkinsonHyperlegibleMono[wght].ttf` | OFL 1.1 | Braille Institute — www.github.com/googlefonts/atkinson-hyperlegible-next-mono |
| Fira Mono | `FiraMono-Regular.ttf` | OFL 1.1 | Mozilla — www.mozilla.org/projects/fira |

## Where they came from

Every file is taken from the [google/fonts](https://github.com/google/fonts)
repository at commit `47d54ca1ebb14fd864695f1d39dc734f39017c33`, unchanged, from
`ofl/jetbrainsmono/`, `ofl/ibmplexmono/`, `ofl/atkinsonhyperlegiblemono/` and
`ofl/firamono/`. The file names are the upstream ones, so a file here can be
diffed against the file there.

Two of them (`JetBrainsMono[wght]`, `AtkinsonHyperlegibleMono[wght]`) are variable
fonts: google/fonts carries no static instances of these families, and `egui`
renders such a font at its default instance, which for both is the Regular weight.

## The licence

All four are under the SIL Open Font License 1.1. The licence text each one ships
under is in the `.OFL.txt` file beside it, which is what OFL clause 2 requires when
the font is redistributed, and the copyright line at its top is the notice clause 1
asks to be preserved. The OFL permits embedding, redistributing and using the fonts
in an application; it does not permit selling the fonts on their own, and it
requires derivatives to stay under the same licence. None of these files is a
derivative — they are the upstream binaries.

The four families are monospaced on purpose. Both surfaces that draw a document are
monospaced by design — the rendered Markdown view indents with spaces, which is
exact only when a level is exactly its source's worth of them — so a proportional
family in this list would misrender one of the two views.
