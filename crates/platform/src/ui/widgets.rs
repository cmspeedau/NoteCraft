//! Small shared widgets: the pill switch and the flat icon button.

use crate::ui::theme;

/// A Windows 11 style pill switch. Returns the interaction response; the
/// response reports `changed` when the value was toggled.
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(40.0, 20.0), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    let radius = rect.height() / 2.0;
    let track = if *on {
        theme::ACCENT
    } else if response.hovered() {
        theme::ACTIVE
    } else {
        theme::BORDER
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(radius as u8), track);

    let knob_x = if *on {
        rect.right() - radius
    } else {
        rect.left() + radius
    };
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        radius - 2.5,
        egui::Color32::WHITE,
    );
    response
}

/// A flat icon button (no frame until hovered). `glyph` is a single symbol.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str) -> egui::Response {
    let button =
        egui::Button::new(egui::RichText::new(glyph).size(15.0)).min_size(egui::vec2(30.0, 26.0));
    ui.add(button)
}

/// A toolbar button with a styled text label (used for `B`, `I`, `S`, `Aa`).
pub fn rich_button(ui: &mut egui::Ui, text: egui::RichText) -> egui::Response {
    ui.add(egui::Button::new(text).min_size(egui::vec2(30.0, 26.0)))
}
