//! The palettes and the widget style, matched to the reference screenshots.
//!
//! One place for every colour, so the chrome, editor, status bar, and settings
//! page all agree.
//!
//! The colours are *fields of a [`Palette`]* rather than module constants, and the
//! reason is a failure mode rather than a style preference: `Settings ▸ App theme`
//! can now pick a light palette, and a leftover `theme::TEXT` constant would
//! compile and run forever — one widget quietly staying dark in a light window, in
//! a build whose tests paint the surfaces they care about and not every pixel of
//! chrome. Deleting the constants is what turns "did I convert every site" into a
//! question the compiler answers.
//!
//! The dark palette is byte for byte the colours the reference was captured in.
//! The light palette has no capture to copy, so it is derived by keeping the
//! *relationships* the dark one has — content surfaces lighter than the chrome,
//! cards the same fill as the chrome, borders a step beyond the chrome, text one
//! step off the extreme — with the accent taken from Windows 11's light-mode blue.
//! `docs/ui-reference.md` §4 records it as a decision.

use egui::{Color32, Context, CornerRadius, Margin, Stroke, Theme, Vec2};

use crate::ui::state::AppTheme;

/// Every colour the shell paints.
///
/// `Copy`: it is twelve colours, and threading it by value means a helper that
/// needs one of them cannot be handed a half-updated window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The editor and window background, and the active tab's fill.
    pub bg: Color32,
    /// The menu bar, toolbar, title bar, and status bar background.
    pub chrome: Color32,
    /// The background of a settings card.
    pub card: Color32,
    /// A card / row hover fill, and the inactive tab's hover.
    pub row_hover: Color32,
    /// The generic widget hover fill.
    pub hover: Color32,
    /// The generic widget press fill.
    pub active: Color32,
    /// Primary text.
    pub text: Color32,
    /// Disabled / secondary text.
    pub text_dim: Color32,
    /// Hairline borders (card edges, separators) and the account avatar.
    pub border: Color32,
    /// The track of a switch in its off state.
    ///
    /// Its own colour rather than [`Palette::border`] because the knob stays white
    /// in both palettes: the hairline grey a light border needs for a card edge is
    /// too faint to read behind a white knob.
    pub switch_off: Color32,
    /// The accent: toggles, links, the active-tab underline.
    pub accent: Color32,
    /// The accent, dimmed for selection backgrounds.
    pub accent_soft: Color32,
    /// Warning text: a dialog's error line.
    pub danger: Color32,
    /// Whether this is a dark palette, which `egui` needs for the parts it draws
    /// itself (shadows, scroll bars, the text cursor).
    pub dark: bool,
}

impl Palette {
    /// The dark palette, matched to the reference screenshots.
    pub const DARK: Palette = Palette {
        bg: Color32::from_rgb(0x20, 0x20, 0x20),
        chrome: Color32::from_rgb(0x2b, 0x2b, 0x2b),
        card: Color32::from_rgb(0x2b, 0x2b, 0x2b),
        row_hover: Color32::from_rgb(0x3a, 0x3a, 0x3a),
        hover: Color32::from_rgba_premultiplied(45, 45, 45, 255),
        active: Color32::from_rgba_premultiplied(60, 60, 60, 255),
        text: Color32::from_rgb(0xf0, 0xf0, 0xf0),
        text_dim: Color32::from_rgb(0x9a, 0x9a, 0x9a),
        border: Color32::from_rgb(0x3a, 0x3a, 0x3a),
        switch_off: Color32::from_rgb(0x3a, 0x3a, 0x3a),
        accent: Color32::from_rgb(0x4c, 0xc2, 0xff),
        accent_soft: Color32::from_rgba_premultiplied(0x2a, 0x6a, 0x8c, 255),
        danger: Color32::from_rgb(0xff, 0x8a, 0x80),
        dark: true,
    };

