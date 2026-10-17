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
//!
//! A frame here reports a timestamp, which it did not always do, and the difference is
//! not cosmetic: `egui` decides when a programmatic scroll has *arrived* from the clock of
//! a frame later than the one that asked, so a `RawInput` with no `time` leaves every
//! scroll request unapplied forever. Measured both ways on one request — with a timestamp
//! the view jumps to its target, without one it never moves. A frame that lies about the
//! time cannot show a caret being followed.

use super::{draw, theme};
use crate::ui::state::{AppTheme, DocKind, Shell};
use crate::ui::theme::Palette;
use egui::{Color32, Rect, Shape, Theme};

/// The tab strip's height, as `title_bar` sizes each tab.
const TAB_HEIGHT: f32 = 34.0;
/// The unsaved-changes dot's radius, as `title_bar` paints it.
const DOT_RADIUS: f32 = 3.0;

/// The window the reference was captured at, which is what every frame here paints.
const SCREEN: egui::Vec2 = egui::vec2(1180.0, 720.0);

/// Paints one frame of the whole shell at the reference's window size.
fn paint(shell: &mut Shell) -> egui::FullOutput {
    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);
    frame(&ctx, shell, None)
}

/// One frame on a context already brought up with a palette installed.
fn frame(ctx: &egui::Context, shell: &mut Shell, system: Option<Theme>) -> egui::FullOutput {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
        // What the operating system reports, which is all a shell set to follow it
        // has to go on — `egui` learns it from the platform the same way.
        system_theme: system,
        ..Default::default()
    };
    render(ctx, input, shell)
}

/// One frame on a live context with `events` delivered first: what a click or a key
/// press does to the painted surface, as opposed to what a field assignment does.
fn interact(ctx: &egui::Context, shell: &mut Shell, events: Vec<egui::Event>) -> egui::FullOutput {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
        events,
        ..Default::default()
    };
    render(ctx, input, shell)
}

fn render(ctx: &egui::Context, mut input: egui::RawInput, shell: &mut Shell) -> egui::FullOutput {
    // Every real frame carries a timestamp, and `egui` needs one to decide when an
    // animated scroll has *arrived*: a request made in one frame is applied from the
    // clock of the next, so a harness that reports no time at all leaves every
    // scroll-to-caret request stuck unapplied — measured, not guessed, because the same
    // request with a clock jumps to its target and without one never moves. Step this
    // context's own clock forward one frame at 60fps.
    if input.time.is_none() {
        input.time = Some(ctx.input(|input| input.time) + 1.0 / 60.0);
    }
    let mut output = ctx.run_ui(input, |ui| draw(shell, ui));
    output.textures_delta.clear();
    output
}

/// Paints `count` frames on a live context and returns the last one.
///
/// A scroll request is consumed at the *end* of the frame that makes it and applied at
/// the start of the one after, so the moved view is on screen on the frame after that —
/// two frames, a third of a second at 60fps, which is why nobody sees it as a delay.
/// Tests that assert a caret came into view paint that many.
fn frames(ctx: &egui::Context, shell: &mut Shell, count: usize) -> egui::FullOutput {
    let mut last = frame(ctx, shell, None);
    for _ in 1..count {
        last = frame(ctx, shell, None);
    }
    last
}

/// A click, as the windowing layer reports one: press then release, same point.
fn click_at(point: egui::Pos2) -> Vec<egui::Event> {
    [true, false]
        .iter()
        .map(|pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: *pressed,
            modifiers: egui::Modifiers::NONE,
        })
        .collect()
}

/// The palette one shell ends each of two frames on, with the system reporting
/// `systems` in turn.
///
/// Two frames on *one* context is the point: what `NC-P001` promises is that the
/// open window changes, and an open window is a context that survives from one frame
/// to the next. A fresh context per frame would only prove the arithmetic.
fn palettes_across(shell: &mut Shell, systems: [Option<Theme>; 2]) -> Vec<Palette> {
    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);
    systems
        .iter()
        .map(|system| {
            frame(&ctx, shell, *system);
            shell.palette
        })
        .collect()
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
    let accent = theme::Palette::DARK.accent;
    filled_rects(output)
        .into_iter()
        .filter(|(rect, fill)| *fill == accent && rect.height() <= 2.5 && rect.width() > 20.0)
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
    let accent = theme::Palette::DARK.accent;
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Circle(shape)
                if (shape.radius - DOT_RADIUS).abs() < 0.01
                    && shape.center.y <= band
                    && shape.fill == accent =>
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

