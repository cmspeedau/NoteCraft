//! The settings page, opened from the gear in the toolbar.

use egui::accesskit::Role;
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, WidgetType, vec2};

use crate::ui::fonts::FAMILIES;
use crate::ui::state::{AppTheme, OpenBehavior, Shell};
use crate::ui::{a11y, theme::Palette, widgets};

/// Draws the settings page.
pub fn show(shell: &mut Shell, ui: &mut egui::Ui) {
    egui::CentralPanel::no_frame()
        .frame(
            Frame::NONE
                .fill(shell.palette.bg)
                .inner_margin(Margin::symmetric(30, 16)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let palette = shell.palette;
                ui.label(RichText::new("Settings").size(32.0).color(palette.text));
                ui.add_space(16.0);

                ui.horizontal_top(|ui| {
                    let total = ui.available_width();
                    let left_width = (total * 0.60).max(260.0);
                    ui.allocate_ui_with_layout(
                        vec2(left_width, 0.0),
                        Layout::top_down(Align::Min),
                        |ui| settings_column(shell, ui, palette),
                    );
                    ui.add_space(total * 0.04);
                    ui.allocate_ui_with_layout(
                        vec2(total * 0.34, 0.0),
                        Layout::top_down(Align::Min),
                        |ui| about_column(ui, palette),
                    );
                });
            });
        });
}

/// The left column: the actual settings.
fn settings_column(shell: &mut Shell, ui: &mut egui::Ui, palette: Palette) {
    section(ui, "Appearance");
    card(ui, palette, |ui| {
        row(
            ui,
            palette,
            "App theme",
            Some("Select which app theme to display"),
            |ui| {
                dropdown(ui, "theme", "App theme", &mut shell.theme);
            },
        );
    });

    ui.add_space(10.0);
    section(ui, "Text Formatting");
    card(ui, palette, |ui| {
        row(ui, palette, "Font", None, |ui| font_dropdown(ui, shell));
    });
    card(ui, palette, |ui| {
        row(
            ui,
            palette,
            "Word wrap",
            Some("Fit text within window by default"),
            |ui| {
                switch_with_label(ui, palette, "Word wrap", &mut shell.word_wrap);
            },
        );
    });
    card(ui, palette, |ui| {
        row(ui, palette, "Formatting", None, |ui| {
            switch_with_label(ui, palette, "Formatting", &mut shell.formatting);
        });
    });
    card(ui, palette, |ui| {
        row(
            ui,
            palette,
            "Spell check",
            Some("Underline likely misspelled words"),
            |ui| {
                switch_with_label(ui, palette, "Spell check", &mut shell.spell_check);
            },
        );
    });

    ui.add_space(10.0);
    section(ui, "Opening NoteCraft");
    card(ui, palette, |ui| {
        row(ui, palette, "Opening files", None, |ui| {
            dropdown(ui, "opening", "Opening files", &mut shell.opening);
        });
    });
}

