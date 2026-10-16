//! The command registry: every command the application knows about, with its
//! label, menu placement, default shortcut, availability rule, and the roadmap
//! milestone that first gives it a real effect.
//!
//! The registry is the single authoritative specification of the command
//! surface. The shell (menus, toolbar, keyboard) reads it rather than restating
//! commands, and [`registry_integrity`] is asserted by the test suite so the
//! registry stays internally consistent.

use crate::availability::Availability;
use crate::menu::{MenuNode, TopMenu};
use crate::shortcut::{Key, Modifiers, Shortcut};
use std::fmt;

/// A stable, human-readable command identifier (`cmd.<group>.<name>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(pub &'static str);

impl CommandId {
    /// The identifier text.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// A state field a command's check mark reflects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Toggle {
    /// The `View ▸ Status bar` check state.
    StatusBar,
    /// The `View ▸ Word wrap` check state.
    WordWrap,
    /// The `View ▸ Markdown ▸ Formatted view` check state.
    FormattedView,
}

/// The roadmap milestone that first gives a command a real effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Milestone(u8);

impl Milestone {
    /// Builds a milestone from its number (`2` for M2).
    pub const fn new(number: u8) -> Self {
        Self(number)
    }

    /// The milestone number.
    pub const fn number(self) -> u8 {
        self.0
    }
}

impl fmt::Display for Milestone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "M{}", self.0)
    }
}

/// Where a command appears in the menu bar, if it appears at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuPlacement {
    /// The top-level menu.
    pub menu: TopMenu,
    /// The submenu within it, if any.
    pub submenu: Option<&'static str>,
}

/// A single command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// Stable identifier.
    pub id: CommandId,
    /// Numeric identifier (used by the shell to route actions).
    pub menu_id: u16,
    /// Menu label, without accelerator text.
    pub label: &'static str,
    /// Menu placement, if the command is exposed in the menu bar.
    pub placement: Option<MenuPlacement>,
    /// Default shortcut.
    pub shortcut: Option<Shortcut>,
    /// When the command is enabled.
    pub availability: Availability,
    /// The check state this command reflects, for toggle items.
    pub toggle: Option<Toggle>,
    /// Milestone that first gives the command a real effect.
    pub milestone: Milestone,
}

impl CommandSpec {
    /// The accelerator text shown next to the label, e.g. `Ctrl+N`.
    pub fn accelerator(&self) -> Option<String> {
        self.shortcut.map(|shortcut| shortcut.to_string())
    }

    /// Whether the command is enabled in `context`.
    pub fn is_available(&self, context: crate::availability::AvailabilityContext) -> bool {
        self.availability.allows(context)
    }
}