// ---- M7: the formatted view, seen as painted text ---------------------------

/// A Markdown tab holding `text`, optionally asking for the formatted view.
///
/// The flag is set here rather than fired through `cmd.view.md_formatted` because
/// the command's behaviour is [`crate::ui::markdown`]'s business; this suite asks
/// only what the shell paints once a tab asks to be rendered.
fn rendered(text: &str, formatted: bool) -> Shell {
    let mut shell = Shell::new();
    shell.open_tab(DocKind::Markdown);
    let index = shell.active;
    let tab = &mut shell.tabs[index];
    tab.document_mut().insert_str(text);
    tab.formatted = formatted;
    shell
}

/// The text painted this frame, with the largest font size in each run.
///
/// The formatted view is nothing but text, so this is how it is observed: a galley
/// holding the words *without* their markers, at a larger size, is the evidence that
/// the view rendered the document instead of redrawing its source.
fn painted_text(output: &egui::FullOutput) -> Vec<(String, f32)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(shape) => {
                // One label is one galley with one format, so this is the size the
                // run was laid out at rather than some average over the frame.
                let size = shape
                    .galley
                    .job
                    .sections
                    .iter()
                    .map(|section| section.format.font_id.size)
                    .fold(0.0f32, f32::max);
                Some((shape.galley.text().to_owned(), size))
            }
            _ => None,
        })
        .collect()
}

/// The largest size at which `needle` appears anywhere in the frame.
fn painted_at(output: &egui::FullOutput, needle: &str) -> Option<f32> {
    let mut largest: Option<f32> = None;
    for (_, size) in painted_text(output)
        .into_iter()
        .filter(|(text, _)| text.contains(needle))
    {
        largest = Some(largest.map_or(size, |seen| seen.max(size)));
    }
    largest
}

#[test]
fn the_rendered_view_paints_a_heading_larger_than_the_source_view_does() {
    let source = paint(&mut rendered("# Heading text", false));
    let formatted = paint(&mut rendered("# Heading text", true));

    assert!(
        painted_text(&source)
            .iter()
            .any(|(text, _)| text.contains("# Heading text")),
        "the source view paints the line the file holds, markers included"
    );
    assert!(
        !painted_text(&formatted)
            .iter()
            .any(|(text, _)| text.contains("# Heading")),
        "the rendered view paints no ATX marker"
    );

    let rendered = painted_at(&formatted, "Heading text").expect("the heading is painted");
    let typed = painted_at(&source, "Heading text").expect("and in the source view too");
    assert!(
        rendered > typed,
        "a heading is drawn larger than the prose under it: {rendered} against {typed}"
    );
}

#[test]
fn the_rendered_view_paints_emphasis_as_style_rather_than_as_asterisks() {
    let formatted = paint(&mut rendered("a **marked** word", true));
    let painted: Vec<String> = painted_text(&formatted)
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    assert!(
        painted.iter().any(|text| text.contains("marked")),
        "the emphasised words are painted, got {painted:?}"
    );
    assert!(
        !painted.iter().any(|text| text.contains("**")),
        "and the markers are what made it bold, not characters on screen: {painted:?}"
    );

    let source = paint(&mut rendered("a **marked** word", false));
    assert!(
        painted_text(&source)
            .iter()
            .any(|(text, _)| text.contains("**marked**")),
        "the source view still shows the bytes a user edits"
    );
}

#[test]
fn the_rendered_view_paints_syntax_it_cannot_render_exactly_as_written() {
    // NC-M011's rule seen from the other side. A view that hid what it could not
    // parse would show the user a document that is not the one in the file, which
    // is the failure the whole loss-keeping design exists to avoid.
    for unrendered in [
        "[^1]: a footnote",
        "<div class=\"note\">",
        "==highlighted==",
        "2 * 3 = 6",
        "```",
    ] {
        let output = paint(&mut rendered(unrendered, true));
        assert!(
            painted_text(&output)
                .iter()
                .any(|(text, _)| text.contains(unrendered)),
            "{unrendered:?} vanished from the rendered view"
        );
    }
}

