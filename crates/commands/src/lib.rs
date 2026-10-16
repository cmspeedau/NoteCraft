//! NoteCraft command registry.
//!
//! This crate is the **single authoritative specification** of the
//! application's commands: their labels, menu placement, default shortcuts,
//! availability rules, and the roadmap milestone that first gives each one a
//! real effect.
//!
//! Nothing here talks to a window, a keyboard, or the operating system. The
//! registry is pure data plus the small amount of arithmetic needed to render
//! it, which means the entire menu and shortcut surface can be asserted in
//! tests that run on any host, and the shell only has to translate an event
//! into a lookup.
//!
//! ```rust
//! use nc_commands::{build_menu_bar, AvailabilityContext, by_shortcut, Modifiers, Key};
//!
//! let menu = build_menu_bar(AvailabilityContext::initial());
//! assert_eq!(menu.first().unwrap().menu.label(), "File");
//!
//! let open = by_shortcut(Modifiers::CTRL, Key::Letter('O')).unwrap();
//! assert_eq!(open.label, "Open");
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod availability;
mod menu;
mod registry;
mod shortcut;

pub use availability::{Availability, AvailabilityContext};
pub use menu::{MenuItem, MenuModel, MenuNode, TopMenu, all_disabled, build_menu_bar, checked};
pub use registry::{
    CommandId, CommandSpec, MENU_BAR, MenuBar, MenuPlacement, Milestone, REGISTRY, RegistryProblem,
    Toggle, by_id, by_menu_id, by_shortcut, commands_at, registry_integrity,
};
pub use shortcut::{Key, Modifiers, Shortcut};

/// The number of commands in the registry.
pub fn command_count() -> usize {
    REGISTRY.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_smoke() {
        assert_eq!(command_count(), REGISTRY.len());
        assert!(
            command_count() >= 40,
            "registry should cover the full command surface"
        );
        assert!(registry_integrity().is_empty());
        let menubar = build_menu_bar(AvailabilityContext::initial());
        assert_eq!(menubar.len(), 3);
    }

    #[test]
    fn menu_bar_matches_the_documented_structure() {
        let labels: Vec<_> = build_menu_bar(AvailabilityContext::initial())
            .iter()
            .map(|model| model.menu.label())
            .collect();
        assert_eq!(labels, ["File", "Edit", "View"]);
    }
}
