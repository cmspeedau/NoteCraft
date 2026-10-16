//! The dark theme, matched to the reference screenshots.
//!
//! One place for every colour and the widget style, so the chrome, editor,
//! status bar, and settings page all agree.

use egui::{Color32, Context, CornerRadius, Margin, Stroke, Vec2};

/// The editor and window background.
pub const BG: Color32 = Color32::from_rgb(0x20, 0x20, 0x20);
/// The menu bar, toolbar, title bar, and status bar background.
pub const CHROME: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
/// The background of a settings card.
pub const CARD: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
/// A card / row hover fill.
pub const ROW_HOVER: Color32 = Color32::from_rgba_premultiplied(0x3a, 0x3a, 0x3a, 0xff);
/// The generic widget hover fill (a faint white wash).
pub const HOVER: Color32 = Color32::from_rgba_premultiplied(45, 45, 45, 255);
/// The generic widget press fill.
pub const ACTIVE: Color32 = Color32::from_rgba_premultiplied(60, 60, 60, 255);
/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
/// Disabled / secondary text.
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9a, 0x9a, 0x9a);
/// Hairline borders (card edges, separators).
pub const BORDER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
/// The Windows 11 accent blue: toggles, links, the active-tab underline.
pub const ACCENT: Color32 = Color32::from_rgb(0x4c, 0xc2, 0xff);
/// The accent, dimmed for selection backgrounds.
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(0x2a, 0x6a, 0x8c, 255);

/// Installs the theme on the egui context.
pub fn install(ctx: &Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(style_mut);
}

/// Applies the NoteCraft look to a style.
fn style_mut(style: &mut egui::Style) {
    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.panel_fill = BG;
    visuals.window_fill = CHROME;
    visuals.extreme_bg_color = BG;
    visuals.text_edit_bg_color = Some(BG);
    visuals.faint_bg_color = ROW_HOVER;
    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(TEXT_DIM);
    visuals.hyperlink_color = ACCENT;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.selection.bg_fill = ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0, TEXT);

    let widgets = &mut visuals.widgets;
    for widget in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        widget.bg_stroke = Stroke::NONE;
        widget.corner_radius = CornerRadius::same(4);
    }
    widgets.noninteractive.bg_fill = CHROME;
    widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    widgets.inactive.bg_fill = CHROME;
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    widgets.hovered.bg_fill = HOVER;
    widgets.hovered.weak_bg_fill = HOVER;
    widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    widgets.active.bg_fill = ACTIVE;
    widgets.active.weak_bg_fill = ACTIVE;
    widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    widgets.open.bg_fill = HOVER;
    widgets.open.weak_bg_fill = HOVER;
    widgets.open.fg_stroke = Stroke::new(1.0, TEXT);

    style.spacing.item_spacing = Vec2::new(6.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.menu_margin = Margin::symmetric(4, 6);
    style.spacing.interact_size.y = 24.0;
}
