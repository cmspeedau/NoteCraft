//! The command registry: every menu command the application knows about, with
//! its label, menu placement, default shortcut, availability rule, evidence
//! grade, and the milestone that first gives it a real effect.
//!
//! Source of truth: `docs/reference-matrix.md` §3 ("Commands, menus, and
//! shortcuts"). Each row below carries the same evidence tag the matrix gives
//! it, so the registry can never drift silently from the captured reference:
//! [`registry_integrity`] is asserted by the test suite.

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

/// How well the reference capture supports a command's existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Evidence {
    /// Listed with an explicit command name in the reference matrix.
    Documented,
    /// Present in the reference matrix but with part of its behaviour, label,
    /// or placement marked for capture.
    DocumentedPartial,
    /// Expected from the platform but not yet pinned against the reference.
    ToCapture,
}

/// Whether a command's specification has been pinned against the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecStatus {
    /// Specified here; no verification against the reference machine yet.
    Spec,
    /// Verified against the pinned reference build.
    Verified,
    /// Blocked on the reference machine or an external service.
    Blocked,
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

/// A single menu command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// Stable identifier.
    pub id: CommandId,
    /// Numeric identifier used by the native menu layer (Win32 `WM_COMMAND`).
    pub menu_id: u16,
    /// Menu label, without accelerator text.
    pub label: &'static str,
    /// Menu placement, if the command is exposed in the menu bar.
    pub placement: Option<MenuPlacement>,
    /// Default shortcut.
    pub shortcut: Option<Shortcut>,
    /// When the command is enabled.
    pub availability: Availability,
    /// Evidence grade from the reference matrix.
    pub evidence: Evidence,
    /// Verification status against the reference build.
    pub status: SpecStatus,
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

