//! M6's drawn surface, asserted on what the shell actually paints.
//!
//! The acceptance suite in [`crate::ui::lifecycle`] settles what the shell *does*;
//! this one settles what it *shows*, which is the half `NC-T001` and `NC-T004` are
//! written against. It renders the real shell headlessly through an `egui` context
//! at the reference's window size and reads back the painted shapes — the same
//! route `AGENT.md` §6 prescribes for UI tests.
//!
//! This narrows what "matches the reference" means, and the narrowing is worth
//! stating rather than hiding behind. `docs/ui-reference.md` describes the tab
//! strip in words (§1: one tab per document, the active tab underlined in the
//! accent colour, an unsaved document showing a dot where the close button sits)
//! and gives no coordinates. So these tests assert the *structural* claims — that
//! each document gets a tab, that exactly one tab carries the accent underline,
//! that a dirty document carries a dot and a clean one does not — and the
//! reference's own dimensions where it states them. Whether the result *looks*
//! like the captured screenshot is a judgement a shape list cannot make; that
//! comparison still needs a person looking at a running window.

use super::{draw, theme};
use crate::ui::state::{DocKind, Shell};
use egui::{Color32, Rect, Shape};

/// The tab strip's height, as `title_bar` sizes each tab.
const TAB_HEIGHT: f32 = 34.0;
/// The unsaved-changes dot's radius, as `title_bar` paints it.
const DOT_RADIUS: f32 = 3.0;

/// Paints one frame of the whole shell at the reference's window size.
fn paint(shell: &mut Shell) -> egui::FullOutput {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1180.0, 720.0),
        )),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw(shell, ui));
    output.textures_delta.clear();
    output
}

/// The filled rectangles painted this frame, with the colour they were filled in.
fn filled_rects(output: &egui::FullOutput) -> Vec<(Rect, Color32)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(shape) if shape.fill != Color32::TRANSPARENT => {
                Some((shape.rect, shape.fill))
            }
            _ => None,
        })
        .collect()
}

/// The rectangles the size of a tab — one per document in the strip.
fn tab_rects(output: &egui::FullOutput) -> Vec<Rect> {
    filled_rects(output)
        .into_iter()
        .filter(|(rect, _)| (rect.height() - TAB_HEIGHT).abs() < 0.01)
        .map(|(rect, _)| rect)
        .collect()
}

/// Underlines painted in the accent colour, which is what marks the active tab.
///
/// The colour is the point of `docs/ui-reference.md` §1's wording ("underlined in
/// the accent colour"), so it is matched rather than assumed from geometry.
fn accent_underlines(output: &egui::FullOutput) -> Vec<Rect> {
    filled_rects(output)
        .into_iter()
        .filter(|(rect, fill)| {
            *fill == theme::ACCENT && rect.height() <= 2.5 && rect.width() > 20.0
        })
        .map(|(rect, _)| rect)
        .collect()
}

/// The unsaved-changes dots: accent-coloured circles of the marker's radius inside
/// the title bar band.
///
/// Filtered by radius and band because the same shell paints other circles — the
/// account avatar and the settings toggles — and neither is a dirty marker.
fn dirty_dots(output: &egui::FullOutput) -> Vec<egui::Pos2> {
    let band = super::title_bar::HEIGHT;
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Circle(shape)
                if (shape.radius - DOT_RADIUS).abs() < 0.01
                    && shape.center.y <= band
                    && shape.fill == theme::ACCENT =>
            {
                Some(shape.center)
            }
            _ => None,
        })
        .collect()
}

/// A shell holding `count` documents, the one at `active` marked dirty.
fn shell_with_unsaved(count: usize, dirty: usize) -> Shell {
    let mut shell = Shell::new();
    while shell.tabs.len() < count {
        shell.open_tab(DocKind::Plain);
    }
    if dirty < count {
        let end = shell.tabs[dirty].text().len();
        let document = shell.tabs[dirty].document_mut();
        document.set_caret(end);
        document.insert_str("unsaved edit");
    }
    shell
}

// ---- NC-T001 — "Geometry matches the reference" ------------------------------

