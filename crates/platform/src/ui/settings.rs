//! The settings page, opened from the gear in the toolbar.

use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, vec2};

use crate::ui::state::{AppTheme, FONT_FAMILIES, OpenBehavior, Shell};
use crate::ui::{theme, widgets};

/// Draws the settings page.
pub fn show(shell: &mut Shell, ui: &mut egui::Ui) {
    egui::CentralPanel::no_frame()
        .frame(
            Frame::NONE
                .fill(theme::BG)
                .inner_margin(Margin::symmetric(30, 16)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(RichText::new("Settings").size(32.0).color(theme::TEXT));
                ui.add_space(16.0);

                ui.horizontal_top(|ui| {
                    let total = ui.available_width();
                    let left_width = (total * 0.60).max(260.0);
                    ui.allocate_ui_with_layout(
                        vec2(left_width, 0.0),
                        Layout::top_down(Align::Min),
                        |ui| settings_column(shell, ui),
                    );
                    ui.add_space(total * 0.04);
                    ui.allocate_ui_with_layout(
                        vec2(total * 0.34, 0.0),
                        Layout::top_down(Align::Min),
                        about_column,
                    );
                });
            });
        });
}

/// The left column: the actual settings.
fn settings_column(shell: &mut Shell, ui: &mut egui::Ui) {
    section(ui, "Appearance");
    card(ui, |ui| {
        row(
            ui,
            "App theme",
            Some("Select which app theme to display"),
            |ui| {
                dropdown(ui, "theme", &mut shell.theme);
            },
        );
    });

    ui.add_space(10.0);
    section(ui, "Text Formatting");
    card(ui, |ui| {
        row(ui, "Font", None, |ui| font_dropdown(ui, shell));
    });
    card(ui, |ui| {
        row(
            ui,
            "Word wrap",
            Some("Fit text within window by default"),
            |ui| {
                switch_with_label(ui, &mut shell.word_wrap);
            },
        );
    });
    card(ui, |ui| {
        row(ui, "Formatting", None, |ui| {
            let mut formatting = shell.word_wrap;
            switch_with_label(ui, &mut formatting);
        });
    });

    ui.add_space(10.0);
    section(ui, "Opening NoteCraft");
    card(ui, |ui| {
        row(ui, "Opening files", None, |ui| {
            dropdown(ui, "opening", &mut shell.opening);
        });
    });
}

/// The right column: the "About this app" panel.
fn about_column(ui: &mut egui::Ui) {
    ui.label(RichText::new("About this app").size(15.0).strong());
    ui.add_space(6.0);
    ui.label(RichText::new(format!("NoteCraft {}", env!("CARGO_PKG_VERSION"))).size(13.0));
    ui.label(RichText::new("© 2026 NoteCraft contributors. All rights reserved.").size(13.0));
    ui.add_space(12.0);
    for link in [
        "Software License Terms",
        "Terms of Service",
        "Privacy Statement",
        "Third-Party Software Acknowledgments",
    ] {
        ui.add_space(4.0);
        let _ = ui.link(RichText::new(link).size(13.0).color(theme::ACCENT));
    }
    ui.add_space(18.0);
    let _ = ui.button("Send feedback");
    ui.add_space(6.0);
    let _ = ui.button("Help");
}

/// A section heading.
fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(15.0).strong());
    ui.add_space(6.0);
}

/// A settings card: a rounded, slightly lighter panel.
fn card(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    Frame::NONE
        .fill(theme::CARD)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
    ui.add_space(6.0);
}

/// A card row: a title, an optional subtitle, and right-aligned control.
fn row(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    control: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(14.0).color(theme::TEXT));
            if let Some(subtitle) = subtitle {
                ui.label(RichText::new(subtitle).size(12.0).color(theme::TEXT_DIM));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            control(ui);
        });
    });
}

/// An option type a dropdown can list.
trait MenuOption: PartialEq + Copy + 'static {
    /// The option's label.
    fn title(self) -> &'static str;
    /// Every option, in menu order.
    fn options() -> &'static [Self];
}

impl MenuOption for AppTheme {
    fn title(self) -> &'static str {
        self.label()
    }
    fn options() -> &'static [Self] {
        &AppTheme::ALL
    }
}

impl MenuOption for OpenBehavior {
    fn title(self) -> &'static str {
        self.label()
    }
    fn options() -> &'static [Self] {
        &OpenBehavior::ALL
    }
}

/// A dropdown bound to a [`MenuOption`] type.
fn dropdown<T: MenuOption>(ui: &mut egui::Ui, id: &str, selected: &mut T) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected.title())
        .width(180.0)
        .show_ui(ui, |ui| {
            for option in T::options() {
                if ui
                    .selectable_label(*selected == *option, option.title())
                    .clicked()
                {
                    *selected = *option;
                    ui.close();
                }
            }
        });
}

/// The font-family dropdown.
fn font_dropdown(ui: &mut egui::Ui, shell: &mut Shell) {
    egui::ComboBox::from_id_salt("font")
        .selected_text(shell.font_family)
        .width(180.0)
        .show_ui(ui, |ui| {
            for family in FONT_FAMILIES {
                if ui
                    .selectable_label(shell.font_family == family, family)
                    .clicked()
                {
                    shell.font_family = family;
                    ui.close();
                }
            }
        });
}

/// A switch followed by its on/off label.
fn switch_with_label(ui: &mut egui::Ui, on: &mut bool) {
    let label = if *on { "On" } else { "Off" };
    ui.label(RichText::new(label).size(13.0).color(theme::TEXT_DIM));
    widgets::switch(ui, on);
}