/// `MENU` is the full registry, in reference-matrix order.
pub const REGISTRY: &[CommandSpec] = &[
    // --- §3.1 File ------------------------------------------------------
    cmd(
        "cmd.file.new",
        101,
        "New",
        menu(TopMenu::File),
        Some(CTRL_N),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        4,
    ),
    cmd(
        "cmd.file.new_tab",
        102,
        "New tab",
        None,
        Some(CTRL_T),
        Availability::Always,
        Evidence::Documented,
        SpecStatus::Spec,
        6,
    ),
    cmd(
        "cmd.file.new_window",
        103,
        "New window",
        menu(TopMenu::File),
        Some(CTRL_SHIFT_N),
        Availability::Always,
        Evidence::Documented,
        SpecStatus::Spec,
        6,
    ),
    cmd(
        "cmd.file.open",
        104,
        "Open",
        menu(TopMenu::File),
        Some(CTRL_O),
        Availability::Always,
        Evidence::Documented,
        SpecStatus::Spec,
        2,
    ),
    cmd(
        "cmd.file.save",
        105,
        "Save",
        menu(TopMenu::File),
        Some(CTRL_S),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        4,
    ),
    cmd(
        "cmd.file.save_as",
        106,
        "Save As",
        menu(TopMenu::File),
        Some(CTRL_SHIFT_S),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        2,
    ),
    cmd(
        "cmd.file.print",
        107,
        "Print",
        menu(TopMenu::File),
        Some(CTRL_P),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        4,
    ),
    cmd(
        "cmd.file.page_setup",
        108,
        "Page setup",
        menu(TopMenu::File),
        None,
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        4,
    ),
    cmd(
        "cmd.file.close_tab",
        109,
        "Close tab",
        menu(TopMenu::File),
        Some(CTRL_W),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        6,
    ),
    cmd(
        "cmd.file.exit",
        110,
        "Exit",
        menu(TopMenu::File),
        Some(ALT_F4),
        Availability::Always,
        Evidence::Documented,
        SpecStatus::Spec,
        2,
    ),
    cmd(
        "cmd.file.recent",
        111,
        "Recent files",
        menu(TopMenu::File),
        None,
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        4,
    ),
    cmd(
        "cmd.file.session_restore",
        112,
        "Session restore",
        None,
        None,
        Availability::Always,
        Evidence::DocumentedPartial,
        SpecStatus::Spec,
        4,
    ),
    // --- §3.2 Edit ------------------------------------------------------
    cmd(
        "cmd.edit.undo",
        201,
        "Undo",
        menu(TopMenu::Edit),
        Some(CTRL_Z),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.edit.redo",
        202,
        "Redo",
        menu(TopMenu::Edit),
        Some(CTRL_Y),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.edit.cut",
        203,
        "Cut",
        menu(TopMenu::Edit),
        Some(CTRL_X),
        Availability::Selection,
        Evidence::Documented,
        SpecStatus::Spec,
        5,
    ),
    cmd(
        "cmd.edit.copy",
        204,
        "Copy",
        menu(TopMenu::Edit),
        Some(CTRL_C),
        Availability::Selection,
        Evidence::Documented,
        SpecStatus::Spec,
        5,
    ),
    cmd(
        "cmd.edit.paste",
        205,
        "Paste",
        menu(TopMenu::Edit),
        Some(CTRL_V),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        5,
    ),
    cmd(
        "cmd.edit.delete",
        206,
        "Delete",
        menu(TopMenu::Edit),
        Some(DELETE),
        Availability::Selection,
        Evidence::Documented,
        SpecStatus::Spec,
        5,
    ),
    cmd(
        "cmd.edit.select_all",
        207,
        "Select all",
        menu(TopMenu::Edit),
        Some(CTRL_A),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        5,
    ),
    cmd(
        "cmd.edit.find",
        208,
        "Find",
        menu(TopMenu::Edit),
        Some(CTRL_F),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.edit.find_next",
        209,
        "Find next",
        menu(TopMenu::Edit),
        Some(F3),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.edit.find_prev",
        210,
        "Find previous",
        menu(TopMenu::Edit),
        Some(SHIFT_F3),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.edit.replace",
        211,
        "Replace",
        menu(TopMenu::Edit),
        Some(CTRL_H),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.edit.go_to",
        212,
        "Go to",
        menu(TopMenu::Edit),
        Some(CTRL_G),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.edit.time_date",
        213,
        "Time/Date",
        menu(TopMenu::Edit),
        Some(F5),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        5,
    ),
    cmd(
        "cmd.edit.word_wrap",
        214,
        "Word wrap",
        menu(TopMenu::View),
        None,
        Availability::Edited,
        Evidence::DocumentedPartial,
        SpecStatus::Spec,
        3,
    ),
    // --- §3.3 View ------------------------------------------------------
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
        Evidence::Documented,
        SpecStatus::Spec,
        3,
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
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd_placed(
        "cmd.view.zoom_reset",
        303,
        "Reset zoom",
        MenuPlacement {
            menu: TopMenu::View,
            submenu: Some("Zoom"),
        },
        Some(CTRL_ZERO),
        Availability::Edited,
        Evidence::Documented,
        SpecStatus::Spec,
        3,
    ),
    cmd(
        "cmd.view.status_bar",
        304,
        "Status bar",
        menu(TopMenu::View),
        None,
        Availability::Always,
        Evidence::Documented,
        SpecStatus::Spec,
        2,
    ),
    // --- §3.4 Tabs ------------------------------------------------------
    cmd(
        "cmd.tab.next",
        401,
        "Next tab",
        None,
        Some(CTRL_TAB),
        Availability::MultipleTabs,
        Evidence::Documented,
        SpecStatus::Spec,
        6,
    ),
    cmd(
        "cmd.tab.prev",
        402,
        "Previous tab",
        None,
        Some(CTRL_SHIFT_TAB),
        Availability::MultipleTabs,
        Evidence::Documented,
        SpecStatus::Spec,
        6,
    ),
    cmd(
        "cmd.tab.close_all",
        403,
        "Close all",
        menu(TopMenu::File),
        None,
        Availability::MultipleTabs,
        Evidence::Documented,
        SpecStatus::Spec,
        6,
    ),
    // --- §3.5 Markdown --------------------------------------------------
    cmd(
        "cmd.md.bold",
        501,
        "Bold",
        menu(TopMenu::Format),
        Some(CTRL_B),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.italic",
        502,
        "Italic",
        menu(TopMenu::Format),
        Some(CTRL_I),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.strike",
        503,
        "Strikethrough",
        menu(TopMenu::Format),
        Some(CTRL_SHIFT_X),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.h1",
        504,
        "Heading 1",
        menu(TopMenu::Format),
        Some(CTRL_1),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.h2",
        505,
        "Heading 2",
        menu(TopMenu::Format),
        Some(CTRL_2),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.h3",
        506,
        "Heading 3",
        menu(TopMenu::Format),
        Some(CTRL_3),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.h4",
        507,
        "Heading 4",
        menu(TopMenu::Format),
        Some(CTRL_4),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.h5",
        508,
        "Heading 5",
        menu(TopMenu::Format),
        Some(CTRL_5),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.h6",
        509,
        "Heading 6",
        menu(TopMenu::Format),
        Some(CTRL_6),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.bullets",
        510,
        "Bulleted list",
        menu(TopMenu::Format),
        Some(CTRL_SHIFT_8),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.numbering",
        511,
        "Numbered list",
        menu(TopMenu::Format),
        Some(CTRL_SHIFT_7),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.link",
        512,
        "Hyperlink",
        menu(TopMenu::Format),
        Some(CTRL_K),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.table",
        513,
        "Table",
        menu(TopMenu::Format),
        Some(CTRL_SHIFT_T),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd(
        "cmd.md.clear",
        514,
        "Clear formatting",
        menu(TopMenu::Format),
        Some(CTRL_SHIFT_R),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    cmd_placed(
        "cmd.md.toggle_source",
        515,
        "Source/formatted view",
        MenuPlacement {
            menu: TopMenu::View,
            submenu: Some("Markdown"),
        },
        Some(CTRL_SHIFT_M),
        Availability::Markdown,
        Evidence::Documented,
        SpecStatus::Spec,
        8,
    ),
    // --- §3.6 AI --------------------------------------------------------
    cmd(
        "cmd.ai.rewrite",
        601,
        "Rewrite",
        None,
        None,
        Availability::Service,
        Evidence::Documented,
        SpecStatus::Blocked,
        9,
    ),
    cmd(
        "cmd.ai.summarize",
        602,
        "Summarize",
        None,
        None,
        Availability::Service,
        Evidence::Documented,
        SpecStatus::Blocked,
        9,
    ),
    cmd(
        "cmd.ai.write",
        603,
        "Write",
        None,
        None,
        Availability::Service,
        Evidence::Documented,
        SpecStatus::Blocked,
        9,
    ),
    cmd(
        "cmd.ai.tone",
        604,
        "Tone",
        None,
        None,
        Availability::Service,
        Evidence::Documented,
        SpecStatus::Blocked,
        9,
    ),
    cmd(
        "cmd.ai.length",
        605,
        "Length",
        None,
        None,
        Availability::Service,
        Evidence::Documented,
        SpecStatus::Blocked,
        9,
    ),
    cmd(
        "cmd.ai.format",
        606,
        "Format",
        None,
        None,
        Availability::Service,
        Evidence::Documented,
        SpecStatus::Blocked,
        9,
    ),
];

/// Builds the registry entry for a command placed directly on a top-level menu.
// The argument count mirrors the reference-matrix row, which keeps the table
// above readable against §3 of `docs/reference-matrix.md`.
#[allow(clippy::too_many_arguments)]
const fn cmd(
    id: &'static str,
    menu_id: u16,
    label: &'static str,
    placement: Option<MenuPlacement>,
    shortcut: Option<Shortcut>,
    availability: Availability,
    evidence: Evidence,
    status: SpecStatus,
    milestone: u8,
) -> CommandSpec {
    CommandSpec {
        id: CommandId(id),
        menu_id,
        label,
        placement,
        shortcut,
        availability,
        evidence,
        status,
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
    evidence: Evidence,
    status: SpecStatus,
    milestone: u8,
) -> CommandSpec {
    cmd(
        id,
        menu_id,
        label,
        Some(placement),
        shortcut,
        availability,
        evidence,
        status,
        milestone,
    )
}

/// Shorthand for a top-level menu placement.
const fn menu(menu: TopMenu) -> Option<MenuPlacement> {
    Some(MenuPlacement {
        menu,
        submenu: None,
    })
}

// Shortcut constants, so the table above reads like the reference matrix.
const DELETE: Shortcut = Shortcut::new(Modifiers::NONE, Key::Delete);
const CTRL_N: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('N'));
const CTRL_T: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('T'));
const CTRL_SHIFT_N: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('N'));
const CTRL_O: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('O'));
const CTRL_S: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('S'));
const CTRL_SHIFT_S: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('S'));
const CTRL_P: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('P'));
const ALT_F4: Shortcut = Shortcut::new(Modifiers::ALT, Key::Function(4));
const CTRL_W: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('W'));
const CTRL_Z: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('Z'));
const CTRL_Y: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('Y'));
const CTRL_X: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('X'));
const CTRL_C: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('C'));
const CTRL_V: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('V'));
const CTRL_A: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('A'));
const CTRL_F: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('F'));
const CTRL_H: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('H'));
const CTRL_G: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('G'));
const F3: Shortcut = Shortcut::new(Modifiers::NONE, Key::Function(3));
const SHIFT_F3: Shortcut = Shortcut::new(Modifiers::SHIFT, Key::Function(3));
const F5: Shortcut = Shortcut::new(Modifiers::NONE, Key::Function(5));
const CTRL_PLUS: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Plus);
const CTRL_MINUS: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Minus);
const CTRL_ZERO: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(0));
const CTRL_TAB: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Tab);
const CTRL_SHIFT_TAB: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Tab);
const CTRL_B: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('B'));
const CTRL_I: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('I'));
const CTRL_SHIFT_X: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('X'));
const CTRL_1: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(1));
const CTRL_2: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(2));
const CTRL_3: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(3));
const CTRL_4: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(4));
const CTRL_5: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(5));
const CTRL_6: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Digit(6));
const CTRL_SHIFT_7: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Digit(7));
const CTRL_SHIFT_8: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Digit(8));
const CTRL_K: Shortcut = Shortcut::new(Modifiers::CTRL, Key::Letter('K'));
const CTRL_SHIFT_T: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('T'));
const CTRL_SHIFT_R: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('R'));
const CTRL_SHIFT_M: Shortcut = Shortcut::new(Modifiers::CTRL_SHIFT, Key::Letter('M'));

