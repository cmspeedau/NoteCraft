//! Extra top-level windows (`File ▸ New window`, NC-F003).
//!
//! A window is an `egui` viewport with its own [`Shell`], so it holds its own
//! tabs and active document independently of the main window. The main window
//! is the one the application was started with; this model owns only the extra
//! ones. Each is drawn every frame with [`egui::Context::show_viewport_immediate`],
//! which is what keeps it alive — a viewport that is not shown in a frame is
//! gone. On a backend without multiple viewports `egui` embeds the window in
//! the current one instead, so the same code still runs.
//!
//! The reference capture fixes the *command* (`File ▸ New window`, Ctrl+Shift+N)
//! but not what the new window contains; it is opened with one empty untitled
//! document, the same state the application starts in (a recorded decision).
//! When a file being opened is what asks for a window (`Settings ▸ Opening
//! files`, *Open in a new window*), that window opens showing the file instead:
//! the request carries the document with it, and an empty list means the plain
//! command's empty document.

use egui::{Context, Id, ViewportBuilder, ViewportId};

use crate::ui::draw;
use crate::ui::settings::Settings;
use crate::ui::state::{Shell, Tab};

/// The set of extra windows the application has opened.
pub struct Windows {
    /// One per extra window, in creation order.
    open: Vec<Window>,
    /// The next viewport number to hand out; it keeps ids unique as windows
    /// come and go within a run.
    next_number: u64,
}

impl Windows {
    /// No extra windows open.
    pub fn new() -> Self {
        Self {
            open: Vec::new(),
            next_number: 1,
        }
    }

    /// The number of extra windows open.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.open.len()
    }

    /// Whether there are no extra windows.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }

    /// Creates a window for every pending `New window` request: the main
    /// window's and each extra window's own. Called before [`show`](Self::show)
    /// so a new window is drawn the frame it was asked for.
    pub fn create_requested(&mut self, main: &mut Shell) {
        // The new window opens as the window that asked for it is. Settings belong
        // to a window, so a New window fired in one that is light, word-wrapped and
        // zoomed to 130% is a window that starts the same way; inheriting the
        // defaults instead would make the command lose the user's own setup.
        //
        // The request carries documents with it when what asked for the window was
        // a file being opened (`Settings ▸ Opening files`, *Open in a new window*),
        // and none for a plain `File ▸ New window`, which opens one empty document.
        let mut inherited: Option<(Settings, Vec<Tab>)> = None;
        if std::mem::take(&mut main.new_window_request) {
            inherited = Some((
                Settings::of(main),
                std::mem::take(&mut main.new_window_documents),
            ));
        }
        for window in &mut self.open {
            if std::mem::take(&mut window.shell.new_window_request) && inherited.is_none() {
                inherited = Some((
                    Settings::of(&window.shell),
                    std::mem::take(&mut window.shell.new_window_documents),
                ));
            }
        }
        if let Some((settings, documents)) = inherited {
            self.open
                .push(Window::new(self.next_number, settings, documents));
            self.next_number += 1;
        }
    }

    /// Draws every extra window for this frame, first dropping any the user has
    /// closed.
    ///
    /// A window keeps existing only by being shown: once its close has been
    /// requested — by its own title-bar `✕`, through the same window-close path
    /// as the main window (`actions::close_window`) — it is removed here, and
    /// with it its viewport.
    pub fn show(&mut self, ctx: &Context) {
        self.open
            .retain(|window| !ctx.input_for(window.id, |input| input.viewport().close_requested()));

        for window in &mut self.open {
            let id = window.id;
            let title = window.title.clone();
            ctx.show_viewport_immediate(
                id,
                ViewportBuilder::default()
                    .with_title(title)
                    .with_decorations(false)
                    .with_inner_size([900.0, 600.0]),
                |ui, _class| {
                    draw(&mut window.shell, ui);
                },
            );
        }
    }
}

impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

/// One extra top-level window: its viewport and its own document set.
struct Window {
    /// The `egui` viewport this window is drawn in.
    id: ViewportId,
    /// The title the OS shows, when the backend draws decorations of its own.
    title: String,
    /// The window's own shell: its tabs, active document, and view options.
    shell: Shell,
}

