//! What a screen reader is told, and in what order (`NC-P007`).
//!
//! `egui` builds an [AccessKit](https://accesskit.dev) tree as a frame is drawn:
//! a widget added with `ui.button("Save")` reaches a screen reader named `Save`
//! with the role of a button, and nothing has to be written here for it. What
//! `egui` cannot name is a control this shell painted itself — the pill switch, a
//! tab, the round avatar, any button whose whole visible text is a glyph (`🔗`,
//! `▦`, `✕`) — because there is no text for it to read. Those are what this
//! module is for.
//!
//! The other half of the row, *focus order*, is `egui`'s to decide: Tab moves
//! through the widgets in the order a frame registers them, and the tree lists
//! them in that same order. So the order is pinned rather than chosen, and the one
//! thing worth knowing is that a group laid out right to left — the window
//! buttons, the top bar's right-hand controls — registers right to left too, which
//! is the order the tests below hold it to. Measured, not assumed: a nested
//! left-to-right layout inside the right-to-left one puts the group at the left
//! edge of the bar, so the two cannot both be had from `egui`'s own layouts, and
//! the layout the capture fixes wins. A screen-reader user is not stuck with it:
//! `egui` also moves focus by direction (arrow keys), which is spatial, so the
//! group is reachable in the order it is drawn as well.
//!
//! Every call here is a no-op unless a screen reader is running — `egui` keeps no
//! tree otherwise — so the module costs nothing to a window nobody is listening to.

use egui::accesskit::Role;
use egui::{Context, Id, Rect, Response, WidgetInfo, WidgetType};

/// Names the control behind `response`, and gives it the role its kind would have
/// if `egui` had drawn it.
///
/// For what is painted by hand. A control whose visible text is a glyph is
/// announced as the glyph otherwise, which is not a name: `🔗` is read as a
/// codepoint, or skipped, and `✕` is the same character whether it closes a tab or
/// the window.
pub fn label(response: &Response, kind: WidgetType, name: impl ToString) {
    response.widget_info(|| WidgetInfo::labeled(kind, response.enabled(), name.to_string()));
}

/// Names a control that is on or off, or one of a set of choices, and says which.
///
/// `WidgetInfo::selected` reports the state as *toggled*, so this is the one to
/// use where that is what the reader should hear — a switch, a checkbox. A tab
/// wants [`role`] and [`selected`] instead: a tab is selected, not toggled.
pub fn state(response: &Response, kind: WidgetType, name: impl ToString, on: bool) {
    response.widget_info(|| WidgetInfo::selected(kind, response.enabled(), on, name.to_string()));
}

/// Sets the role alone, for a control that is finer than any [`WidgetType`].
///
/// Call after [`label`] or [`state`]: those set the role their kind maps to, and
/// this is what corrects it. `WidgetType` has no tab and no switch, and a tab
/// announced as a button is a small lie a reader will hear.
pub fn role(ctx: &Context, id: Id, role: Role) {
    ctx.accesskit_node_builder(id, |node| node.set_role(role));
}

/// Says whether the control is selected — what a tab is, rather than toggled.
pub fn selected(ctx: &Context, id: Id, selected: bool) {
    ctx.accesskit_node_builder(id, |node| node.set_selected(selected));
}

/// Adds the state that is painted rather than written: the unsaved-changes dot on
/// a tab, which no text on the tab says.
pub fn described(ctx: &Context, id: Id, text: impl Into<Box<str>>) {
    ctx.accesskit_node_builder(id, |node| node.set_description(text));
}

/// Marks a control this build draws but does not act on as unavailable.
///
/// The alternative is to leave it out of the tab order, which is worse: the
/// screen then disagrees with the picture about what is on it. Announcing it as
/// unavailable is what is true of it.
pub fn inert(ctx: &Context, id: Id) {
    ctx.accesskit_node_builder(id, |node| node.set_disabled());
}

/// Names a string this shell painted, with `rect` as what the screen reader
/// points at.
///
/// A painted string has no widget behind it, so nothing is announced for it; the
/// status bar's document kind is one.
pub fn painted(ctx: &Context, id: Id, rect: Rect, text: impl Into<Box<str>>) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(Role::Label);
        node.set_value(text);
        node.set_bounds(area(rect));
    });
}

/// Exposes a document this shell paints rather than edits.
///
/// The formatted view is a rendering of the text and holds no text field, so
/// without this a screen reader would read the body of the window as empty.
/// What is exposed is the document's own text — the view is one rendering of it —
/// and not the shape of that rendering: headings, lists and tables are painted,
/// and the tree does not say which is which.
pub fn document(
    ctx: &Context,
    id: Id,
    rect: Rect,
    name: impl Into<Box<str>>,
    text: impl Into<Box<str>>,
) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(Role::Document);
        node.set_label(name);
        node.set_value(text);
        node.set_bounds(area(rect));
    });
}

