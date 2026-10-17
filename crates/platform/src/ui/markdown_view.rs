//! The formatted view: the document rendered from its own Markdown.
//!
//! `docs/architecture.md` §6 sets the rule this module obeys: the source is the
//! document, and a formatted view is *derived* from it. So this renders; it does
//! not edit. Everything a user types goes through the source view, which means the
//! buffer never holds two versions of the same text, and a formatted view can never
//! drift from what the file will contain.
//!
//! The rendering is as tolerant as the parser. [`runs`] drops only the delimiters of
//! emphasis the parser actually recognised, so syntax this editor does not
//! implement — `[^1]:`, raw HTML, `==highlight==`, an unterminated `**` — stays on
//! screen exactly as written, in step with the rule that nothing is silently
//! normalised (`NC-M011`). Nothing here can hide a byte the source has, and
//! switching back to the source view shows the whole document again.
//!
//! Presentation is kept apart from painting for one reason: [`runs`] and [`view_of`]
//! decide *what* is drawn and are testable without an `egui` context, while
//! [`show`] only turns those decisions into labels.

use egui::RichText;

use crate::ui::state::Tab;
use crate::ui::theme::Palette;

/// How many bytes of source indentation read as one level of nesting.
const INDENT_SPACES: usize = 2;

/// A stretch of a block's text that shares one presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Run {
    /// The text, with any emphasis delimiters removed.
    pub(crate) text: String,
    /// How it is emphasised.
    pub(crate) style: RunStyle,
}

/// How a run is emphasised.
///
/// Deliberately free of `egui` types: the split into runs is the part worth
/// testing, and it should not need a context in order to be tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RunStyle {
    /// `**strong**`
    pub(crate) strong: bool,
    /// `*emphasis*`
    pub(crate) italics: bool,
    /// `~~strikethrough~~`
    pub(crate) strike: bool,
    /// A backtick code span, drawn slightly smaller.
    pub(crate) code: bool,
    /// A link's label, drawn in the accent colour.
    pub(crate) link: bool,
}

/// How a block is drawn: its size, weight, nesting, and list marker.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BlockView {
    /// The font size as a multiple of the view's base size.
    pub(crate) scale: f32,
    /// Whether the text is drawn bold regardless of its inline emphasis.
    pub(crate) strong: bool,
    /// How many levels of indentation to draw.
    pub(crate) indent: usize,
    /// The marker drawn before the text, if the block has one.
    pub(crate) marker: Option<String>,
    /// Whether the text is drawn in the dim colour, as a blockquote is.
    pub(crate) dim: bool,
    /// Whether to keep the source's own spacing, which a pipe table needs.
    pub(crate) monospace: bool,
}

/// Splits `content` into runs, applying the inline emphasis `nc-markdown` finds.
///
/// The delimiters of a recognised span are consumed; everything else is emitted
/// verbatim, including an unpaired `**`. The parser leaves that as text, and so does
/// the view.
pub(crate) fn runs(content: &str) -> Vec<Run> {
    use nc_markdown::SpanKind;
    let found = nc_markdown::spans(content);
    let mut runs: Vec<Run> = Vec::with_capacity(found.len() * 2 + 1);
    let mut cursor = 0usize;

    for span in found {
        if span.range.start > cursor {
            runs.push(Run {
                text: content[cursor..span.range.start].to_string(),
                style: RunStyle::default(),
            });
        }
        let style = match span.kind {
            SpanKind::Strong => RunStyle {
                strong: true,
                ..Default::default()
            },
            SpanKind::Emphasis => RunStyle {
                italics: true,
                ..Default::default()
            },
            SpanKind::Strike => RunStyle {
                strike: true,
                ..Default::default()
            },
            SpanKind::InlineCode => RunStyle {
                code: true,
                ..Default::default()
            },
            SpanKind::Link | SpanKind::Image | SpanKind::AutoLink => RunStyle {
                link: true,
                ..Default::default()
            },
        };
        runs.push(Run {
            text: span.content.slice(content).to_string(),
            style,
        });
        cursor = span.range.end;
    }

    if cursor < content.len() {
        runs.push(Run {
            text: content[cursor..].to_string(),
            style: RunStyle::default(),
        });
    }
    runs
}