impl Window {
    /// A window `number`, shown with `settings`.
    ///
    /// It holds `documents` when a file being opened asked for the window
    /// (`Settings ▸ Opening files`), and one empty untitled document when there
    /// are none — which is what a plain `File ▸ New window` opens.
    fn new(number: u64, settings: Settings, documents: Vec<Tab>) -> Self {
        let mut shell = Shell::new();
        settings.apply_to(&mut shell);
        if !documents.is_empty() {
            shell.tabs = documents;
            shell.active = 0;
        }
        Self {
            id: ViewportId(Id::new(("notecraft_window", number))),
            title: format!("NoteCraft — Window {number}"),
            shell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::state::{AppTheme, DocKind, OpenBehavior};

    #[test]
    fn a_request_creates_one_window_with_its_own_shell() {
        let mut windows = Windows::new();
        let mut main = Shell::new();

        main.new_window_request = true;
        windows.create_requested(&mut main);

        assert_eq!(windows.len(), 1, "one window was created");
        assert!(!main.new_window_request, "the request was consumed");
        // The new window starts with its own single untitled document.
        assert_eq!(windows.open[0].shell.tabs.len(), 1);
    }

    #[test]
    fn opening_in_a_new_window_takes_the_file_to_the_window_it_opens() {
        // `Settings ▸ Opening files`, *Open in a new window*: the file is read in
        // the window the user used — so one that cannot be read is reported there —
        // and the window that shows it is the one that opens next.
        let path =
            std::env::temp_dir().join(format!("notecraft-window-open-{}", std::process::id()));
        std::fs::write(&path, "hello").expect("a file");

        let mut windows = Windows::new();
        let mut main = Shell::new();
        main.opening = OpenBehavior::NewWindow;

        crate::ui::actions::open_path(&mut main, &path, None);

        assert_eq!(main.tabs.len(), 1, "this window kept its own document");
        assert_eq!(main.tabs[0].text(), "", "and it is still the empty one");
        assert!(main.new_window_request, "the window was asked for");
        assert_eq!(
            main.new_window_documents.len(),
            1,
            "with the file travelling to it"
        );

        windows.create_requested(&mut main);

        assert_eq!(windows.len(), 1);
        assert!(!main.new_window_request, "the request was consumed");
        assert!(
            main.new_window_documents.is_empty(),
            "and so was the document that travelled with it"
        );
        let shell = &windows.open[0].shell;
        assert_eq!(shell.tabs.len(), 1, "the new window shows the file alone");
        assert_eq!(shell.tabs[0].text(), "hello");
        assert_eq!(shell.active, 0);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_new_window_opens_with_the_settings_of_the_window_it_came_from() {
        // The settings belong to a window, so the new one has to start as the window
        // that asked for it looks. Inheriting the build's defaults instead would make
        // `File ▸ New window` throw away the theme, the zoom and the wrap the user
        // chose — and after M8, it would throw away the ones they set days ago.
        let mut windows = Windows::new();
        let mut main = Shell::new();
        main.theme = AppTheme::Light;
        main.zoom = 130;
        main.word_wrap = false;
        main.opening = OpenBehavior::NewWindow;

        main.new_window_request = true;
        windows.create_requested(&mut main);

        let shell = &windows.open[0].shell;
        assert_eq!(
            shell.theme,
            AppTheme::Light,
            "the window opened light, as asked"
        );
        assert_eq!(shell.zoom, 130);
        assert!(!shell.word_wrap);
        assert_eq!(shell.opening, OpenBehavior::NewWindow);
        assert_eq!(
            shell.tabs.len(),
            1,
            "and still starts with one empty document"
        );
    }

    #[test]
    fn a_window_keeps_its_own_documents() {
        let mut windows = Windows::new();
        let mut main = Shell::new();
        main.open_tab(DocKind::Plain);

        main.new_window_request = true;
        windows.create_requested(&mut main);

        assert_eq!(main.tabs.len(), 2, "the main window kept its tabs");
        assert_eq!(
            windows.open[0].shell.tabs.len(),
            1,
            "the new window has its own"
        );
        assert_ne!(
            windows.open[0].id,
            ViewportId::ROOT,
            "the window has a viewport of its own"
        );
    }

    #[test]
    fn each_window_gets_a_distinct_viewport() {
        let mut windows = Windows::new();
        let mut main = Shell::new();

        main.new_window_request = true;
        windows.create_requested(&mut main);
        main.new_window_request = true;
        windows.create_requested(&mut main);

        assert_eq!(windows.len(), 2);
        assert_ne!(windows.open[0].id, windows.open[1].id);
    }

    #[test]
    fn a_request_from_an_extra_window_opens_another() {
        let mut windows = Windows::new();
        let mut main = Shell::new();

        main.new_window_request = true;
        windows.create_requested(&mut main);
        // The first extra window now asks for one of its own.
        windows.open[0].shell.new_window_request = true;
        windows.create_requested(&mut main);

        assert_eq!(windows.len(), 2);
        assert!(!windows.open[0].shell.new_window_request);
    }

    #[test]
    fn no_request_creates_nothing() {
        let mut windows = Windows::new();
        let mut main = Shell::new();
        windows.create_requested(&mut main);
        assert!(windows.is_empty());
    }

    #[test]
    fn showing_a_window_paints_a_frame() {
        // The native multi-viewport path cannot run headlessly, but eframe's
        // fallback embeds the window, which exercises the same `show` code and
        // paints the window's own shell.
        let ctx = egui::Context::default();
        let mut windows = Windows::new();
        let mut main = Shell::new();
        main.new_window_request = true;
        windows.create_requested(&mut main);

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1180.0, 720.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |_ui| windows.show(&ctx));
        output.textures_delta.clear();

        assert_eq!(windows.len(), 1, "the window is still open");
        assert!(!output.shapes.is_empty(), "the window painted nothing");
    }
}