#[test]
fn one_tab_is_painted_for_each_open_document() {
    for count in [1, 2, 5] {
        let mut shell = Shell::new();
        while shell.tabs.len() < count {
            shell.open_tab(DocKind::Plain);
        }

        let tabs = tab_rects(&paint(&mut shell));
        assert_eq!(
            tabs.len(),
            count,
            "`docs/ui-reference.md` §1: one tab per document, {count} open"
        );
        assert!(
            tabs.windows(2)
                .all(|pair| pair[0].max.x <= pair[1].min.x + 0.01),
            "the tabs sit side by side rather than stacked or overlapping"
        );
    }
}

#[test]
fn tabs_are_drawn_the_height_the_shell_sizes_them() {
    let mut shell = Shell::new();
    shell.open_tab(DocKind::Plain);
    let tabs = tab_rects(&paint(&mut shell));
    assert_eq!(tabs.len(), 2);
    for tab in tabs {
        assert!(
            tab.width() >= 120.0 && tab.width() <= 220.0,
            "a tab keeps its stated 120..=220 width bounds, got {}",
            tab.width()
        );
    }
}

#[test]
fn exactly_one_tab_carries_the_accent_underline_and_it_is_the_active_one() {
    for active in 0..3 {
        let mut shell = Shell::new();
        while shell.tabs.len() < 3 {
            shell.open_tab(DocKind::Plain);
        }
        shell.active = active;

        let output = paint(&mut shell);
        let tabs = tab_rects(&output);
        let underlines = accent_underlines(&output);

        // Each underline is inset 6pt from the sides of the tab it marks, so a
        // tab counts as underlined when some underline lies inside its width.
        let marked: Vec<usize> = tabs
            .iter()
            .enumerate()
            .filter(|(_, tab)| {
                underlines.iter().any(|underline| {
                    underline.min.x >= tab.min.x && underline.max.x <= tab.max.x + 0.01
                })
            })
            .map(|(index, _)| index)
            .collect();

        assert_eq!(
            marked,
            vec![active],
            "`docs/ui-reference.md` §1: the active tab is underlined, and only it"
        );
    }
}

// ---- NC-T004 — "Marker shown when dirty" -------------------------------------

#[test]
fn a_clean_document_shows_no_unsaved_dot() {
    let mut shell = Shell::new();
    shell.open_tab(DocKind::Plain);

    let dots = dirty_dots(&paint(&mut shell));
    assert!(
        dots.is_empty(),
        "nothing is unsaved, so no tab should carry a dot, found {}",
        dots.len()
    );
}

#[test]
fn an_unsaved_document_shows_exactly_one_dot_on_its_own_tab() {
    let mut shell = shell_with_unsaved(3, 1);
    let output = paint(&mut shell);

    let dots = dirty_dots(&output);
    assert_eq!(dots.len(), 1, "one unsaved document, one dot");

    let tabs = tab_rects(&output);
    assert_eq!(tabs.len(), 3);
    assert!(
        tabs[1].contains(dots[0]),
        "the dot sits on the tab that is unsaved, not on another"
    );
}

#[test]
fn each_unsaved_document_shows_its_own_dot() {
    let mut shell = Shell::new();
    while shell.tabs.len() < 4 {
        shell.open_tab(DocKind::Plain);
    }
    for index in [0, 2] {
        let end = shell.tabs[index].text().len();
        let document = shell.tabs[index].document_mut();
        document.set_caret(end);
        document.insert_str("work");
    }
    let output = paint(&mut shell);

    let dots = dirty_dots(&output);
    assert_eq!(dots.len(), 2, "two unsaved documents, two dots");

    let tabs = tab_rects(&output);
    let on_first = dots.iter().any(|dot| tabs[0].contains(*dot));
    let on_third = dots.iter().any(|dot| tabs[2].contains(*dot));
    assert!(on_first && on_third, "each dot lands on its own tab");
    assert!(
        !dots
            .iter()
            .any(|dot| tabs[1].contains(*dot) || tabs[3].contains(*dot)),
        "and the clean tabs stay unmarked"
    );
}

#[test]
fn answering_the_unsaved_prompt_clears_the_dot() {
    // The marker follows the dirty flag rather than the text, so a discarded
    // edit stops being advertised once the prompt is answered.
    let mut shell = shell_with_unsaved(2, 1);
    assert_eq!(dirty_dots(&paint(&mut shell)).len(), 1);

    crate::ui::actions::close_tab(&mut shell, 1);
    crate::ui::actions::answer_unsaved_dont_save(&mut shell);

    assert_eq!(
        dirty_dots(&paint(&mut shell)).len(),
        0,
        "the closed document no longer marks anything"
    );
}