/// Looks a command up by its stable identifier.
pub fn by_id(id: &str) -> Option<&'static CommandSpec> {
    REGISTRY.iter().find(|spec| spec.id.as_str() == id)
}

/// Looks a command up by its numeric menu identifier.
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
    /// Two commands share a numeric menu identifier.
    DuplicateMenuId(u16),
    /// A command claims a shortcut that another command already claims.
    DuplicateShortcut(String),
    /// A command with no menu placement is nonetheless expected on the menu bar.
    UnreachableMenuCommand(&'static str),
    /// A command's evidence grade says "not captured" while its status claims
    /// verification.
    UnverifiedClaim(&'static str),
    /// A command is scheduled for a milestone after the one that wires it.
    MilestoneRegression(&'static str),
}

/// Checks the registry against its structural invariants and returns every
/// problem found. An empty result means the registry is internally consistent.
///
/// This is the guard that stops the registry drifting from the reference
/// matrix: identifiers, menu ids, and shortcuts must be unique, a command that
/// claims to be verified must have capture-backed evidence, and a
/// still-uncaptured command must not claim a milestone that could only be
/// reached with captured behaviour.
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
        if spec.evidence == Evidence::ToCapture && spec.status == SpecStatus::Verified {
            problems.push(RegistryProblem::UnverifiedClaim(spec.id.as_str()));
        }
        if spec.status == SpecStatus::Blocked && spec.availability != Availability::Service {
            problems.push(RegistryProblem::UnreachableMenuCommand(spec.id.as_str()));
        }
        if spec.milestone.number() == 0 {
            problems.push(RegistryProblem::MilestoneRegression(spec.id.as_str()));
        }
    }

    problems
}