/// `REGISTRY` is the full command registry.
pub const REGISTRY: &[CommandSpec] = &[
    // --- File -----------------------------------------------------------
    cmd(
        "cmd.file.new_tab",
        101,
        "New tab",
        menu(TopMenu::File),
        Some(CTRL_N),
        Availability::Always,
        6,
    ),
    cmd(
        "cmd.file.new_window",
        102,
        "New window",
        menu(TopMenu::File),
        Some(CTRL_SHIFT_N),
        Availability::Always,
        6,
    ),
    cmd(
        "cmd.file.new_markdown",
        103,
        "New Markdown tab",
        menu(TopMenu::File),
        None,
        Availability::Always,
        7,
    ),
    cmd(
        "cmd.file.open",
        104,
        "Open",
        menu(TopMenu::File),
        Some(CTRL_O),
        Availability::Always,
        4,
    ),
    cmd(
        "cmd.file.recent_empty",
        105,
        "No recent files",
        None,
        None,
        Availability::Never,
        6,
    ),
    cmd(
        "cmd.file.save",
        106,
        "Save",
        menu(TopMenu::File),
        Some(CTRL_S),
        Availability::Edited,
        4,
    ),
    cmd(
        "cmd.file.save_as",
        107,
        "Save as",
        menu(TopMenu::File),
        Some(CTRL_SHIFT_S),
        Availability::Edited,
        4,
    ),
    cmd(
        "cmd.file.save_all",
        108,
        "Save all",
        menu(TopMenu::File),
        Some(CTRL_ALT_S),
        Availability::MultipleTabs,
        6,
    ),
    cmd(
        "cmd.file.page_setup",
        109,
        "Page setup",
        menu(TopMenu::File),
        None,
        Availability::Edited,
        8,
    ),
    cmd(
        "cmd.file.print",
        110,
        "Print",
        menu(TopMenu::File),
        Some(CTRL_P),
        Availability::Edited,
        8,
    ),
    cmd(
        "cmd.file.close_tab",
        111,
        "Close tab",
        menu(TopMenu::File),
        Some(CTRL_W),
        Availability::Always,
        6,
    ),
    cmd(
        "cmd.file.close_window",
        112,
        "Close window",
        menu(TopMenu::File),
        Some(CTRL_SHIFT_W),
        Availability::Always,
        6,
    ),
    cmd(
        "cmd.file.exit",
        113,
        "Exit",
        menu(TopMenu::File),
        Some(ALT_F4),
        Availability::Always,
        2,
    ),
    // --- Edit -----------------------------------------------------------
    cmd(
        "cmd.edit.undo",
        201,
        "Undo",
        menu(TopMenu::Edit),
        Some(CTRL_Z),
        Availability::Edited,
        3,
    ),
    cmd(
        "cmd.edit.redo",
        202,
        "Redo",
        None,
        Some(CTRL_Y),
        Availability::Dirty,
        3,
    ),
    cmd(
        "cmd.edit.cut",
        203,
        "Cut",
        menu(TopMenu::Edit),
        Some(CTRL_X),
        Availability::Selection,
        5,
    ),
    cmd(
        "cmd.edit.copy",
        204,
        "Copy",
        menu(TopMenu::Edit),
        Some(CTRL_C),
        Availability::Selection,
        5,
    ),
    cmd(
        "cmd.edit.paste",
        205,
        "Paste",
        menu(TopMenu::Edit),
        Some(CTRL_V),
        Availability::Edited,
        5,
    ),
    cmd(
        "cmd.edit.delete",
        206,
        "Delete",
        menu(TopMenu::Edit),
        Some(DELETE),
        Availability::Selection,
        5,
    ),
    cmd(
        "cmd.md.clear",
        207,
        "Clear formatting",
        menu(TopMenu::Edit),
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.edit.define_bing",
        208,
        "Define with Bing",
        menu(TopMenu::Edit),
        Some(CTRL_E),
        Availability::Selection,
        9,
    ),
    cmd(
        "cmd.edit.find",
        209,
        "Find",
        menu(TopMenu::Edit),
        Some(CTRL_F),
        Availability::Edited,
        5,
    ),
    cmd(
        "cmd.edit.find_next",
        210,
        "Find next",
        menu(TopMenu::Edit),
        Some(F3),
        Availability::Edited,
        5,
    ),
    cmd(
        "cmd.edit.find_prev",
        211,
        "Find previous",
        menu(TopMenu::Edit),
        Some(SHIFT_F3),
        Availability::Edited,
        5,
    ),
    cmd(
        "cmd.edit.replace",
        212,
        "Replace",
        menu(TopMenu::Edit),
        Some(CTRL_H),
        Availability::Edited,
        5,
    ),
    cmd(
        "cmd.edit.go_to",
        213,
        "Go to",
        menu(TopMenu::Edit),
        Some(CTRL_G),
        Availability::Edited,
        5,
    ),
    cmd(
        "cmd.edit.select_all",
        214,
        "Select all",
        menu(TopMenu::Edit),
        Some(CTRL_A),
        Availability::Edited,
        5,
    ),
    cmd(
        "cmd.edit.time_date",
        215,
        "Time/Date",
        menu(TopMenu::Edit),
        Some(F5),
        Availability::Edited,
        5,
    ),
    // --- View -----------------------------------------------------------
    cmd_placed(
        "cmd.view.zoom_in",
        301,
        "Zoom in",
        MenuPlacement {
            menu: TopMenu::View,
            submenu: Some("Zoom"),
        },
        Some(CTRL_PLUS),
        Availability::Edited,
        2,
    ),
    cmd_placed(
        "cmd.view.zoom_out",
        302,
        "Zoom out",
        MenuPlacement {
            menu: TopMenu::View,
            submenu: Some("Zoom"),
        },
        Some(CTRL_MINUS),
        Availability::Edited,
        2,
    ),
    cmd_placed(
        "cmd.view.zoom_reset",
        303,
        "Restore default zoom",
        MenuPlacement {
            menu: TopMenu::View,
            submenu: Some("Zoom"),
        },
        Some(CTRL_ZERO),
        Availability::Edited,
        2,
    ),
    cmd_toggle(
        "cmd.view.status_bar",
        304,
        "Status bar",
        menu(TopMenu::View),
        None,
        Availability::Always,
        Toggle::StatusBar,
        2,
    ),
    cmd_toggle(
        "cmd.view.word_wrap",
        305,
        "Word wrap",
        menu(TopMenu::View),
        None,
        Availability::Edited,
        Toggle::WordWrap,
        3,
    ),
    cmd_placed(
        "cmd.view.md_source",
        306,
        "Source view",
        MenuPlacement {
            menu: TopMenu::View,
            submenu: Some("Markdown"),
        },
        None,
        Availability::Markdown,
        7,
    ),
    cmd_placed(
        "cmd.view.md_formatted",
        307,
        "Formatted view",
        MenuPlacement {
            menu: TopMenu::View,
            submenu: Some("Markdown"),
        },
        None,
        Availability::Markdown,
        7,
    ),
    // --- Format (toolbar) ----------------------------------------------
    cmd(
        "cmd.format.title",
        501,
        "Title",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.format.subtitle",
        502,
        "Subtitle",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.format.heading",
        503,
        "Heading",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.format.subheading",
        504,
        "Subheading",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.format.section",
        505,
        "Section",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.format.subsection",
        506,
        "Subsection",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.format.body",
        507,
        "Body",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.bullets",
        510,
        "Bulleted list",
        None,
        Some(CTRL_SHIFT_8),
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.numbering",
        511,
        "Numbered list",
        None,
        Some(CTRL_SHIFT_7),
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.indent_increase",
        512,
        "Increase indent",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.indent_decrease",
        513,
        "Decrease indent",
        None,
        None,
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.bold",
        520,
        "Bold",
        None,
        Some(CTRL_B),
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.italic",
        521,
        "Italic",
        None,
        Some(CTRL_I),
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.strike",
        522,
        "Strikethrough",
        None,
        Some(CTRL_SHIFT_X),
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.link",
        523,
        "Hyperlink",
        None,
        Some(CTRL_K),
        Availability::Markdown,
        7,
    ),
    cmd(
        "cmd.md.table",
        524,
        "Table",
        None,
        Some(CTRL_SHIFT_T),
        Availability::Markdown,
        7,
    ),
    // --- App chrome -----------------------------------------------------
    cmd(
        "cmd.app.settings",
        700,
        "Settings",
        None,
        None,
        Availability::Always,
        8,
    ),
    cmd(
        "cmd.app.feedback",
        701,
        "Send feedback",
        None,
        None,
        Availability::Always,
        9,
    ),
    cmd(
        "cmd.app.help",
        702,
        "Help",
        None,
        None,
        Availability::Always,
        9,
    ),
    cmd(
        "cmd.ai.rewrite",
        601,
        "Rewrite",
        None,
        None,
        Availability::Service,
        9,
    ),
    cmd(
        "cmd.ai.summarize",
        602,
        "Summarize",
        None,
        None,
        Availability::Service,
        9,
    ),
    cmd(
        "cmd.ai.write",
        603,
        "Write",
        None,
        None,
        Availability::Service,
        9,
    ),
];

