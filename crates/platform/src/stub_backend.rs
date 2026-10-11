//! Headless platform backend.
//!
//! Used on hosts without a native window system (Linux CI, developer machines
//! that only build the headless crates). It does not block: the "window" is a
//! no-op and the process returns immediately so tests and tooling can run.
//!
//! It still resolves the menu bar from `nc-commands`, so the headless host
//! exercises the same command surface the native shell renders — a build that
//! cannot present a window can still prove the menu model is well-formed.

use nc_commands::{AvailabilityContext, MenuItem, MenuModel};

use crate::{PlatformError, RunOutcome, StartupOptions};

/// The state at first launch: one empty, clean, unselected document.
fn launch_context() -> AvailabilityContext {
    AvailabilityContext::initial()
}

/// Counts the clickable commands in a resolved menu, for the headless log.
fn command_count(items: &[MenuItem]) -> usize {
    items
        .iter()
        .map(|item| match item {
            MenuItem::Command { .. } => 1,
            MenuItem::Submenu { items, .. } => command_count(items),
            MenuItem::Separator => 0,
        })
        .sum()
}

pub(crate) fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    tracing::warn!(
        target: "nc_platform",
        "headless backend: no native window is available on this host; \
         the UI will not be presented"
    );
    tracing::info!(
        target: "nc_platform",
        title = %options.title,
        files = ?options.open_paths,
        "startup acknowledged (headless)"
    );

    let context = launch_context();
    for MenuModel { menu, items } in nc_commands::build_menu_bar(context) {
        tracing::debug!(
            target: "nc_platform",
            menu = menu.label(),
            items = command_count(&items),
            "menu resolved (headless)"
        );
    }

    Ok(RunOutcome { exit_code: 0 })
}