    /// The light palette: the same relationships, read off a white page.
    pub const LIGHT: Palette = Palette {
        bg: Color32::from_rgb(0xff, 0xff, 0xff),
        chrome: Color32::from_rgb(0xf3, 0xf3, 0xf3),
        card: Color32::from_rgb(0xf3, 0xf3, 0xf3),
        row_hover: Color32::from_rgb(0xea, 0xea, 0xea),
        hover: Color32::from_rgba_premultiplied(0, 0, 0, 16),
        active: Color32::from_rgba_premultiplied(0, 0, 0, 30),
        text: Color32::from_rgb(0x1a, 0x1a, 0x1a),
        text_dim: Color32::from_rgb(0x6a, 0x6a, 0x6a),
        border: Color32::from_rgb(0xd9, 0xd9, 0xd9),
        switch_off: Color32::from_rgb(0x8a, 0x8a, 0x8a),
        accent: Color32::from_rgb(0x00, 0x5f, 0xb8),
        accent_soft: Color32::from_rgb(0xcc, 0xe4, 0xf7),
        danger: Color32::from_rgb(0xc5, 0x0f, 0x1f),
        dark: false,
    };

    /// The palette for a chosen theme, given the one the system reports.
    ///
    /// Only [`AppTheme::System`] consults the system; an explicit Light or Dark
    /// means exactly what it says, which is the difference the reference's three
    /// choices exist to express.
    pub fn for_choice(choice: AppTheme, system: Option<Theme>) -> Palette {
        match choice {
            AppTheme::Light => Self::LIGHT,
            AppTheme::Dark => Self::DARK,
            AppTheme::System => match system {
                Some(Theme::Light) => Self::LIGHT,
                // A dark system and no answer at all both keep the dark palette:
                // "follow the system" cannot be rendered until the platform says
                // something, and the look to hold it at is the captured one.
                Some(Theme::Dark) | None => Self::DARK,
            },
        }
    }
}

/// The `dark` flag is not decoration — it is what tells `egui` which of its own
/// parts (shadows, scroll bars, the text cursor) to draw for a dark surface — so the
/// two palettes are not allowed to agree about it. Checked at compile time because it
/// is a property of two constants, not of anything a test run could vary.
const _: () = assert!(Palette::DARK.dark && !Palette::LIGHT.dark);

/// Installs `palette` on the context, so `egui`'s own parts agree with the
/// shell's own painting.
///
/// Frames go through [`apply`], which knows what is already installed; this is for
/// the one moment that does not apply — bringing a window up before its first frame.
pub fn install(ctx: &Context, palette: Palette) {
    ctx.set_theme(if palette.dark {
        Theme::Dark
    } else {
        Theme::Light
    });
    let style = |style: &mut egui::Style| style_mut(style, palette);
    ctx.all_styles_mut(style);
}

/// The key the installed palette is remembered under, per `egui` context.
fn installed_key() -> egui::Id {
    egui::Id::new("notecraft-installed-palette")
}

/// Brings the context's style in line with what `choice` asks for and returns the
/// palette the window should paint this frame with.
///
/// The comparison is against what the *context* was last given, not against what
/// this window's shell remembers, and that distinction is the whole point. Every
/// window of an `eframe` app shares one `egui` context and therefore one style, so
/// a second window that wants another palette changes the style the first one
/// draws with. A shell-local check would then read "my palette is already the
/// current one", skip the install, and paint a light frame in a window set to dark.
/// Asking the context instead means each window installs the palette it wants at the
/// start of its own drawing, and two windows with different choices each get their
/// own colours.
///
/// Nothing here asks for a repaint: a theme change always arrives with the click
/// that made it, and asking on every frame in which two windows disagree would ask
/// forever.
pub fn apply(ctx: &Context, choice: AppTheme) -> Palette {
    let wanted = Palette::for_choice(choice, ctx.system_theme());
    let key = installed_key();
    let installed = ctx.data(|data| data.get_temp::<Palette>(key));
    if installed != Some(wanted) {
        install(ctx, wanted);
        ctx.data_mut(|data| data.insert_temp(key, wanted));
    }
    wanted
}