#[test]
fn the_rendered_view_indents_a_nested_item_it_cannot_show_otherwise() {
    // A bullet's nesting is the only thing indentation carries, so a rendered view
    // that flattened it would lose the document's structure on screen.
    let output = paint(&mut rendered("- one\n  - two\n", true));
    let painted = painted_text(&output);
    assert!(
        painted.iter().any(|(text, _)| text.contains("one"))
            && painted.iter().any(|(text, _)| text.contains("two")),
        "both items are painted, as their own words: {painted:?}"
    );

    // Where an item's ink actually starts, not where its galley is placed: the
    // nesting is painted as the leading spaces the source itself holds, so a block's
    // origin is the same for every item and only the first real glyph gives it away.
    let ink_x = |needle: &str| {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(shape) if shape.galley.text().contains(needle) => shape
                    .galley
                    .rows
                    .iter()
                    .flat_map(|placed| placed.row.glyphs.iter())
                    .find(|glyph| !glyph.chr.is_whitespace())
                    .map(|glyph| shape.pos.x + glyph.pos.x),
                _ => None,
            })
            .fold(f32::MAX, f32::min)
    };
    assert!(
        ink_x("two") > ink_x("one"),
        "the nested item is drawn to the right of its parent ({} against {})",
        ink_x("two"),
        ink_x("one")
    );
}

// ---- M8: word wrap, seen as painted -----------------------------------------

/// How the editor laid out a document: the rows its galley broke it into, and how
/// wide that galley wants to be.
fn editor_layout(output: &egui::FullOutput, needle: &str) -> (usize, f32) {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(shape) if shape.galley.text().contains(needle) => {
                Some((shape.galley.rows.len(), shape.galley.rect.width()))
            }
            _ => None,
        })
        .expect("the document is painted")
}

/// A shell holding one long line, wrapped or not.
fn long_line(wrap: bool) -> Shell {
    let mut shell = Shell::new();
    shell.word_wrap = wrap;
    let text = "wrap this line for me  ".repeat(30);
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .insert_str(&text);
    shell
}

#[test]
fn word_wrap_breaks_a_long_line_inside_the_window_and_turning_it_off_does_not() {
    // `NC-P003`'s row is about the *preference* surviving a restart, which
    // `settings.rs` settles; this settles the half M8's exit criterion adds — that
    // what survives is a behaviour and not a stored word.
    let (rows, width) = editor_layout(&paint(&mut long_line(true)), "wrap");
    assert!(
        rows > 1,
        "wrapped, a line longer than the window is broken across rows: {rows} rows"
    );
    assert!(
        width <= 1180.0,
        "and none of it runs past the window it was fitted to: {width}"
    );

    let (rows, width) = editor_layout(&paint(&mut long_line(false)), "wrap");
    assert_eq!(
        rows, 1,
        "unwrapped, the line stays one line however long it is"
    );
    assert!(
        width > 1180.0,
        "and it runs past the window, which is what turning wrapping off is for: {width}"
    );
}

// ---- The caret is kept where the user can see it ----------------------------
//
// No acceptance row asks for scrolling, and none can: the capture is a screenshot and
// cannot show a scrollbar. But every row that puts a caret somewhere — Find, Go to line,
// Select all — silently depends on the shell being able to *show* it, and for the whole
// of M2 to M8 that was not true: the text edit sat in the panel with nothing scrolling
// it, so a caret moved below the fold was painted outside the window. These tests are
// the gap's replacement, in the shape the other painted tests use.

/// A shell holding `lines` numbered lines — numbered so a painted galley can be
/// identified by a line that only ever appears once in it.
fn long_document(lines: usize) -> Shell {
    let mut shell = Shell::new();
    let text: String = (0..lines).map(|n| format!("line {n}\n")).collect();
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .insert_str(&text);
    shell
}

/// Where the caret is on screen in a painted frame.
///
/// `egui` paints the editor's galley at the text shape's position and reports a caret's
/// rect in the galley's own coordinates, so the sum is where the caret actually is in
/// the window. `needle` names the document to look at, which matters once more than one
/// text is painted in the frame.
fn caret_on_screen(output: &egui::FullOutput, needle: &str, caret: usize) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(shape) if shape.galley.text().contains(needle) => Some(
                shape.pos
                    + shape
                        .galley
                        .pos_from_cursor(egui::text::CCursor::new(caret))
                        .min
                        .to_vec2(),
            ),
            _ => None,
        })
        .expect("the document is painted")
}