/// The right column: the "About this app" panel.
fn about_column(ui: &mut egui::Ui, palette: Palette) {
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
        let _ = ui.link(RichText::new(link).size(13.0).color(palette.accent));
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
fn card(ui: &mut egui::Ui, palette: Palette, body: impl FnOnce(&mut egui::Ui)) {
    Frame::NONE
        .fill(palette.card)
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
    palette: Palette,
    title: &str,
    subtitle: Option<&str>,
    control: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(14.0).color(palette.text));
            if let Some(subtitle) = subtitle {
                ui.label(RichText::new(subtitle).size(12.0).color(palette.text_dim));
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

/// A dropdown bound to a [`MenuOption`] type, named for the setting it sets: a
/// combo box shows only its value, and "Dark" does not say what it is dark of.
fn dropdown<T: MenuOption>(ui: &mut egui::Ui, id: &str, name: &str, selected: &mut T) {
    let combo = egui::ComboBox::from_id_salt(id)
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
    a11y::label(&combo.response, WidgetType::ComboBox, name);
}

/// The font-family dropdown.
fn font_dropdown(ui: &mut egui::Ui, shell: &mut Shell) {
    let combo = egui::ComboBox::from_id_salt("font")
        .selected_text(shell.font_family)
        .width(180.0)
        .show_ui(ui, |ui| {
            for family in FAMILIES {
                if ui
                    .selectable_label(shell.font_family == family, family)
                    .clicked()
                {
                    shell.font_family = family;
                    ui.close();
                }
            }
        });
    a11y::label(&combo.response, WidgetType::ComboBox, "Font");
}

/// A switch followed by its on/off label, named for the setting it sets: the
/// label beside it reads "On", which is the state and not the setting.
fn switch_with_label(ui: &mut egui::Ui, palette: Palette, name: &str, on: &mut bool) {
    let label = if *on { "On" } else { "Off" };
    ui.label(RichText::new(label).size(13.0).color(palette.text_dim));
    let switch = widgets::switch(ui, on, palette);
    a11y::state(&switch, WidgetType::Checkbox, name, *on);
    a11y::role(ui.ctx(), switch.id, Role::Switch);
}

// ---- The settings file ------------------------------------------------------
//
// `NC-P003`'s criterion is "survives restart", and M8's exit criterion is that every
// supported setting persists *and produces the expected behaviour* afterwards. Those
// two together rule out the easiest way to implement persistence: a file that is
// written faithfully and read back into fields nothing consults. So a setting enters
// this file only once it changes something a user can see, and `every_setting_the_
// shell_holds_round_trips` below is what keeps the list honest in both directions —
// nothing stored that is never restored, nothing restored that was never stored.
//
// Like the session file, this is `key value` lines written by hand, with a version
// header, and the same escape function. Two rules about reading it are worth stating
// because they are choices rather than accidents:
//
// * A header this build does not recognise means no settings at all, rather than a
//   partial read. A file from a future build may mean something different by a key
//   this one merely spells the same.
// * A key this build does not know at all is kept verbatim and written back
//   unchanged. That way an older build run after a newer one cannot quietly delete
//   the newer one's settings the first time the user changes anything — the same
//   loss-keeping rule the Markdown engine follows for syntax it cannot parse. A key
//   this build *does* know, holding a value it has no option for (a theme or a font
//   a later version dropped), falls back to that key's default, since a value that
//   cannot be honoured is a smaller problem than a file that cannot be read.

use std::path::PathBuf;

use nc_textproc::{Orientation, PaperSize};

use super::config;
use super::session::{SessionError, escape, parse_bool, unescape};

/// The settings file's format marker, which also fixes its version.
const VERSION: &str = "notecraft-settings 1";

/// The name of the settings file inside the configuration directory.
const FILE: &str = "settings";

/// Every setting the shell holds across runs.
///
/// `PartialEq` is what makes the store write on change rather than on a flag: the
/// frame compares the shell's settings with what is on the file, so a control added
/// later is persisted by the same rule without having to remember to mark anything
/// dirty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Settings {
    /// `Settings ▸ App theme`.
    theme: AppTheme,
    /// `Settings ▸ Font`.
    font_family: &'static str,
    /// The zoom percentage, which is the reference's font size control (§2).
    zoom: u32,
    /// `Settings ▸ Word wrap`, and `View ▸ Word wrap`.
    word_wrap: bool,
    /// `Settings ▸ Formatting`: whether the formatting toolbar is offered at all.
    formatting: bool,
    /// `View ▸ Status bar`.
    status_bar_visible: bool,
    /// `Settings ▸ Opening files`.
    opening: OpenBehavior,
    /// `Settings ▸ Spell check`: whether likely misspellings are underlined
    /// (`NC-P004`). Stored and restored like the switches beside it.
    spell_check: bool,
    /// `File ▸ Page setup` (`NC-F007`, §8): the sheet, its frame, and the header/footer
    /// line. It is a setting like any other, so it too survives a restart.
    page_setup: nc_textproc::PageSetup,
}

impl Settings {
    /// What a shell's current state says its settings are.
    pub(crate) fn of(shell: &Shell) -> Self {
        Self {
            theme: shell.theme,
            font_family: shell.font_family,
            zoom: shell.zoom,
            word_wrap: shell.word_wrap,
            formatting: shell.formatting,
            status_bar_visible: shell.status_bar_visible,
            opening: shell.opening,
            spell_check: shell.spell_check,
            page_setup: shell.page_setup.clone(),
        }
    }

    /// Puts these settings into a shell.
    ///
    /// The theme is stored as the *choice*: which palette it means is resolved every
    /// frame, so a shell restored with `Use system setting` follows whatever the
    /// system reports now rather than the theme it happened to be showing when the
    /// file was written.
    pub(crate) fn apply_to(self, shell: &mut Shell) {
        shell.theme = self.theme;
        shell.font_family = self.font_family;
        shell.zoom = self.zoom;
        shell.word_wrap = self.word_wrap;
        shell.formatting = self.formatting;
        shell.status_bar_visible = self.status_bar_visible;
        shell.opening = self.opening;
        shell.spell_check = self.spell_check;
        shell.page_setup = self.page_setup;
    }

    /// The file's text, with any keys this build does not know kept as they were.
    fn encode(&self, foreign: &[String]) -> String {
        let mut out = String::new();
        out.push_str(VERSION);
        out.push('\n');
        out.push_str(&format!("theme {}\n", theme_token(self.theme)));
        out.push_str(&format!("font {}\n", escape(self.font_family)));
        out.push_str(&format!("zoom {}\n", self.zoom));
        out.push_str(&format!("word_wrap {}\n", u8::from(self.word_wrap)));
        out.push_str(&format!("formatting {}\n", u8::from(self.formatting)));
        out.push_str(&format!(
            "status_bar {}\n",
            u8::from(self.status_bar_visible)
        ));
        out.push_str(&format!("opening {}\n", opening_token(self.opening)));
        out.push_str(&format!("spell_check {}\n", u8::from(self.spell_check)));
        out.push_str(&format!("paper {}\n", self.page_setup.paper.label()));
        out.push_str(&format!(
            "orientation {}\n",
            self.page_setup.orientation.label()
        ));
        out.push_str(&format!("margin_left {}\n", self.page_setup.margins.left));
        out.push_str(&format!("margin_right {}\n", self.page_setup.margins.right));
        out.push_str(&format!("margin_top {}\n", self.page_setup.margins.top));
        out.push_str(&format!(
            "margin_bottom {}\n",
            self.page_setup.margins.bottom
        ));
        out.push_str(&format!("header {}\n", escape(&self.page_setup.header)));
        out.push_str(&format!("footer {}\n", escape(&self.page_setup.footer)));
        for line in foreign {
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    /// Reads the file: the settings it carries, and the lines that belong to a key
    /// this build does not implement.
    ///
    /// Only a wrong header makes the whole file unreadable. Everything else is
    /// tolerated line by line, because a settings file the application refuses to
    /// start with is indistinguishable from one that was never written.
    fn decode(text: &str) -> Result<(Self, Vec<String>), SessionError> {
        let mut lines = text.lines();
        if lines.next() != Some(VERSION) {
            return Err(SessionError::Header);
        }

        // A half-read file is worse than an absent one only when the *structure* is
        // wrong; a value that names an option this build has never heard of is just
        // a value that cannot be honoured, and the key's default is the honest answer.
        let mut settings = defaults();
        let mut foreign = Vec::new();
        for line in lines.filter(|line| !line.is_empty()) {
            if let Some(value) = line.strip_prefix("theme ") {
                settings.theme = theme_from(value);
            } else if let Some(value) = line.strip_prefix("font ") {
                settings.font_family = font_from(&unescape(value)?);
            } else if let Some(value) = line.strip_prefix("zoom ") {
                settings.zoom = value.parse().unwrap_or(defaults().zoom);
            } else if let Some(value) = line.strip_prefix("word_wrap ") {
                settings.word_wrap = parse_bool(value).unwrap_or(defaults().word_wrap);
            } else if let Some(value) = line.strip_prefix("formatting ") {
                settings.formatting = parse_bool(value).unwrap_or(defaults().formatting);
            } else if let Some(value) = line.strip_prefix("status_bar ") {
                settings.status_bar_visible =
                    parse_bool(value).unwrap_or(defaults().status_bar_visible);
            } else if let Some(value) = line.strip_prefix("opening ") {
                settings.opening = opening_from(value);
            } else if let Some(value) = line.strip_prefix("spell_check ") {
                settings.spell_check = parse_bool(value).unwrap_or(defaults().spell_check);
            } else if let Some(value) = line.strip_prefix("paper ") {
                if let Some(paper) = PaperSize::from_label(value) {
                    settings.page_setup.paper = paper;
                }
            } else if let Some(value) = line.strip_prefix("orientation ") {
                if let Some(orientation) = Orientation::from_label(value) {
                    settings.page_setup.orientation = orientation;
                }
            } else if let Some(value) = line.strip_prefix("margin_left ") {
                settings.page_setup.margins.left =
                    value.parse().unwrap_or(settings.page_setup.margins.left);
            } else if let Some(value) = line.strip_prefix("margin_right ") {
                settings.page_setup.margins.right =
                    value.parse().unwrap_or(settings.page_setup.margins.right);
            } else if let Some(value) = line.strip_prefix("margin_top ") {
                settings.page_setup.margins.top =
                    value.parse().unwrap_or(settings.page_setup.margins.top);
            } else if let Some(value) = line.strip_prefix("margin_bottom ") {
                settings.page_setup.margins.bottom =
                    value.parse().unwrap_or(settings.page_setup.margins.bottom);
            } else if let Some(value) = line.strip_prefix("header ") {
                settings.page_setup.header = unescape(value)?;
            } else if let Some(value) = line.strip_prefix("footer ") {
                settings.page_setup.footer = unescape(value)?;
            } else {
                // Not corruption — a key from a newer build. Kept, and written back.
                foreign.push(line.to_owned());
            }
        }
        Ok((settings, foreign))
    }
}

/// The settings as they ship, and the fallback for any value that cannot be read.
///
/// One function rather than a list of per-key defaults, so the shell's starting state
/// and the fallback for a broken file cannot drift apart. It is a function, not a
/// constant, only because the page setup holds the header/footer text.
fn defaults() -> Settings {
    Settings {
        theme: AppTheme::Dark,
        font_family: crate::ui::fonts::DEFAULT,
        zoom: 100,
        word_wrap: true,
        formatting: true,
        status_bar_visible: true,
        opening: OpenBehavior::NewTab,
        spell_check: false,
        page_setup: nc_textproc::PageSetup::default(),
    }
}

/// The settings file, and what it holds right now.
///
/// The main window owns this. Extra windows get their settings from the window that
/// opened them and are not written to disk, which is the same policy the session
/// already follows for its own file: one file per run, written by the window the
/// application was started with.
pub(crate) struct Store {
    /// Where the file is, or `None` when there is no configuration directory — a
    /// run that keeps no settings rather than a failing one.
    path: Option<PathBuf>,
    /// What is on the file now, which is what a change is measured against.
    stored: Settings,
    /// Keys from a newer build, kept so writing cannot erase what this build cannot
    /// read.
    foreign: Vec<String>,
}

impl Store {
    /// The stored settings, read from the configuration directory, or the defaults if
    /// there is no file there to read.
    pub(crate) fn open() -> Self {
        Self::at(config::file(FILE))
    }

    /// The store backed by one file, which is the shape the tests drive.
    fn at(path: Option<PathBuf>) -> Self {
        let read = path
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| Settings::decode(&text).ok());
        match read {
            Some((settings, foreign)) => Self {
                path,
                stored: settings,
                foreign,
            },
            None => Self {
                path,
                stored: defaults(),
                foreign: Vec::new(),
            },
        }
    }

    /// Puts the stored settings into a shell — the one the window starts with.
    pub(crate) fn apply_to(&self, shell: &mut Shell) {
        self.stored.clone().apply_to(shell);
    }

    /// Writes the settings if — and only if — the shell's differ from the file's.
    ///
    /// Called once a frame from the application, after the frame has been drawn, so a
    /// change lands one frame after the click that made it and a window sitting idle
    /// writes nothing. Failing to write is logged and forgotten: the settings stay in
    /// the shell, so the run behaves as the user asked, and the next change tries
    /// again.
    pub(crate) fn sync(&mut self, shell: &Shell) {
        let now = Settings::of(shell);
        if now == self.stored {
            return;
        }
        let Some(path) = self.path.as_ref() else {
            return;
        };
        match write(path, &now, &self.foreign) {
            Ok(()) => self.stored = now,
            Err(error) => tracing::warn!(%error, "could not write the settings"),
        }
    }
}

/// Writes the file through a sibling temporary file, so an interrupted write cannot
/// leave a settings file half-written where the next start would read it.
fn write(path: &PathBuf, settings: &Settings, foreign: &[String]) -> std::io::Result<()> {
    config::ensure_directory(path)?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, settings.encode(foreign))?;
    std::fs::rename(&temporary, path)
}

/// The stored name of a theme choice.
fn theme_token(theme: AppTheme) -> &'static str {
    match theme {
        AppTheme::System => "system",
        AppTheme::Light => "light",
        AppTheme::Dark => "dark",
    }
}