/// Builds a registry entry for a command.
#[allow(clippy::too_many_arguments)]
const fn cmd(
    id: &'static str,
    menu_id: u16,
    label: &'static str,
    placement: Option<MenuPlacement>,
    shortcut: Option<Shortcut>,
    availability: Availability,
    milestone: u8,
) -> CommandSpec {
    CommandSpec {
        id: CommandId(id),
        menu_id,
        label,
        placement,
        shortcut,
        availability,
        toggle: None,
        milestone: Milestone::new(milestone),
    }
}

/// Like [`cmd`], but takes a full [`MenuPlacement`] so a command can live in a
/// submenu (for example `View ▸ Zoom`).
#[allow(clippy::too_many_arguments)]
const fn cmd_placed(
    id: &'static str,
    menu_id: u16,
    label: &'static str,
    placement: MenuPlacement,
    shortcut: Option<Shortcut>,
    availability: Availability,
    milestone: u8,
) -> CommandSpec {
    cmd(
        id,
        menu_id,
        label,
        Some(placement),
        shortcut,
        availability,
        milestone,
    )
}

/// Like [`cmd`], but the command carries a check mark reflecting `toggle`.
#[allow(clippy::too_many_arguments)]
const fn cmd_toggle(
    id: &'static str,
    menu_id: u16,
    label: &'static str,
    placement: Option<MenuPlacement>,
    shortcut: Option<Shortcut>,
    availability: Availability,
    toggle: Toggle,
    milestone: u8,
) -> CommandSpec {
    let mut spec = cmd(
        id,
        menu_id,
        label,
        placement,
        shortcut,
        availability,
        milestone,
    );
    spec.toggle = Some(toggle);
    spec
}