/// An `egui` rectangle as the screen reader's coordinate type.
fn area(rect: Rect) -> egui::accesskit::Rect {
    egui::accesskit::Rect {
        x0: rect.min.x.into(),
        y0: rect.min.y.into(),
        x1: rect.max.x.into(),
        y1: rect.max.y.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::actions;
    use crate::ui::fonts::FAMILIES;
    use crate::ui::state::{
        AppTheme, DocKind, FileDialog, OpenBehavior, Prompt, Shell, SpellDialog,
    };
    use crate::ui::{draw, theme};
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// One node of the tree a painted frame produced, with the parts `NC-P007` is
    /// about.
    #[derive(Debug)]
    struct Seen {
        role: Role,
        name: Option<String>,
        value: Option<String>,
        description: Option<String>,
        selected: Option<bool>,
        toggled: Option<bool>,
        disabled: bool,
        /// Whether Tab stops here, which is what the focus order is about.
        focusable: bool,
        bounds: Option<egui::accesskit::Rect>,
    }

    /// The accesskit tree one painted frame produces, walked from the root.
    ///
    /// Walked rather than read off: the update's node list is an `IdMap`'s own
    /// order, which is a hash order and not the order a screen reader sees. The
    /// tree's children are pushed as the widgets are created, so a pre-order walk
    /// from the root is the order the frame registered them in — the order Tab
    /// moves through them.
    fn tree(shell: &mut Shell) -> Vec<Seen> {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        theme::install(&ctx, shell.palette);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1180.0, 720.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| draw(shell, ui));
        output.textures_delta.clear();
        let update = output
            .platform_output
            .accesskit_update
            .expect("a tree for the frame");

        let nodes: HashMap<egui::accesskit::NodeId, egui::accesskit::Node> =
            update.nodes.into_iter().collect();
        let root = egui::accesskit_root_id().accesskit_id();

        let mut walked = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let Some(node) = nodes.get(&id) else {
                continue;
            };
            // A stack would reverse them; children are in creation order.
            for child in node.children().iter().rev() {
                stack.push(*child);
            }
            walked.push(Seen {
                role: node.role(),
                name: node.label().map(str::to_owned),
                value: node.value().map(str::to_owned),
                description: node.description().map(str::to_owned),
                selected: node.is_selected(),
                toggled: node
                    .toggled()
                    .map(|toggled| matches!(toggled, egui::accesskit::Toggled::True)),
                disabled: node.is_disabled(),
                focusable: node.supports_action(egui::accesskit::Action::Focus),
                bounds: node.bounds(),
            });
        }
        walked
    }

    /// What Tab moves through, in order, as a screen reader would read it.
    fn focus_order(shell: &mut Shell) -> Vec<String> {
        tree(shell)
            .into_iter()
            .filter(|node| node.focusable)
            .map(|node| {
                node.name
                    .or(node.value)
                    .unwrap_or_else(|| format!("<unnamed {:?}>", node.role))
            })
            .collect()
    }

    /// The node named `name`, which must be there.
    fn named(shell: &mut Shell, name: &str) -> Seen {
        tree(shell)
            .into_iter()
            .find(|node| node.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no node named {name:?}"))
    }

    /// The node with that role and name. Two nodes can share a name — the editor
    /// and the tab showing the same document are both named after it — and the
    /// role is what tells them apart.
    fn named_role(shell: &mut Shell, role: Role, name: &str) -> Seen {
        tree(shell)
            .into_iter()
            .find(|node| node.role == role && node.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"))
    }

    #[test]
    fn every_control_in_the_tab_order_has_a_name() {
        // The check the row is written against, done as a screen reader would
        // meet it: everywhere Tab can go must say what it is. A glyph is not a
        // name — `🔗`, `▦`, `✕` and the pill switch all reached a reader as
        // themselves or as nothing before this — so a name has to carry a letter.
        // Every surface a user can be looking at, because each one adds stops of
        // its own.
        for (surface, configure) in surfaces() {
            let mut shell = Shell::new();
            configure(&mut shell);
            for node in tree(&mut shell).into_iter().filter(|node| node.focusable) {
                let name = node.name.as_deref().unwrap_or_else(|| {
                    panic!(
                        "{surface}: {:?} at {:?} has no name",
                        node.role, node.bounds
                    )
                });
                assert!(
                    name.chars().any(|c| c.is_ascii_alphanumeric()),
                    "{surface}: {name:?} ({:?}) is not a name",
                    node.role
                );
            }
        }
    }

    /// What puts a shell in one surface.
    type Setup = fn(&mut Shell);

    /// A surface a user can be looking at, and what puts the shell in it.
    fn surfaces() -> Vec<(&'static str, Setup)> {
        fn formatted_view(shell: &mut Shell) {
            shell.open_tab(DocKind::Markdown);
            let index = shell.active;
            shell.tabs[index].formatted = true;
        }

        vec![
            ("the source view", |_| {}),
            ("the formatted view", formatted_view),
            ("Settings", |shell| shell.show_settings = true),
            ("Find and Replace", |shell| {
                shell.find.open = true;
                shell.find.replace = true;
            }),
            ("Go to line", |shell| shell.go_to.open = true),
            ("a spelling dialog", |shell| {
                shell.spell_check = true;
                shell.spelling = SpellDialog {
                    open: true,
                    tab: 0,
                    word: "teh".to_owned(),
                    range: 4..7,
                    suggestions: vec!["the".to_owned(), "ten".to_owned()],
                };
            }),
            ("the Open dialog", |shell| {
                shell.file_dialog = Some(FileDialog::open());
            }),
            ("an unsaved-changes prompt", |shell| {
                shell.prompt = Some(Prompt::UnsavedChanges { tab: 0 });
            }),
            ("an outside-change prompt", |shell| {
                shell.prompt = Some(Prompt::OutsideChange {
                    tab: 0,
                    path: std::path::PathBuf::from("note.txt"),
                });
            }),
        ]
    }

    #[test]
    fn the_tab_order_walks_the_window_it_is_laid_out_in() {
        // Recorded rather than chosen: this is the order a frame registers the
        // controls, which is the order `egui` tabs through them. The two
        // right-aligned groups — the window buttons and the top bar's right-hand
        // controls — walk right to left, because that is the direction their
        // layout is built in; see the module note.
        let mut shell = Shell::new();
        assert_eq!(
            focus_order(&mut shell),
            [
                "Back",
                "Untitled",
                "Close Untitled",
                "New tab",
                "Close window",
                "Maximize window",
                "Minimize window",
                "File",
                "Edit",
                "View",
                "Paragraph style",
                "Lists",
                "Bold",
                "Italic",
                "Strikethrough",
                "Insert a link",
                "Insert a table",
                "Clear all formatting",
                "Settings",
                "Account",
                "Send feedback",
                "AI",
                "Untitled",
            ]
        );
    }

    #[test]
    fn a_document_is_a_tab_that_says_its_name_and_whether_it_is_shown() {
        let scratch = Scratch::new("tabs");
        let path = scratch.file("note.txt", "hello");
        let mut shell = Shell::new();
        actions::open_path(&mut shell, &path, None);

        let web = named(&mut shell, "note.txt");
        assert_eq!(web.role, Role::Tab, "a tab is a tab, not a button");
        assert_eq!(web.selected, Some(true), "it is the one being shown");
        assert_eq!(web.description, None, "and it has nothing unsaved in it");

        let first = named(&mut shell, "Untitled");
        assert_eq!(first.selected, Some(false), "the other tab is not shown");

        // The dot a tab paints when its text has moved on is what a reader cannot
        // see, so the tab says it in words.
        shell.active = 0;
        append(&mut shell.tabs[0], "edited");
        let edited = named(&mut shell, "Untitled");
        assert_eq!(edited.description.as_deref(), Some("Unsaved changes"));
    }

    #[test]
    fn a_tab_can_be_closed_by_name() {
        let scratch = Scratch::new("tab-close");
        let path = scratch.file("note.txt", "hello");
        let mut shell = Shell::new();
        actions::open_path(&mut shell, &path, None);

        let close = named(&mut shell, "Close note.txt");
        assert_eq!(close.role, Role::Button);
        assert!(close.focusable, "a close button is reachable by keyboard");
    }

    #[test]
    fn the_inert_back_arrow_is_announced_as_unavailable() {
        // The capture has a back arrow in the title bar and this build has
        // nowhere to go back to. Leaving it out of the tree would put the screen
        // at odds with the picture; saying it is unavailable is what is true.
        let mut shell = Shell::new();
        let back = named(&mut shell, "Back");
        assert!(back.focusable);
        assert!(back.disabled, "it is drawn, and it does nothing");
    }

    #[test]
    fn a_setting_says_its_name_and_its_state() {
        let mut shell = Shell::new();
        shell.show_settings = true;

        let wrap = named(&mut shell, "Word wrap");
        assert_eq!(wrap.toggled, Some(true), "the switch is on and says so");
        let formatting = named(&mut shell, "Formatting");
        assert_eq!(formatting.toggled, Some(true));
        // Spell check is the one switch that starts off, so this also pins the
        // default the reference-captured page could not show.
        assert_eq!(named(&mut shell, "Spell check").toggled, Some(false));

        shell.word_wrap = false;
        shell.formatting = false;
        shell.spell_check = true;
        assert_eq!(named(&mut shell, "Word wrap").toggled, Some(false));
        assert_eq!(named(&mut shell, "Formatting").toggled, Some(false));
        assert_eq!(named(&mut shell, "Spell check").toggled, Some(true));

        // A dropdown is named by what it sets, and its value is read separately:
        // "Dark" and "JetBrains Mono" do not say which setting they belong to.
        // Both halves are asserted by moving the settings, so what passes is the
        // name staying put while the value follows it.
        shell.theme = AppTheme::Light;
        shell.font_family = FAMILIES[1];
        shell.opening = OpenBehavior::ReplaceTab;
        let expected = [
            ("App theme", shell.theme.label().to_owned()),
            ("Font", shell.font_family.to_owned()),
            ("Opening files", shell.opening.label().to_owned()),
        ];
        for (name, value) in expected {
            let dropdown = named(&mut shell, name);
            assert_eq!(dropdown.role, Role::ComboBox, "{name}");
            assert_eq!(dropdown.value.as_deref(), Some(value.as_str()), "{name}");
        }
    }

    #[test]
    fn a_text_field_says_what_it_is_for() {
        let mut shell = Shell::new();

        // The editor is the document it shows.
        let editor = named_role(&mut shell, Role::TextInput, "Untitled");
        assert_eq!(editor.role, Role::TextInput);

        shell.find.open = true;
        shell.find.replace = true;
        assert_eq!(named(&mut shell, "Find what").role, Role::TextInput);
        assert_eq!(named(&mut shell, "Replace with").role, Role::TextInput);

        shell.find.open = false;
        shell.go_to.open = true;
        assert_eq!(named(&mut shell, "Line number").role, Role::TextInput);

        shell.go_to.open = false;
        shell.file_dialog = Some(FileDialog::open());
        assert_eq!(named(&mut shell, "File name").role, Role::TextInput);
        assert_eq!(named(&mut shell, "Encoding").role, Role::ComboBox);
        assert_eq!(
            named(&mut shell, "Up one level").role,
            Role::Button,
            "the folder-up arrow is a button, not a glyph"
        );
    }

    #[test]
    fn a_prompt_asks_its_question_in_words() {
        let mut shell = Shell::new();
        shell.prompt = Some(Prompt::UnsavedChanges { tab: 0 });

        let question = named(&mut shell, "Save");
        assert_eq!(question.role, Role::Button);
        // The question itself is a widget `egui` made, so it is already named;
        // this holds that it says the file, which is the part a reader needs.
        assert!(
            tree(&mut shell).iter().any(|node| node
                .value
                .as_deref()
                .is_some_and(|text| text.contains("\"Untitled\""))),
            "the prompt names the document it is about"
        );
    }

    #[test]
    fn a_document_shown_as_markdown_is_still_readable() {
        // The formatted view paints the document and has no text field in it, so
        // without a node of its own a reader finds the body of the window empty.
        let mut shell = Shell::new();
        shell.open_tab(DocKind::Markdown);
        let index = shell.active;
        append(&mut shell.tabs[index], "# Heading\n\nSome text.");
        shell.tabs[index].formatted = true;

        let document = named_role(&mut shell, Role::Document, "Untitled");
        assert_eq!(
            document.value.as_deref(),
            Some("# Heading\n\nSome text."),
            "the document's own text is what is shown, and what is exposed"
        );
        assert!(
            document.bounds.is_some(),
            "and it has a place on the screen"
        );

        // The source view is the other way round: the text is in the field, and a
        // second copy of it as a document would be read out twice.
        shell.tabs[index].formatted = false;
        assert!(
            tree(&mut shell)
                .into_iter()
                .all(|node| node.role != Role::Document),
            "the source view keeps its text in the text field"
        );
    }

    #[test]
    fn the_status_bar_reads_its_document_kind() {
        let mut shell = Shell::new();
        let kind = tree(&mut shell)
            .into_iter()
            .find(|node| node.value.as_deref() == Some("Plain text"))
            .expect("the painted document kind has a node of its own");
        assert_eq!(kind.role, Role::Label);
        assert!(kind.bounds.is_some(), "and a place on the screen");
    }

    /// A scratch directory, removed when the test ends. Named for the test, since
    /// every test in this binary shares one process id.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("notecraft-a11y-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("a scratch directory");
            Self(directory)
        }

        fn file(&self, name: &str, text: &str) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, text).expect("a file");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Types `text` at the end of a tab's document, the way an edit does.
    fn append(tab: &mut crate::ui::state::Tab, text: &str) {
        let end = tab.text().len();
        tab.document_mut().set_caret(end);
        tab.document_mut().insert_str(text);
    }
}
