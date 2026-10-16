//! The menu-bar model: a resolved, ordered view of the registry ready for the
//! shell to render.
//!
//! The registry (`crate::registry`) is the data; this module turns it into the
//! tree a shell consumes, applying availability at build time and normalising
//! separators so a shell never has to. Building the model is pure, so the exact
//! menu a user will see can be asserted in a test.

use crate::availability::AvailabilityContext;
use crate::registry::{self, CommandId, CommandSpec, MenuBar, Toggle};
use std::fmt;

/// A top-level menu in the menu bar, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TopMenu {
    /// The `File` menu.
    File,
    /// The `Edit` menu.
    Edit,
    /// The `View` menu.
    View,
}

impl TopMenu {
    /// Every top-level menu, in display order.
    pub const ALL: [TopMenu; 3] = [TopMenu::File, TopMenu::Edit, TopMenu::View];

    /// The label shown in the menu bar.
    pub const fn label(self) -> &'static str {
        match self {
            TopMenu::File => "File",
            TopMenu::Edit => "Edit",
            TopMenu::View => "View",
        }
    }
}

impl fmt::Display for TopMenu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A node in a menu's static structure, before availability is applied.
#[derive(Debug, Clone, Copy)]
pub enum MenuNode {
    /// A command, referenced by its stable identifier.
    Command(CommandId),
    /// A separator line between groups.
    Separator,
    /// A submenu with a title and its own nodes.
    Submenu(&'static str, &'static [MenuNode]),
}

/// A resolved menu item, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    /// A command with its accelerator text and current enabled state.
    Command {
        /// The command being shown.
        spec: &'static CommandSpec,
        /// Accelerator text (for example `Ctrl+N`), if any.
        accelerator: Option<String>,
        /// Whether the item is clickable in the context it was built for.
        enabled: bool,
        /// The check state for a toggle item, or `None` for a plain command.
        checked: Option<bool>,
    },
    /// A separator line.
    Separator,
    /// A resolved submenu.
    Submenu {
        /// The submenu title.
        title: &'static str,
        /// The items inside it.
        items: Vec<MenuItem>,
    },
}

impl MenuItem {
    /// The command behind this item, if any.
    pub fn command(&self) -> Option<&'static CommandSpec> {
        match self {
            MenuItem::Command { spec, .. } => Some(spec),
            _ => None,
        }
    }

    /// Whether this item is a clickable command.
    pub fn is_enabled(&self) -> bool {
        matches!(self, MenuItem::Command { enabled: true, .. })
    }

    /// Whether this item is a toggle that is currently checked.
    pub fn is_checked(&self) -> bool {
        matches!(
            self,
            MenuItem::Command {
                checked: Some(true),
                ..
            }
        )
    }
}

/// A resolved top-level menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuModel {
    /// Which menu this is.
    pub menu: TopMenu,
    /// Its resolved contents.
    pub items: Vec<MenuItem>,
}

/// Builds every top-level menu for `context`, in display order.
pub fn build_menu_bar(context: AvailabilityContext) -> Vec<MenuModel> {
    registry::MENU_BAR
        .iter()
        .map(|bar| resolve(bar, context))
        .collect()
}

/// Builds a single top-level menu for `context`.
pub fn resolve(bar: &MenuBar, context: AvailabilityContext) -> MenuModel {
    MenuModel {
        menu: bar.menu,
        items: normalise(resolve_nodes(bar.nodes, context)),
    }
}

fn resolve_nodes(nodes: &'static [MenuNode], context: AvailabilityContext) -> Vec<MenuItem> {
    nodes
        .iter()
        .filter_map(|node| resolve_node(node, context))
        .collect()
}

fn resolve_node(node: &MenuNode, context: AvailabilityContext) -> Option<MenuItem> {
    match node {
        MenuNode::Command(id) => {
            let spec = registry::by_id(id.as_str())?;
            Some(MenuItem::Command {
                spec,
                accelerator: spec.accelerator(),
                enabled: spec.is_available(context),
                checked: spec.toggle.map(|toggle| checked(toggle, context)),
            })
        }
        MenuNode::Separator => Some(MenuItem::Separator),
        MenuNode::Submenu(title, children) => {
            let items = normalise(resolve_nodes(children, context));
            if items.is_empty() {
                // A submenu whose every entry was dropped is not worth showing.
                None
            } else {
                Some(MenuItem::Submenu { title, items })
            }
        }
    }
}

/// The current check state of a toggle item.
pub fn checked(toggle: Toggle, context: AvailabilityContext) -> bool {
    match toggle {
        Toggle::StatusBar => context.status_bar_visible,
        Toggle::WordWrap => context.word_wrap,
        Toggle::FormattedView => context.formatted_view,
    }
}

/// Whether every command inside `items` is disabled — a submenu like this is
/// shown greyed rather than opening.
pub fn all_disabled(items: &[MenuItem]) -> bool {
    let mut saw_command = false;
    for item in items {
        match item {
            MenuItem::Command { enabled, .. } => {
                saw_command = true;
                if *enabled {
                    return false;
                }
            }
            MenuItem::Submenu { items, .. } => {
                if !all_disabled(items) {
                    return false;
                }
            }
            MenuItem::Separator => {}
        }
    }
    saw_command
}