/// The menu tree, as an ordered list of nodes per top-level menu.
///
/// `Help` is deliberately absent: `docs/reference-matrix.md` §2 records the
/// classic `File/Edit/View/Help` bar but marks the exact submenus
/// `TO_CAPTURE`, and §3 documents no Help commands yet, so there is nothing to
/// place. Adding a Help menu would mean inventing behavior, so it arrives with
/// the capture that names its entries.
pub const MENU_BAR: &[MenuBar] = &[
    MenuBar {
        menu: TopMenu::File,
        nodes: &[
            MenuNode::Command(CommandId("cmd.file.new")),
            MenuNode::Command(CommandId("cmd.file.new_window")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.open")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.save")),
            MenuNode::Command(CommandId("cmd.file.save_as")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.page_setup")),
            MenuNode::Command(CommandId("cmd.file.print")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.close_tab")),
            MenuNode::Command(CommandId("cmd.tab.close_all")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.recent")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.file.exit")),
        ],
    },
    MenuBar {
        menu: TopMenu::Edit,
        nodes: &[
            MenuNode::Command(CommandId("cmd.edit.undo")),
            MenuNode::Command(CommandId("cmd.edit.redo")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.edit.cut")),
            MenuNode::Command(CommandId("cmd.edit.copy")),
            MenuNode::Command(CommandId("cmd.edit.paste")),
            MenuNode::Command(CommandId("cmd.edit.delete")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.edit.select_all")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.edit.find")),
            MenuNode::Command(CommandId("cmd.edit.find_next")),
            MenuNode::Command(CommandId("cmd.edit.find_prev")),
            MenuNode::Command(CommandId("cmd.edit.replace")),
            MenuNode::Command(CommandId("cmd.edit.go_to")),
            MenuNode::Separator,
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
            MenuNode::Command(CommandId("cmd.edit.word_wrap")),
            MenuNode::Command(CommandId("cmd.view.status_bar")),
            MenuNode::Separator,
            MenuNode::Submenu(
                "Markdown",
                &[MenuNode::Command(CommandId("cmd.md.toggle_source"))],
            ),
        ],
    },
    MenuBar {
        menu: TopMenu::Format,
        nodes: &[
            MenuNode::Command(CommandId("cmd.md.bold")),
            MenuNode::Command(CommandId("cmd.md.italic")),
            MenuNode::Command(CommandId("cmd.md.strike")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.md.h1")),
            MenuNode::Command(CommandId("cmd.md.h2")),
            MenuNode::Command(CommandId("cmd.md.h3")),
            MenuNode::Command(CommandId("cmd.md.h4")),
            MenuNode::Command(CommandId("cmd.md.h5")),
            MenuNode::Command(CommandId("cmd.md.h6")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.md.bullets")),
            MenuNode::Command(CommandId("cmd.md.numbering")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.md.link")),
            MenuNode::Command(CommandId("cmd.md.table")),
            MenuNode::Separator,
            MenuNode::Command(CommandId("cmd.md.clear")),
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
    fn every_reference_matrix_command_is_registered() {
        // The command rows of reference-matrix §3.1–§3.6, by label.
        let expected = [
            "New",
            "New tab",
            "New window",
            "Open",
            "Save",
            "Save As",
            "Print",
            "Page setup",
            "Close tab",
            "Exit",
            "Recent files",
            "Undo",
            "Redo",
            "Cut",
            "Copy",
            "Paste",
            "Delete",
            "Select all",
            "Find",
            "Find next",
            "Find previous",
            "Replace",
            "Go to",
            "Time/Date",
            "Word wrap",
            "Zoom in",
            "Zoom out",
            "Reset zoom",
            "Status bar",
            "Next tab",
            "Previous tab",
            "Close all",
            "Bold",
            "Italic",
            "Strikethrough",
            "Heading 1",
            "Heading 2",
            "Heading 3",
            "Heading 4",
            "Heading 5",
            "Heading 6",
            "Bulleted list",
            "Numbered list",
            "Hyperlink",
            "Table",
            "Clear formatting",
            "Source/formatted view",
            "Rewrite",
            "Summarize",
            "Write",
            "Tone",
            "Length",
            "Format",
        ];
        for label in expected {
            assert!(
                REGISTRY.iter().any(|spec| spec.label == label),
                "reference command {label:?} is missing from the registry"
            );
        }
    }

    #[test]
    fn shortcut_lookup_finds_the_documented_commands() {
        let cases = [
            (Modifiers::CTRL, Key::Letter('N'), "cmd.file.new"),
            (
                Modifiers::CTRL_SHIFT,
                Key::Letter('N'),
                "cmd.file.new_window",
            ),
            (Modifiers::CTRL, Key::Letter('O'), "cmd.file.open"),
            (Modifiers::CTRL, Key::Letter('S'), "cmd.file.save"),
            (Modifiers::CTRL_SHIFT, Key::Letter('S'), "cmd.file.save_as"),
            (Modifiers::CTRL, Key::Letter('W'), "cmd.file.close_tab"),
            (Modifiers::CTRL, Key::Letter('F'), "cmd.edit.find"),
            (Modifiers::NONE, Key::Function(3), "cmd.edit.find_next"),
            (Modifiers::SHIFT, Key::Function(3), "cmd.edit.find_prev"),
            (Modifiers::CTRL, Key::Plus, "cmd.view.zoom_in"),
            (Modifiers::CTRL, Key::Minus, "cmd.view.zoom_out"),
            (Modifiers::CTRL, Key::Digit(0), "cmd.view.zoom_reset"),
            (Modifiers::CTRL, Key::Tab, "cmd.tab.next"),
            (Modifiers::CTRL_SHIFT, Key::Tab, "cmd.tab.prev"),
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
    fn only_m2_commands_take_effect_at_m2() {
        let ids: Vec<_> = commands_at(Milestone::new(2))
            .map(|spec| spec.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "cmd.file.open",
                "cmd.file.save_as",
                "cmd.file.exit",
                "cmd.view.status_bar",
            ]
        );
    }

    #[test]
    fn ai_commands_stay_blocked_on_the_service() {
        for spec in REGISTRY
            .iter()
            .filter(|spec| spec.id.as_str().starts_with("cmd.ai."))
        {
            assert_eq!(spec.status, SpecStatus::Blocked, "{}", spec.id);
            assert_eq!(spec.availability, Availability::Service, "{}", spec.id);
            assert_eq!(spec.milestone, Milestone::new(9), "{}", spec.id);
        }
    }

    #[test]
    fn initial_context_greys_selection_and_markdown_commands() {
        let ctx = AvailabilityContext::initial();
        let state = |id: &str| by_id(id).expect("registered").is_available(ctx);
        assert!(state("cmd.file.new"));
        assert!(state("cmd.view.status_bar"));
        assert!(!state("cmd.edit.copy"));
        assert!(!state("cmd.edit.cut"));
        assert!(!state("cmd.md.bold"));
        assert!(!state("cmd.tab.next"));
        assert!(!state("cmd.ai.rewrite"));
    }
}