/// Whether a point is inside the painted window.
fn on_screen(point: egui::Pos2) -> bool {
    Rect::from_min_size(egui::Pos2::ZERO, SCREEN).contains(point)
}

#[test]
fn a_command_that_moves_the_caret_to_the_document_end_brings_it_on_screen() {
    // The queued-selection half, which is what Find and Go to line use: the command
    // writes the editor's caret between frames, so the widget sees no change inside its
    // own frame and the shell asks for the scroll on its behalf.
    let mut shell = long_document(200);
    let end = shell.active_tab().expect("a tab").text().chars().count();
    shell.pending_selection = Some((end, end));
    assert!(
        end > 1500,
        "the test needs a document taller than the window"
    );

    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);
    // One frame first, to bring the context up. Installing the shipped faces takes effect
    // at the start of the *next* pass, so a cold first frame lays the document out in a
    // face it is about to leave, and the scroll it works out from that is short by however
    // much taller the real face is. One frame of warm-up is the difference between testing
    // "a command moved the caret" and testing the first frame of a process.
    frame(&ctx, &mut shell, None);
    let output = frames(&ctx, &mut shell, 3);

    let caret = caret_on_screen(&output, "line 120", end);
    assert!(
        on_screen(caret),
        "the caret the command moved to is outside the window at y={}",
        caret.y
    );
}

#[test]
fn a_caret_that_is_already_visible_leaves_the_view_alone() {
    // The follow-scroll has to bring a caret *into* view, not centre it: a view that
    // shifted whenever a command named a visible caret would jitter on every Find, and
    // on a short document it would move the text the user is reading for no reason.
    let mut shell = long_document(200);
    shell.pending_selection = Some((0, 0));

    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);
    let before = caret_on_screen(&frame(&ctx, &mut shell, None), "line 120", 0);
    let after = caret_on_screen(&frames(&ctx, &mut shell, 3), "line 120", 0);

    assert_eq!(
        after, before,
        "naming a caret that was already on screen moved the document"
    );
}

#[test]
fn typing_past_the_bottom_of_the_window_keeps_the_caret_in_view() {
    // This half is `egui`'s own work: a text edit asks for its caret to be kept in view
    // whenever the caret moves *inside* it, by calling `Ui::scroll_to_rect`. What was
    // missing was not the request but anything hearing it, so the test exists to keep
    // the enclosing scroll area from going missing again — driven through keys and a
    // click, the way a person moves a caret down a document.
    let mut shell = long_document(400);

    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);
    frame(&ctx, &mut shell, None);
    interact(&ctx, &mut shell, click_at(egui::pos2(400.0, 300.0)));

    let down = egui::Event::Key {
        key: egui::Key::ArrowDown,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    interact(&ctx, &mut shell, vec![down; 200]);
    let output = frames(&ctx, &mut shell, 2);

    let caret = shell.caret;
    assert!(caret > 1000, "the keys moved the caret down: {caret}");
    let at = caret_on_screen(&output, "line 120", caret);
    assert!(
        on_screen(at),
        "after {caret} characters of arrow keys the caret is off screen at y={}",
        at.y
    );
}

#[test]
fn a_document_that_grows_under_the_caret_keeps_the_caret_in_view() {
    // The other way a caret ends up off screen without anything moving it: the document
    // changes size underneath it. Zoom and the font choice both reflow every line, so the
    // row the caret sits on travels — and since the caret did not change, neither the text
    // edit nor any command has a reason to follow it. The editor watches the metrics it
    // laid the document out in and follows the caret when they move.
    let mut shell = long_document(200);
    let end = shell.active_tab().expect("a tab").text().chars().count();
    shell.pending_selection = Some((end, end));

    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);
    frame(&ctx, &mut shell, None);
    let arrived = caret_on_screen(&frames(&ctx, &mut shell, 3), "line 120", end);
    assert!(
        on_screen(arrived),
        "the caret did not arrive in view to start with"
    );

    shell.zoom = 160;
    let output = frames(&ctx, &mut shell, 3);

    let grown = caret_on_screen(&output, "line 120", end);
    assert!(
        on_screen(grown),
        "the document grew 60% under a caret at its end and left it at y={}",
        grown.y
    );
}