/// Reads a theme choice, defaulting when the name is not one this build offers.
fn theme_from(value: &str) -> AppTheme {
    match value {
        "system" => AppTheme::System,
        "light" => AppTheme::Light,
        "dark" => AppTheme::Dark,
        _ => defaults().theme,
    }
}

/// The stored name of an opening behaviour.
fn opening_token(opening: OpenBehavior) -> &'static str {
    match opening {
        OpenBehavior::NewTab => "new_tab",
        OpenBehavior::NewWindow => "new_window",
        OpenBehavior::ReplaceTab => "replace_tab",
    }
}

/// Reads an opening behaviour, defaulting when the name is not one this build offers.
fn opening_from(value: &str) -> OpenBehavior {
    match value {
        "new_tab" => OpenBehavior::NewTab,
        "new_window" => OpenBehavior::NewWindow,
        "replace_tab" => OpenBehavior::ReplaceTab,
        _ => defaults().opening,
    }
}

/// Reads a font family, which has to be one this build carries a face for.
///
/// The families are a fixed list because the faces are embedded, not discovered: a name
/// from a newer list would ask the editor to draw in a font that is not there, so the
/// default family takes it instead.
fn font_from(value: &str) -> &'static str {
    FAMILIES
        .into_iter()
        .find(|family| *family == value)
        .unwrap_or(defaults().font_family)
}