/// Applies the NoteCraft look to a style.
fn style_mut(style: &mut egui::Style, palette: Palette) {
    let visuals = &mut style.visuals;
    visuals.dark_mode = palette.dark;
    visuals.panel_fill = palette.bg;
    visuals.window_fill = palette.chrome;
    visuals.extreme_bg_color = palette.bg;
    visuals.text_edit_bg_color = Some(palette.bg);
    visuals.faint_bg_color = palette.row_hover;
    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.text_dim);
    visuals.hyperlink_color = palette.accent;
    visuals.window_stroke = Stroke::new(1.0, palette.border);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, palette.text);

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
    widgets.noninteractive.bg_fill = palette.chrome;
    widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_dim);
    widgets.inactive.bg_fill = palette.chrome;
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
    widgets.hovered.bg_fill = palette.hover;
    widgets.hovered.weak_bg_fill = palette.hover;
    widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);
    widgets.active.bg_fill = palette.active;
    widgets.active.weak_bg_fill = palette.active;
    widgets.active.fg_stroke = Stroke::new(1.0, palette.text);
    widgets.open.bg_fill = palette.hover;
    widgets.open.weak_bg_fill = palette.hover;
    widgets.open.fg_stroke = Stroke::new(1.0, palette.text);

    style.spacing.item_spacing = Vec2::new(6.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.menu_margin = Margin::symmetric(4, 6);
    style.spacing.interact_size.y = 24.0;

    // A caret the shell moves — Find, Go to line, Select all — is *asked* to be brought
    // on screen, and `egui` glides that request over a tenth of a second by default. In
    // a text editor the glide misreports the command: the match was found the moment the
    // key went down, and while the view travels the caret is somewhere in between, which
    // reads as a command that did nothing. So the scroll snaps.
    style.scroll_animation = egui::style::ScrollAnimation::none();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every choice against every system answer, so the resolution table is on the
    /// record — and so an explicit choice is shown not to depend on the system at
    /// all, which is the whole point of offering three.
    #[test]
    fn an_explicit_choice_ignores_the_system_and_a_following_choice_obeys_it() {
        for system in [Some(Theme::Light), Some(Theme::Dark), None] {
            assert_eq!(
                Palette::for_choice(AppTheme::Light, system),
                Palette::LIGHT,
                "Light means light whatever the system says ({system:?})"
            );
            assert_eq!(
                Palette::for_choice(AppTheme::Dark, system),
                Palette::DARK,
                "Dark means dark whatever the system says ({system:?})"
            );
        }

        assert_eq!(
            Palette::for_choice(AppTheme::System, Some(Theme::Light)),
            Palette::LIGHT,
            "following a light system goes light"
        );
        assert_eq!(
            Palette::for_choice(AppTheme::System, Some(Theme::Dark)),
            Palette::DARK,
            "following a dark system goes dark"
        );
        assert_eq!(
            Palette::for_choice(AppTheme::System, None),
            Palette::DARK,
            "no answer from the system keeps the captured look"
        );
    }

    #[test]
    fn the_two_palettes_differ_everywhere_colour_is_the_point() {
        // Two palettes that agree on a colour are a widget that cannot show the
        // difference between them, so the pairs worth differing are named.
        let pairs = [
            ("page", Palette::DARK.bg, Palette::LIGHT.bg),
            ("chrome", Palette::DARK.chrome, Palette::LIGHT.chrome),
            ("text", Palette::DARK.text, Palette::LIGHT.text),
            ("accent", Palette::DARK.accent, Palette::LIGHT.accent),
            ("danger", Palette::DARK.danger, Palette::LIGHT.danger),
            (
                "selection",
                Palette::DARK.accent_soft,
                Palette::LIGHT.accent_soft,
            ),
        ];
        for (label, dark, light) in pairs {
            assert_ne!(dark, light, "{label} must read differently in each theme");
        }
    }

    #[test]
    fn each_palette_keeps_text_off_its_own_page() {
        // A palette where text sits at the same brightness as the page it lands on
        // is unreadable, and the check is cheap enough to keep.
        assert!(luminance(Palette::DARK.text) - luminance(Palette::DARK.bg) > 0.5);
        assert!(luminance(Palette::LIGHT.bg) - luminance(Palette::LIGHT.text) > 0.5);
        assert!(luminance(Palette::DARK.text) - luminance(Palette::DARK.chrome) > 0.3);
        assert!(luminance(Palette::LIGHT.chrome) - luminance(Palette::LIGHT.text) > 0.3);
    }

    /// Rough relative luminance of a colour, 0 (black) to 1 (white).
    fn luminance(colour: Color32) -> f32 {
        let [r, g, b, _] = colour.to_normalized_gamma_f32();
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}