// ---- NC-P002 — "Dropdown shown; applies to the editor" ----------------------

/// How far apart the editor set its characters, in the document containing `needle`.
///
/// The advance, not the width of the painted run: a galley's rect is trimmed to the ink
/// of its first and last glyph, so two faces with clearly different advances can measure
/// the same width for a given sentence. Where a glyph *is* placed is the number that says
/// which face the text edit laid the document out in.
fn advance_of(output: &egui::FullOutput, needle: &str) -> f32 {
    let galley = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(shape) if shape.galley.text().contains(needle) => {
                Some(shape.galley.clone())
            }
            _ => None,
        })
        .expect("the document is painted");
    let glyphs: Vec<_> = galley
        .rows
        .iter()
        .flat_map(|row| row.glyphs.iter())
        .collect();
    let (first, last) = (
        glyphs.first().expect("a glyph"),
        glyphs.last().expect("a glyph"),
    );
    (last.pos.x - first.pos.x) / (glyphs.len() - 1) as f32
}

#[test]
fn the_family_the_settings_page_offers_is_the_face_the_editor_paints_in() {
    // The unit test in `fonts` proves the rasterizer produces different marks per family;
    // this proves the *editor surface* is the thing asking for them. A choice that reached
    // the font table but not the text edit — which is exactly how `NC-P002` sat for six
    // milestones, with the dropdown writing a field nothing read — would pass that test
    // and fail this one.
    let mut shell = Shell::new();
    shell
        .active_tab_mut()
        .expect("a tab")
        .document_mut()
        .insert_str("The quick brown fox jumps");
    let start = shell.font_family;
    assert_eq!(
        start,
        crate::ui::fonts::DEFAULT,
        "the test compares against the starting face"
    );

    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);
    // Two frames, because a face a context has not yet run cannot be measured: new font
    // definitions are taken up at the start of the next pass, so the first frame is the
    // one that asks for the face and the second is the one drawn in it. A person sees one
    // frame; the test has to wait for the same two.
    frame(&ctx, &mut shell, None);
    let before = advance_of(&frame(&ctx, &mut shell, None), "quick");

    // Atkinson Hyperlegible Mono is the accessibility family, and the one of the four
    // whose advance is not the 0.6 em the other three share, so its arrival is measurable
    // where the text is set.
    shell.font_family = "Atkinson Hyperlegible Mono";
    frame(&ctx, &mut shell, None);
    let output = frame(&ctx, &mut shell, None);

    let after = advance_of(&output, "quick");
    assert!(
        (after - before).abs() > 0.2,
        "{start} and Atkinson Hyperlegible Mono set the editor's text at {before} and \
         {after} points per character: the choice never reached the text edit"
    );
}

// ---- NC-P001 — "Dropdown shown; applies the theme" --------------------------
//
// The dropdown's presence is the easy half to read off a screenshot and the hard
// half to lose, so both are asserted here: the row is painted with the choice it
// offers, and the choice *changes what the next frame paints* — which is the part a
// settings page can silently fail at, by holding a value nothing reads.

/// Every colour a frame's filled rectangles were painted in.
fn fills(output: &egui::FullOutput) -> Vec<Color32> {
    filled_rects(output)
        .into_iter()
        .map(|(_, fill)| fill)
        .collect()
}

/// Colours that belong to one palette alone, so finding one in a frame painted in
/// the other is a widget that did not get the message.
///
/// The pairs are the ones a stale widget would show up in: the two surfaces, the two
/// text colours, the accent and the selection wash. Nothing in either palette
/// repeats a value from the other (theme.rs asserts that), so this list has no
/// overlaps to forgive.
fn only_of(palette: Palette, other: Palette) -> Vec<Color32> {
    let mine = [
        palette.bg,
        palette.chrome,
        palette.text,
        palette.text_dim,
        palette.accent,
        palette.accent_soft,
    ];
    let theirs = [
        other.bg,
        other.chrome,
        other.text,
        other.text_dim,
        other.accent,
        other.accent_soft,
    ];
    mine.into_iter()
        .filter(|colour| !theirs.contains(colour))
        .collect()
}