/// How a block of the given kind is drawn.
pub(crate) fn view_of(kind: nc_markdown::BlockKind) -> BlockView {
    use nc_markdown::{BlockKind, ListMarker};
    let plain = BlockView {
        scale: 1.0,
        strong: false,
        indent: 0,
        marker: None,
        dim: false,
        monospace: false,
    };
    match kind {
        BlockKind::Heading(level) => BlockView {
            // Six levels, six sizes, largest first, and none below the size of the
            // prose it heads.
            scale: match level {
                1 => 1.9,
                2 => 1.55,
                3 => 1.3,
                4 => 1.15,
                _ => 1.05,
            },
            strong: true,
            ..plain
        },
        BlockKind::ListItem { indent, marker } => BlockView {
            indent: indent / INDENT_SPACES,
            marker: Some(match marker {
                ListMarker::Dash | ListMarker::Plus | ListMarker::Star => "• ".to_string(),
                ListMarker::Number(number) => format!("{number}. "),
            }),
            ..plain
        },
        BlockKind::Quote => BlockView {
            indent: 1,
            dim: true,
            ..plain
        },
        BlockKind::TableRow => BlockView {
            monospace: true,
            ..plain
        },
        BlockKind::Blank | BlockKind::Paragraph => plain,
    }
}

/// Draws `tab`'s document as rendered Markdown, read-only.
///
/// `base` is the editor's own font size, so a normal line measures the same in
/// either view.
pub(crate) fn show(tab: &Tab, ui: &mut egui::Ui, base: f32, palette: Palette) {
    // Cloned, not borrowed: `Ui::style` hands back a reference into the ui, and the
    // scroll area below needs to borrow the ui mutably.
    let style = ui.style().clone();
    egui::ScrollArea::vertical()
        .id_salt("markdown_view")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for block in nc_markdown::blocks(tab.text()) {
                let view = view_of(block.kind);
                if block.is_blank() {
                    ui.add_space(base * 0.4);
                    continue;
                }
                // The nesting and the list marker go in as text rather than as
                // layout: every run is monospaced, so a level is exactly its
                // source's worth of spaces, and one block stays one wrapping
                // paragraph instead of a row of widgets.
                let lead = format!(
                    "{}{}",
                    " ".repeat(view.indent * INDENT_SPACES),
                    view.marker.as_deref().unwrap_or("")
                );
                ui.label(job_for(
                    block.content(),
                    &lead,
                    &view,
                    base,
                    &style,
                    palette,
                ));
                ui.add_space(base * 0.22);
            }
        });
}

/// The laid-out text of one block: its indentation and marker, then its runs.
///
/// One block is one job rather than one label per run because wrapping belongs to a
/// galley: a widget per run can only break *between* runs, so a heading would
/// fracture across the page wherever a marker happened to sit.
fn job_for(
    content: &str,
    lead: &str,
    view: &BlockView,
    base: f32,
    style: &egui::Style,
    palette: Palette,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    if !lead.is_empty() {
        RichText::new(lead)
            .monospace()
            .size(base)
            .color(palette.text_dim)
            .append_to(
                &mut job,
                style,
                egui::FontSelection::Default,
                egui::Align::Center,
            );
    }
    for run in runs(content) {
        rich_text(&run, view, base, palette).append_to(
            &mut job,
            style,
            egui::FontSelection::Default,
            egui::Align::Center,
        );
    }
    job
}