/// Shorthand for a top-level menu placement.
const fn menu(menu: TopMenu) -> Option<MenuPlacement> {
    Some(MenuPlacement {
        menu,
        submenu: None,
    })
}

// Shortcut constants, so the table above reads like the menus.
const DELETE: Shortcut = Shortcut::new(Modifiers::NONE, Key::Delete);
const CTRL_N: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('N'));
const CTRL_SHIFT_N: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('N'));
const CTRL_O: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('O'));
const CTRL_S: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('S'));
const CTRL_SHIFT_S: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('S'));
const CTRL_ALT_S: Shortcut = Shortcut::new(Modifiers::CTRL_ALT, Key::Letter('S'));
const CTRL_P: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('P'));
const ALT_F4: Shortcut = Shortcut::new(Modifiers::ALT, Key::Function(4));
const CTRL_W: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('W'));
const CTRL_SHIFT_W: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('W'));
const CTRL_Z: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('Z'));
const CTRL_Y: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('Y'));
const CTRL_X: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('X'));
const CTRL_C: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('C'));
const CTRL_V: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('V'));
const CTRL_A: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('A'));
const CTRL_F: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('F'));
const CTRL_H: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('H'));
const CTRL_G: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('G'));
const CTRL_E: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('E'));
const F3: Shortcut = Shortcut::new(Modifiers::NONE, Key::Function(3));
const SHIFT_F3: Shortcut = Shortcut::new(Modifiers::SHIFT, Key::Function(3));
const F5: Shortcut = Shortcut::new(Modifiers::NONE, Key::Function(5));
const CTRL_PLUS: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Plus);
const CTRL_MINUS: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Minus);
const CTRL_ZERO: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(0));
const CTRL_B: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('B'));
const CTRL_I: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('I'));
const CTRL_SHIFT_X: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('X'));
const CTRL_K: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('K'));
const CTRL_SHIFT_T: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('T'));
const CTRL_SHIFT_7: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Digit(7));
const CTRL_SHIFT_8: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Digit(8));

/// Looks a command up by its stable identifier.
pub fn by_id(id: &str) -> Option<&'static CommandSpec> {
    REGISTRY.iter().find(|spec| spec.id.as_str() == id)
}

/// Looks a command up by its numeric identifier.
pub fn by_menu_id(menu_id: u16) -> Option<&'static CommandSpec> {
    REGISTRY.iter().find(|spec| spec.menu_id == menu_id)
}

/// Looks a command up by the shortcut that triggers it.
pub fn by_shortcut(modifiers: Modifiers, key: Key) -> Option<&'static CommandSpec> {
    REGISTRY
        .iter()
        .find(|spec| spec.shortcut == Some(Shortcut::new(modifiers, key)))
}

/// Commands that first take effect in `milestone`.
pub fn commands_at(milestone: Milestone) -> impl Iterator<Item = &'static CommandSpec> {
    REGISTRY
        .iter()
        .filter(move |spec| spec.milestone == milestone)
}