/// Collapses runs of separators to one and trims leading or trailing
/// separators, so the rendered menu never shows a doubled or dangling line.
fn normalise(items: Vec<MenuItem>) -> Vec<MenuItem> {
    let mut result: Vec<MenuItem> = Vec::with_capacity(items.len());
    for item in items {
        if matches!(item, MenuItem::Separator)
            && (result.is_empty() || matches!(result.last(), Some(MenuItem::Separator)))
        {
            continue;
        }
        result.push(item);
    }
    if matches!(result.last(), Some(MenuItem::Separator)) {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcut::Key;

    #[test]
    fn menu_bar_has_the_three_menus_in_order() {
        let bar = build_menu_bar(AvailabilityContext::initial());
        let titles: Vec<_> = bar.iter().map(|menu| menu.menu.label()).collect();
        assert_eq!(titles, vec!["File", "Edit", "View"]);
    }

    #[test]
    fn file_menu_starts_with_new_tab_and_ends_with_exit() {
        let bar = build_menu_bar(AvailabilityContext::initial());
        let file = &bar[0];
        let commands: Vec<_> = file.items.iter().filter_map(MenuItem::command).collect();
        assert_eq!(commands.first().unwrap().id.as_str(), "cmd.file.new_tab");
        assert_eq!(commands.last().unwrap().id.as_str(), "cmd.file.exit");
    }

    #[test]
    fn view_menu_has_the_zoom_submenu() {
        let bar = build_menu_bar(AvailabilityContext::initial());
        let view = bar.iter().find(|menu| menu.menu == TopMenu::View).unwrap();
        let zoom = view
            .items
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu { title, items } if *title == "Zoom" => Some(items),
                _ => None,
            })
            .expect("Zoom submenu present");
        assert_eq!(zoom.len(), 3);
        assert_eq!(zoom[0].command().unwrap().id.as_str(), "cmd.view.zoom_in");
    }

    #[test]
    fn accelerators_come_from_the_registry() {
        let bar = build_menu_bar(AvailabilityContext::initial());
        let file = &bar[0];
        let new_item = file
            .items
            .iter()
            .find(|item| item.command().map(|s| s.id.as_str()) == Some("cmd.file.new_tab"))
            .unwrap();
        match new_item {
            MenuItem::Command { accelerator, .. } => {
                assert_eq!(accelerator.as_deref(), Some("Ctrl+N"));
            }
            _ => panic!("expected a command"),
        }
    }

    #[test]
    fn selection_commands_are_greyed_until_there_is_a_selection() {
        let initial = build_menu_bar(AvailabilityContext::initial());
        let edit = initial.iter().find(|m| m.menu == TopMenu::Edit).unwrap();
        let copy = edit
            .items
            .iter()
            .find(|i| i.command().map(|s| s.id.as_str()) == Some("cmd.edit.copy"))
            .unwrap();
        assert!(!copy.is_enabled());

        let selected = AvailabilityContext {
            has_selection: true,
            ..AvailabilityContext::initial()
        };
        let bar = build_menu_bar(selected);
        let edit = bar.iter().find(|m| m.menu == TopMenu::Edit).unwrap();
        let copy = edit
            .items
            .iter()
            .find(|i| i.command().map(|s| s.id.as_str()) == Some("cmd.edit.copy"))
            .unwrap();
        assert!(copy.is_enabled());
    }

    #[test]
    fn view_toggles_report_their_check_state() {
        let bar = build_menu_bar(AvailabilityContext::initial());
        let view = bar.iter().find(|m| m.menu == TopMenu::View).unwrap();
        let status = view
            .items
            .iter()
            .find(|i| i.command().map(|s| s.id.as_str()) == Some("cmd.view.status_bar"))
            .unwrap();
        assert!(status.is_checked());
    }

    #[test]
    fn markdown_submenu_is_all_disabled_for_a_plain_document() {
        let bar = build_menu_bar(AvailabilityContext::initial());
        let view = bar.iter().find(|m| m.menu == TopMenu::View).unwrap();
        let md = view
            .items
            .iter()
            .find_map(|i| match i {
                MenuItem::Submenu { title, items } if *title == "Markdown" => Some(items),
                _ => None,
            })
            .expect("Markdown submenu present");
        assert!(all_disabled(md));
    }

    #[test]
    fn separators_are_normalised() {
        let collapsed = normalise(vec![
            MenuItem::Separator,
            MenuItem::Separator,
            command("cmd.file.open"),
            MenuItem::Separator,
            MenuItem::Separator,
            command("cmd.file.save"),
            MenuItem::Separator,
        ]);
        assert_eq!(collapsed.len(), 3);
        assert!(collapsed[0].command().is_some());
        assert!(matches!(collapsed[1], MenuItem::Separator));
        assert!(collapsed[2].command().is_some());
    }

    fn command(id: &str) -> MenuItem {
        MenuItem::Command {
            spec: registry::by_id(id).unwrap(),
            accelerator: None,
            enabled: true,
            checked: None,
        }
    }

    #[test]
    fn alt_f4_is_reported_for_exit() {
        let spec =
            registry::by_shortcut(crate::shortcut::Modifiers::ALT, Key::Function(4)).unwrap();
        assert_eq!(spec.accelerator().as_deref(), Some("Alt+F4"));
    }
}