/// Reads the settings at a path, for the tests that assert on the file itself.
#[cfg(test)]
fn read_back(path: &PathBuf) -> Option<Settings> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| Settings::decode(&text).ok())
        .map(|(settings, _)| settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::actions;
    use nc_commands::CommandId;
    use std::path::{Path, PathBuf};

    /// A scratch directory of its own, removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("notecraft-m8-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("a scratch directory");
            Self(directory)
        }

        /// The settings file inside it, whether or not anything has written it yet.
        fn file(&self) -> PathBuf {
            self.0.join(FILE)
        }

        /// A store reading and writing that file.
        fn store(&self) -> Store {
            Store::at(Some(self.file()))
        }

        /// The file's text, or `None` when nothing has been written.
        fn text(&self) -> Option<String> {
            std::fs::read_to_string(self.file()).ok()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A restart: a shell that has never seen the previous run, given what the file
    /// holds. This is the whole of what `NC-P003` asks about.
    fn restart(path: &Path) -> Shell {
        let mut shell = Shell::new();
        Store::at(Some(path.to_path_buf())).apply_to(&mut shell);
        shell
    }

    /// Writes a settings file by hand, as a build this one has never met would.
    fn write_raw(scratch: &Scratch, text: &str) {
        std::fs::write(scratch.file(), text).expect("the file is written");
    }

    // ---- the format itself --------------------------------------------------

    #[test]
    fn the_file_names_each_setting_rather_than_numbering_it() {
        // A user reporting something odd should be able to read this file in a text
        // editor, which is the reason it is not a serialization crate's output.
        let settings = Settings {
            theme: AppTheme::Light,
            font_family: "Fira Mono",
            zoom: 130,
            word_wrap: false,
            formatting: false,
            status_bar_visible: false,
            opening: OpenBehavior::ReplaceTab,
            spell_check: true,
            page_setup: nc_textproc::PageSetup::default(),
        };
        assert_eq!(
            settings.encode(&[]),
            "notecraft-settings 1\n\
             theme light\n\
             font Fira Mono\n\
             zoom 130\n\
             word_wrap 0\n\
             formatting 0\n\
             status_bar 0\n\
             opening replace_tab\n\
             spell_check 1\n\
             paper A4\n\
             orientation Portrait\n\
             margin_left 20\n\
             margin_right 20\n\
             margin_top 20\n\
             margin_bottom 20\n\
             header \n\
             footer \n"
        );
    }

    #[test]
    fn a_written_file_reads_back_as_the_same_settings() {
        let settings = Settings {
            theme: AppTheme::System,
            font_family: "IBM Plex Mono",
            zoom: 90,
            word_wrap: false,
            formatting: true,
            status_bar_visible: true,
            opening: OpenBehavior::NewWindow,
            spell_check: true,
            page_setup: nc_textproc::PageSetup {
                paper: nc_textproc::PaperSize::Letter,
                orientation: nc_textproc::Orientation::Landscape,
                margins: nc_textproc::Margins {
                    left: 10,
                    right: 12,
                    top: 14,
                    bottom: 16,
                },
                header: "Page &p of the run".to_string(),
                footer: "Printed &d".to_string(),
            },
        };
        let (decoded, foreign) = Settings::decode(&settings.encode(&[])).expect("it decodes");
        assert_eq!(decoded, settings);
        assert!(foreign.is_empty(), "nothing here is unknown to this build");
    }

    // ---- surviving a restart ------------------------------------------------

    #[test]
    fn every_setting_the_shell_holds_round_trips_through_the_file() {
        // One field at a time, so a setting that is stored but never restored — or
        // restored but never stored, or restored over a neighbour — shows up as
        // itself rather than hiding behind the five that work.
        /// One setting, moved, with the label to name in a failure.
        type Change = (&'static str, fn(&mut Shell));
        let changes: [Change; 9] = [
            ("theme", |shell| shell.theme = AppTheme::Light),
            ("font", |shell| shell.font_family = "Fira Mono"),
            ("zoom", |shell| shell.zoom = 130),
            ("word wrap", |shell| shell.word_wrap = false),
            ("formatting", |shell| shell.formatting = false),
            ("status bar", |shell| shell.status_bar_visible = false),
            ("opening files", |shell| {
                shell.opening = OpenBehavior::NewWindow
            }),
            ("spell check", |shell| shell.spell_check = true),
            ("page setup", |shell| {
                shell.page_setup.paper = nc_textproc::PaperSize::Legal;
                shell.page_setup.orientation = nc_textproc::Orientation::Landscape;
                shell.page_setup.margins.left = 15;
                shell.page_setup.header = "Draft &p".to_string();
                shell.page_setup.footer = "&d".to_string();
            }),
        ];

        let scratch = Scratch::new("round-trip");
        for (label, change) in changes {
            let _ = std::fs::remove_file(scratch.file());
            let mut shell = Shell::new();
            change(&mut shell);
            assert_ne!(
                Settings::of(&shell),
                defaults(),
                "the test itself is broken: {label} was not changed"
            );

            scratch.store().sync(&shell);
            let restored = restart(&scratch.file());

            assert_eq!(
                Settings::of(&restored),
                Settings::of(&shell),
                "{label}: what went into the file is not what came out of it"
            );
        }
    }

    #[test]
    fn turning_word_wrap_off_stays_off_after_a_restart() {
        // `NC-P003`'s acceptance test, driven through the command a user actually
        // fires rather than by writing the field.
        let scratch = Scratch::new("word-wrap");
        let mut shell = Shell::new();
        assert!(
            shell.word_wrap,
            "it starts wrapped, as §4 shows the switch on"
        );
        let before = shell.active_tab().expect("a tab").text().to_owned();

        actions::run(
            &mut shell,
            &egui::Context::default(),
            CommandId("cmd.view.word_wrap"),
        );
        assert!(!shell.word_wrap);

        // Wrapping is a way of *showing* a document, never a change to it: the other
        // half of `NC-E006`'s criterion. Its suite settles the layout arithmetic; this
        // settles the toggle a user presses.
        let tab = shell.active_tab().expect("a tab");
        assert_eq!(tab.text(), before, "no byte of the document moved");
        assert!(!tab.is_dirty(), "and it is not asking to be saved");

        scratch.store().sync(&shell);

        let restored = restart(&scratch.file());
        assert!(!restored.word_wrap, "the choice survived the restart");
    }

    #[test]
    fn the_theme_choice_survives_a_restart_as_a_choice_not_as_a_colour() {
        let scratch = Scratch::new("theme");
        let mut shell = Shell::new();
        shell.theme = AppTheme::System;
        scratch.store().sync(&shell);

        let restored = restart(&scratch.file());
        assert_eq!(restored.theme, AppTheme::System);
        // The palette is not stored, so a shell restored to follow the system resolves
        // against what the system says now — see theme::Palette::for_choice.
        assert_eq!(
            restored.palette,
            crate::ui::theme::Palette::DARK,
            "as shipped"
        );
    }

    // ---- when it writes -----------------------------------------------------

    #[test]
    fn nothing_is_written_until_a_setting_changes() {
        // A default run should not leave a file behind: the settings are what they are
        // because nobody changed them, and writing the file anyway would make every
        // run look like it had been configured.
        let scratch = Scratch::new("quiet");
        let shell = Shell::new();

        scratch.store().sync(&shell);

        assert!(scratch.text().is_none(), "no change, no file");
    }

    #[test]
    fn an_idle_window_writes_the_file_once_and_not_every_frame() {
        // One store stands in for one open window: the application holds it for the
        // run, so comparing against what it last wrote is what keeps a window sitting
        // still from rewriting the file sixty times a second.
        let scratch = Scratch::new("idle");
        let mut shell = Shell::new();
        shell.zoom = 150;

        let mut store = scratch.store();
        store.sync(&shell);
        assert_eq!(
            scratch
                .text()
                .expect("the change was written")
                .lines()
                .count(),
            17,
            "the header, one line per setting, and the eight lines the page setup is made of"
        );

        let _ = std::fs::remove_file(scratch.file());
        store.sync(&shell);
        store.sync(&shell);
        assert!(
            scratch.text().is_none(),
            "the same settings, unchanged, should not recreate the file"
        );
    }

    #[test]
    fn a_run_whose_file_has_vanished_writes_the_settings_back() {
        // The other half of the same rule. A file removed mid-run — a cleanup script,
        // a dotfile sync tool — should not mean the next start forgets the settings
        // this run is still using, so a store that opens on a missing file treats the
        // defaults as what is on disk and finds the shell disagreeing.
        let scratch = Scratch::new("vanished");
        let mut shell = Shell::new();
        shell.zoom = 150;
        scratch.store().sync(&shell);
        let _ = std::fs::remove_file(scratch.file());

        scratch.store().sync(&shell);

        let restored = restart(&scratch.file());
        assert_eq!(restored.zoom, 150, "the settings were recovered");
    }

    #[test]
    fn a_store_with_nowhere_to_write_keeps_the_settings_in_the_run() {
        // No configuration directory is a run that persists nothing, not a failed
        // start: the change still takes effect for as long as the window is open.
        let mut shell = Shell::new();
        shell.word_wrap = false;
        let mut store = Store::at(None);
        store.sync(&shell);
        assert!(!shell.word_wrap, "the setting still applies");
    }

    // ---- reading something this build did not write -------------------------

    #[test]
    fn a_missing_file_is_no_settings() {
        let scratch = Scratch::new("missing");
        let mut shell = Shell::new();
        shell.zoom = 250;
        scratch.store().apply_to(&mut shell);
        assert_eq!(Settings::of(&shell), defaults());
    }

    #[test]
    fn a_file_from_a_future_build_is_not_partly_applied() {
        // A key that exists in both builds may mean something different in the newer
        // one, so a header this build does not recognise means "no settings", the way
        // an unreadable session means no session.
        let scratch = Scratch::new("future");
        write_raw(&scratch, "notecraft-settings 2\ntheme light\nzoom 300\n");

        let mut shell = Shell::new();
        scratch.store().apply_to(&mut shell);

        assert_eq!(
            Settings::of(&shell),
            defaults(),
            "nothing from an unknown version is honoured"
        );
    }

    #[test]
    fn a_value_this_build_cannot_honour_falls_back_while_the_rest_applies() {
        // A theme or font the build has no option for is not corruption, and losing a
        // whole file over one unreadable word would punish the settings that are fine.
        let scratch = Scratch::new("unknown-value");
        write_raw(
            &scratch,
            "notecraft-settings 1\ntheme sepia\nfont Comic Sans\nzoom 150\n",
        );

        let mut shell = Shell::new();
        scratch.store().apply_to(&mut shell);

        assert_eq!(
            shell.zoom, 150,
            "the setting this build understands applied"
        );
        assert_eq!(
            shell.theme,
            defaults().theme,
            "the theme fell back to its default"
        );
        assert_eq!(shell.font_family, defaults().font_family);
    }

    #[test]
    fn a_key_from_a_newer_build_survives_a_write() {
        // The rule that keeps an older build from erasing a newer one's settings, on
        // the same principle the Markdown engine follows for syntax it cannot parse:
        // what you do not understand is passed through untouched.
        let scratch = Scratch::new("foreign-key");
        write_raw(
            &scratch,
            "notecraft-settings 1\ntheme light\nhyphenation 1\nautocorrect 0\n",
        );

        let mut shell = Shell::new();
        let mut store = scratch.store();
        store.apply_to(&mut shell);
        assert_eq!(shell.theme, AppTheme::Light, "the known keys applied");

        shell.zoom = 110;
        store.sync(&shell);

        let text = scratch.text().expect("the file was rewritten");
        assert!(
            text.contains("hyphenation 1\nautocorrect 0\n"),
            "the keys this build does not implement were not dropped: {text:?}"
        );
        assert_eq!(
            read_back(&scratch.file()).expect("it still reads").zoom,
            110
        );
    }

    #[test]
    fn a_repeated_key_takes_the_last_answer() {
        let scratch = Scratch::new("repeated");
        write_raw(&scratch, "notecraft-settings 1\nzoom 110\nzoom 140\n");

        let mut shell = Shell::new();
        scratch.store().apply_to(&mut shell);

        assert_eq!(shell.zoom, 140, "the last line is the one that won");
    }

    #[test]
    fn a_font_that_is_not_one_of_the_offered_families_is_not_applied() {
        // The list is fixed because the build carries its own faces; a name from a
        // newer list would ask the editor to draw in a font that is not there. The names
        // this rejects are the ones the old list offered: they were real font names on
        // Windows and nothing here.
        for value in ["", "Consolas ", "not a font", "consolas"] {
            assert_eq!(font_from(value), defaults().font_family, "{value:?}");
        }
        for family in FAMILIES {
            assert_eq!(font_from(family), family);
        }
    }
}