/// A problem found while checking the registry against its own invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryProblem {
    /// Two commands share a stable identifier.
    DuplicateId(&'static str),
    /// Two commands share a numeric identifier.
    DuplicateMenuId(u16),
    /// A command claims a shortcut that another command already claims.
    DuplicateShortcut(String),
    /// A command is scheduled for milestone 0.
    MilestoneRegression(&'static str),
    /// A toggle command carries a shortcut, which would make it a plain command.
    ToggleWithShortcut(&'static str),
}

/// Checks the registry against its structural invariants and returns every
/// problem found. An empty result means the registry is internally consistent.
pub fn registry_integrity() -> Vec<RegistryProblem> {
    let mut problems = Vec::new();

    for (index, spec) in REGISTRY.iter().enumerate() {
        let id = spec.id.as_str();
        if REGISTRY[..index].iter().any(|other| other.id == spec.id) {
            problems.push(RegistryProblem::DuplicateId(id));
        }
        if REGISTRY[..index]
            .iter()
            .any(|other| other.menu_id == spec.menu_id)
        {
            problems.push(RegistryProblem::DuplicateMenuId(spec.menu_id));
        }
        if let Some(shortcut) = spec.shortcut {
            let text = shortcut.to_string();
            if REGISTRY[..index]
                .iter()
                .any(|other| other.shortcut.map(|other| other.to_string()) == Some(text.clone()))
            {
                problems.push(RegistryProblem::DuplicateShortcut(text));
            }
        }
    }

    for spec in REGISTRY {
        if spec.milestone.number() == 0 {
            problems.push(RegistryProblem::MilestoneRegression(spec.id.as_str()));
        }
        if spec.toggle.is_some() && spec.shortcut.is_some() {
            problems.push(RegistryProblem::ToggleWithShortcut(spec.id.as_str()));
        }
    }

    problems
}

/// The menu tree, as an ordered list of nodes per top-level menu.
pub const MENU_BAR: &[MenuBar] = &[
    MenuBar {
        menu: TopMenu::File,
        nodes: &[
            MenuNode::Command(CommandId("cmd.file.new_tab")),
            MenuNode::Command(CommandId("cmd.file.new_window")),
            MenuNode::Command(CommandId("cmd.file.new_markdown")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.open")),
            MenuNode::Submenu(
                "Recent",
                &[MenuNode::Command(CommandId("cmd.file.recent_empty"))],
            ),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.save")),
            MenuNode::Command(CommandId("cmd.file.save_as")),
            MenuNode::Command(CommandId("cmd.file.save_all")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.page_setup")),
            MenuNode::Command(CommandId("cmd.file.print")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.close_tab")),
            MenuNode::Command(CommandId("cmd.file.close_window")),
            MenuNode::Command(CommandId("cmd.file.exit")),
        ],
    },
    MenuBar {
        menu: TopMenu::Edit,
        nodes: &[
            MenuNode::Command(CommandId("cmd.edit.undo")),
            MenuNode::Command(CommandId("cmd.edit.cut")),
            MenuNode::Command(CommandId("cmd.edit.copy")),
            MenuNode::Command(CommandId("cmd.edit.paste")),
            MenuNode::Command(CommandId("cmd.edit.delete")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.md.clear")),
            MenuNode::Command(CommandId("cmd.edit.define_bing")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.edit.find")),
            MenuNode::Command(CommandId("cmd.edit.find_next")),
            MenuNode::Command(CommandId("cmd.edit.find_prev")),
            MenuNode::Command(CommandId("cmd.edit.replace")),
            MenuNode::Command(CommandId("cmd.edit.go_to")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.edit.select_all")),
            MenuNode::Command(CommandId("cmd.edit.time_date")),
        ],
    },
    MenuBar {
        menu: TopMenu::View,
        nodes: &[
            MenuNode::Submenu(
                "Zoom",
                &[
                    MenuNode::Command(CommandId("cmd.view.zoom_in")),
                    MenuNode::Command(CommandId("cmd.view.zoom_out")),
                    MenuNode::Command(CommandId("cmd.view.zoom_reset")),
                ],
            ),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.view.status_bar")),
            MenuNode::Command(CommandId("cmd.view.word_wrap")),
            MenuNode::Separator,
            MenuNode::Submenu(
                "Markdown",
                &[
                    MenuNode::Command(CommandId("cmd.view.md_formatted")),
                    MenuNode::Command(CommandId("cmd.view.md_source")),
                ],
            ),
        ],
    },
];

/// A top-level menu and the ordered nodes inside it.
#[derive(Debug, Clone, Copy)]
pub struct MenuBar {
    /// Which top-level menu this is.
    pub menu: TopMenu,
    /// The menu's contents.
    pub nodes: &'static [MenuNode],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::availability::AvailabilityContext;

    #[test]
    fn registry_is_internally_consistent() {
        let problems = registry_integrity();
        assert!(problems.is_empty(), "registry problems: {problems:?}");
    }

    #[test]
    fn every_screenshot_menu_command_is_registered() {
        let expected = [
            // File
            "New tab",
            "New window",
            "New Markdown tab",
            "Open",
            "Save",
            "Save as",
            "Save all",
            "Page setup",
            "Print",
            "Close tab",
            "Close window",
            "Exit",
            // Edit
            "Undo",
            "Cut",
            "Copy",
            "Paste",
            "Delete",
            "Clear formatting",
            "Define with Bing",
            "Find",
            "Find next",
            "Find previous",
            "Replace",
            "Go to",
            "Select all",
            "Time/Date",
            // View
            "Zoom in",
            "Zoom out",
            "Restore default zoom",
            "Status bar",
            "Word wrap",
        ];
        for label in expected {
            assert!(
                REGISTRY.iter().any(|spec| spec.label == label),
                "command {label:?} is missing from the registry"
            );
        }
    }

    #[test]
    fn shortcut_lookup_finds_the_documented_commands() {
        let cases = [
            (Modifiers::CTRL, Key::Letter('N'), "cmd.file.new_tab"),
            (
                Modifiers::CTRL_SHIFT,
                Key::Letter('N'),
                "cmd.file.new_window",
            ),
            (Modifiers::CTRL, Key::Letter('O'), "cmd.file.open"),
            (Modifiers::CTRL, Key::Letter('S'), "cmd.file.save"),
            (Modifiers::CTRL_SHIFT, Key::Letter('S'), "cmd.file.save_as"),
            (Modifiers::CTRL_ALT, Key::Letter('S'), "cmd.file.save_all"),
            (Modifiers::CTRL, Key::Letter('W'), "cmd.file.close_tab"),
            (Modifiers::CTRL, Key::Letter('F'), "cmd.edit.find"),
            (Modifiers::NONE, Key::Function(3), "cmd.edit.find_next"),
            (Modifiers::SHIFT, Key::Function(3), "cmd.edit.find_prev"),
            (Modifiers::CTRL, Key::Plus, "cmd.view.zoom_in"),
            (Modifiers::CTRL, Key::Minus, "cmd.view.zoom_out"),
            (Modifiers::CTRL, Key::Digit(0), "cmd.view.zoom_reset"),
        ];
        for (modifiers, key, id) in cases {
            let found = by_shortcut(modifiers, key)
                .unwrap_or_else(|| panic!("no command for {modifiers}{key}"));
            assert_eq!(found.id.as_str(), id);
        }
    }

    #[test]
    fn alt_f4_is_bound_to_exit() {
        let found = by_shortcut(Modifiers::ALT, Key::Function(4)).expect("Alt+F4 bound");
        assert_eq!(found.id.as_str(), "cmd.file.exit");
    }

    #[test]
    fn menu_ids_round_trip_through_the_registry() {
        for spec in REGISTRY {
            let found = by_menu_id(spec.menu_id).expect("menu id resolves");
            assert_eq!(found.id, spec.id);
        }
    }

    #[test]
    fn initial_context_greys_selection_and_markdown_commands() {
        let ctx = AvailabilityContext::initial();
        let state = |id: &str| by_id(id).expect("registered").is_available(ctx);
        assert!(state("cmd.file.new_tab"));
        assert!(state("cmd.view.status_bar"));
        assert!(!state("cmd.edit.copy"));
        assert!(!state("cmd.edit.cut"));
        assert!(!state("cmd.md.bold"));
        assert!(!state("cmd.ai.rewrite"));
    }

    #[test]
    fn ai_commands_stay_blocked_on_the_service() {
        for spec in REGISTRY
            .iter()
            .filter(|spec| spec.id.as_str().starts_with("cmd.ai."))
        {
            assert_eq!(spec.availability, Availability::Service, "{}", spec.id);
        }
    }
}