/// The `egui` text for one run in one block's presentation.
fn rich_text(run: &Run, view: &BlockView, base: f32, palette: Palette) -> RichText {
    // Size first, then the emphases: `egui` resolves the family variant for bold
    // and italic from the flags, so the size is set on top of whatever it picks.
    let scale = if run.style.code { 0.92 } else { view.scale };
    let mut rich = RichText::new(run.text.clone())
        .monospace()
        .size(base * scale);
    if run.style.strong || view.strong {
        rich = rich.strong();
    }
    if run.style.italics {
        rich = rich.italics();
    }
    if run.style.strike {
        rich = rich.strikethrough();
    }
    if run.style.link {
        rich.color(palette.accent).underline()
    } else if view.dim {
        rich.color(palette.text_dim)
    } else {
        rich
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The runs of `content`, as text-and-style pairs.
    fn painted(content: &str) -> Vec<(String, RunStyle)> {
        runs(content)
            .into_iter()
            .map(|run| (run.text, run.style))
            .collect()
    }

    #[test]
    fn emphasis_markers_are_drawn_as_style_not_as_characters() {
        let drawn = painted("**bold** text");
        assert_eq!(drawn.len(), 2, "{drawn:?}");
        assert_eq!(
            drawn[0],
            (
                "bold".to_string(),
                RunStyle {
                    strong: true,
                    ..Default::default()
                }
            )
        );
        assert_eq!(drawn[1], (" text".to_string(), RunStyle::default()));
    }

    #[test]
    fn every_kind_of_emphasis_the_engine_recognises_is_styled() {
        for (content, text, style) in [
            (
                "**b**",
                "b",
                RunStyle {
                    strong: true,
                    ..Default::default()
                },
            ),
            (
                "*i*",
                "i",
                RunStyle {
                    italics: true,
                    ..Default::default()
                },
            ),
            (
                "~~s~~",
                "s",
                RunStyle {
                    strike: true,
                    ..Default::default()
                },
            ),
            (
                "`c`",
                "c",
                RunStyle {
                    code: true,
                    ..Default::default()
                },
            ),
            (
                "[t](u)",
                "t",
                RunStyle {
                    link: true,
                    ..Default::default()
                },
            ),
            (
                "<https://e.g>",
                "https://e.g",
                RunStyle {
                    link: true,
                    ..Default::default()
                },
            ),
        ] {
            let drawn = painted(content);
            assert_eq!(drawn.len(), 1, "{content:?} -> {drawn:?}");
            assert_eq!(drawn[0].0, text, "{content:?}");
            assert_eq!(drawn[0].1, style, "{content:?}");
        }
    }

    #[test]
    fn syntax_the_parser_does_not_know_is_painted_exactly_as_written() {
        // The view must not hide what it cannot parse. That is the same rule the
        // serializer follows, and it is what makes switching views safe.
        for content in [
            "[^1]: a footnote definition",
            "<div class=\"note\">",
            "==highlighted==",
            "a **unclosed bold",
            "$x^2$",
            "2 * 3 = 6",
            "snake_case_word",
        ] {
            let drawn: String = runs(content).into_iter().map(|run| run.text).collect();
            assert_eq!(drawn, content, "{content:?} was changed by the view");
        }
    }

    #[test]
    fn the_only_text_the_view_drops_is_recognised_delimiters() {
        let content = "plain **bold** *ital* ~~gone~~ `code` [link](u) tail";
        let drawn: String = runs(content)
            .into_iter()
            .map(|run| run.text)
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(drawn, "plain bold ital gone code link tail");
    }

    #[test]
    fn emphasis_inside_a_heading_keeps_both_of_its_style_sources() {
        // A heading is bold because of its kind and the `*emphasis*` inside it is
        // italic because of its span. The two are decided separately and neither
        // erases the other; whether the installed font has a bold-italic monospace
        // face is `egui`'s fallback to make, not something this layer can promise.
        let drawn = painted("# Title with *emphasis*");
        let view = view_of(nc_markdown::BlockKind::Heading(1));
        let emphasis = drawn
            .iter()
            .find(|(text, _)| text == "emphasis")
            .expect("the emphasis is its own run");
        assert!(view.strong, "the block asks for bold");
        assert!(emphasis.1.italics, "the span asks for italic");
        assert!(
            !emphasis.1.strong,
            "and the run does not claim bold for itself"
        );
    }

    #[test]
    fn heading_sizes_do_not_grow_as_the_level_number_does() {
        let scales: Vec<f32> = (1..=6)
            .map(|level| view_of(nc_markdown::BlockKind::Heading(level)).scale)
            .collect();
        for pair in scales.windows(2) {
            assert!(pair[1] <= pair[0], "{scales:?}");
        }
        assert!(scales.iter().all(|scale| *scale >= 1.0), "{scales:?}");
        assert!(
            view_of(nc_markdown::BlockKind::Heading(1)).strong,
            "a heading is drawn bold"
        );
    }

    #[test]
    fn nesting_is_drawn_from_the_indent_the_source_holds() {
        use nc_markdown::{BlockKind, ListMarker};
        for (indent, levels) in [(0, 0), (2, 1), (4, 2)] {
            let view = view_of(BlockKind::ListItem {
                indent,
                marker: ListMarker::Dash,
            });
            assert_eq!(view.indent, levels, "{indent} bytes of indent");
            assert_eq!(view.marker.as_deref(), Some("• "));
        }
    }

    #[test]
    fn a_numbered_item_keeps_its_number_rather_than_a_bullet() {
        use nc_markdown::{BlockKind, ListMarker};
        let view = view_of(BlockKind::ListItem {
            indent: 0,
            marker: ListMarker::Number(3),
        });
        assert_eq!(view.marker.as_deref(), Some("3. "));
    }

    #[test]
    fn a_table_row_keeps_its_own_spacing() {
        // Pipes only line up if nothing re-flows the row.
        assert!(view_of(nc_markdown::BlockKind::TableRow).monospace);
        assert!(!view_of(nc_markdown::BlockKind::Paragraph).monospace);
        assert!(view_of(nc_markdown::BlockKind::Quote).dim);
    }
}