#[test]
fn the_settings_page_paints_the_theme_row_with_the_choice_it_offers() {
    let mut shell = Shell::new();
    shell.show_settings = true;
    let output = paint(&mut shell);
    let painted: Vec<String> = painted_text(&output)
        .into_iter()
        .map(|(text, _)| text)
        .collect();

    for needle in [
        "Appearance",
        "App theme",
        "Select which app theme to display",
        "Dark",
    ] {
        assert!(
            painted.iter().any(|text| text.contains(needle)),
            "`docs/ui-reference.md` §4 lists {needle:?} under Appearance, and it is \
             not in the frame: {painted:?}"
        );
    }
}

#[test]
fn the_theme_choice_decides_the_palette_the_frame_is_painted_in() {
    for (choice, palette, other) in [
        (AppTheme::Light, Palette::LIGHT, Palette::DARK),
        (AppTheme::Dark, Palette::DARK, Palette::LIGHT),
    ] {
        let mut shell = Shell::new();
        shell.theme = choice;
        let output = paint(&mut shell);

        assert_eq!(
            shell.palette, palette,
            "{choice:?} should leave the shell painting in its palette"
        );

        let painted = fills(&output);
        assert!(
            painted.contains(&palette.chrome),
            "{choice:?}: the chrome should be painted in this palette's chrome colour"
        );
        assert!(
            painted.contains(&palette.bg),
            "{choice:?}: the page under the editor should be this palette's page colour"
        );

        let stale = only_of(other, palette);
        let leftover: Vec<Color32> = painted
            .iter()
            .filter(|fill| stale.contains(fill))
            .copied()
            .collect();
        assert!(
            leftover.is_empty(),
            "{choice:?}: {leftover:?} belong to the other palette — something is still \
             painting with it"
        );
    }
}

#[test]
fn switching_the_theme_recolours_the_open_window_from_the_next_frame() {
    // The criterion is "applies the theme", which is about the window that is
    // already up, not about the next start. So this is one shell, one context, and
    // a choice changed between its frames.
    let mut shell = Shell::new();
    let ctx = egui::Context::default();
    theme::install(&ctx, shell.palette);

    frame(&ctx, &mut shell, None);
    assert_eq!(
        shell.palette,
        Palette::DARK,
        "it starts in the captured dark look"
    );

    shell.theme = AppTheme::Light;
    let output = frame(&ctx, &mut shell, None);
    assert_eq!(
        shell.palette,
        Palette::LIGHT,
        "the choice is obeyed at once"
    );
    assert!(
        fills(&output).contains(&Palette::LIGHT.chrome),
        "and the chrome the user is looking at is what changed"
    );

    shell.theme = AppTheme::Dark;
    frame(&ctx, &mut shell, None);
    assert_eq!(
        shell.palette,
        Palette::DARK,
        "and it goes back the same way"
    );
}

#[test]
fn following_the_system_goes_light_when_the_system_does_and_back_when_it_does() {
    let mut shell = Shell::new();
    shell.theme = AppTheme::System;

    let seen = palettes_across(&mut shell, [Some(Theme::Light), Some(Theme::Dark)]);
    assert_eq!(
        seen,
        vec![Palette::LIGHT, Palette::DARK],
        "a shell set to follow the system moves with it, in both directions"
    );
}

#[test]
fn following_the_system_keeps_the_captured_look_until_the_system_says_anything() {
    // Headless, or on a platform that reports nothing: "use the system setting"
    // cannot be honoured, and guessing light would be a visible surprise, so the
    // look the reference was captured in is the one that stays.
    let mut shell = Shell::new();
    shell.theme = AppTheme::System;

    let seen = palettes_across(&mut shell, [None, None]);
    assert_eq!(seen, vec![Palette::DARK, Palette::DARK]);
}

#[test]
fn an_explicit_choice_holds_still_while_the_system_moves() {
    for (choice, expected) in [
        (AppTheme::Light, Palette::LIGHT),
        (AppTheme::Dark, Palette::DARK),
    ] {
        let mut shell = Shell::new();
        shell.theme = choice;
        let seen = palettes_across(&mut shell, [Some(Theme::Dark), Some(Theme::Light)]);
        assert_eq!(
            seen,
            vec![expected, expected],
            "{choice:?} means {expected:?} even as the system changes underneath it"
        );
    }
}
