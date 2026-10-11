//! egui/eframe window backend.
//!
//! The portable twin of [`crate::windows_backend`]: the same window with the
//! same registry-driven menu bar, and the same routing of a clicked menu id
//! back through `nc-commands`. Nothing about the command surface is restated
//! here — it is read from the registry exactly as the Win32 backend does.
//!
//! It sits behind the `egui` feature so the shipped Windows build keeps using
//! the native backend by default, while the shell can also be presented (and
//! its menu surface exercised) on hosts without Win32.
//!
//! As in the Win32 backend, the client area (edit control, toolbar, tabs,
//! status bar) arrives in M2/M3; this renders the window and its menu bar.

use nc_commands::{AvailabilityContext, CommandSpec, MenuItem};

use crate::{PlatformError, RunOutcome, StartupOptions};

/// The state the menu bar is rendered for.
struct ShellState {
    /// The registry context the menu is currently rendered for.
    context: AvailabilityContext,
}

impl ShellState {
    /// The state at first launch: one empty, clean, unselected document.
    fn new() -> Self {
        Self {
            context: AvailabilityContext::initial(),
        }
    }
}

/// Routes a clicked menu id through the registry.
///
/// The registry says whether the command is available and which milestone
/// first gives it a real effect; the *effect* itself belongs to the handler
/// introduced with that milestone, so here the route is recorded and nothing
/// else happens. Returns the spec it resolved, or `None` for an id the
/// registry does not know.
fn route_command(state: &ShellState, menu_id: u16) -> Option<&'static CommandSpec> {
    match nc_commands::by_menu_id(menu_id) {
        Some(spec) => {
            tracing::info!(
                target: "nc_platform",
                command = spec.id.as_str(),
                label = spec.label,
                milestone = %spec.milestone,
                enabled = spec.is_available(state.context),
                "menu command routed"
            );
            Some(spec)
        }
        None => {
            tracing::warn!(target: "nc_platform", menu_id, "unknown menu command id");
            None
        }
    }
}

/// Draws the menu bar from the resolved registry model, returning the menu ids
/// the user clicked during this frame.
fn draw_menu_bar(ui: &mut egui::Ui, context: AvailabilityContext) -> Vec<u16> {
    let mut clicked = Vec::new();
    egui::MenuBar::new().ui(ui, |ui| {
        for model in nc_commands::build_menu_bar(context) {
            let items = &model.items;
            // The menu body only runs while its popup is open, so an unopened
            // menu contributes no clicks.
            let response = ui.menu_button(model.menu.label(), |ui| append_items(ui, items));
            if let Some(inner) = response.inner {
                clicked.extend(inner);
            }
        }
    });
    clicked
}

/// Adds a resolved item list to the open menu, recursively for submenus.
fn append_items(ui: &mut egui::Ui, items: &[MenuItem]) -> Vec<u16> {
    let mut clicked = Vec::new();
    for item in items {
        match item {
            MenuItem::Separator => {
                ui.separator();
            }
            MenuItem::Command { spec, enabled, .. } => {
                // Availability decides whether the item is clickable, exactly
                // as `MF_GRAYED` does on Win32.
                if ui
                    .add_enabled(*enabled, egui::Button::new(spec.label))
                    .clicked()
                {
                    clicked.push(spec.menu_id);
                }
            }
            MenuItem::Submenu { title, items } => {
                let response = ui.menu_button(*title, |ui| append_items(ui, items));
                if let Some(inner) = response.inner {
                    clicked.extend(inner);
                }
            }
        }
    }
    clicked
}

/// The eframe application: the window, driven by `ui` once per frame.
struct ShellApp {
    state: ShellState,
}

impl eframe::App for ShellApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut clicked = Vec::new();

        egui::Panel::top("menu_bar").show(ui, |ui| {
            clicked = draw_menu_bar(ui, self.state.context);
        });

        // The client area is deliberately empty, matching the Win32 backend.
        egui::CentralPanel::default().show(ui, |_ui| {});

        for menu_id in clicked {
            route_command(&self.state, menu_id);
        }
    }
}

pub(crate) fn run(options: StartupOptions) -> Result<RunOutcome, PlatformError> {
    let state = ShellState::new();

    tracing::info!(
        target: "nc_platform",
        title = %options.title,
        files = ?options.open_paths,
        commands = nc_commands::command_count(),
        "main window created"
    );

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(options.title),
        ..Default::default()
    };

    eframe::run_native(
        "NoteCraft",
        native_options,
        Box::new(move |_cc| Ok(Box::new(ShellApp { state }))),
    )
    .map_err(|err| PlatformError::GuiWindow(err.to_string()))?;

    // eframe returns when the last window closes; the loop exits normally
    // rather than through a WM_QUIT-carried exit code.
    Ok(RunOutcome { exit_code: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ShellState {
        ShellState::new()
    }

    /// The menu bar builds and draws a real frame: the top panel emits shapes
    /// and an unopened menu reports no clicks.
    #[test]
    fn menu_bar_renders_a_frame() {
        let ctx = egui::Context::default();
        let mut clicks = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::Panel::top("menu_bar").show(ui, |ui| {
                clicks = Some(draw_menu_bar(ui, AvailabilityContext::initial()));
            });
        });
        // A caller that ignores the texture deltas must clear them, or the
        // delta's debug assertion fires on drop.
        output.textures_delta.clear();

        assert!(!output.shapes.is_empty(), "menu bar painted nothing");
        assert_eq!(clicks.expect("panel ran"), Vec::<u16>::new());
    }

    /// Every item the registry puts in the menu bar is reachable: a click on
    /// it routes to the same spec `by_menu_id` resolves.
    #[test]
    fn routed_ids_resolve_to_registry_specs() {
        let state = state();
        let models = nc_commands::build_menu_bar(state.context);

        // One model per bar the registry declares, in the registry's order.
        assert_eq!(models.len(), nc_commands::MENU_BAR.len());
        for (model, bar) in models.iter().zip(nc_commands::MENU_BAR) {
            assert_eq!(model.menu, bar.menu);
        }

        let mut routed = Vec::new();
        for model in &models {
            let mut items = Vec::new();
            collect(&model.items, &mut items);
            for item in &items {
                let MenuItem::Command { spec, enabled, .. } = item else {
                    continue;
                };
                assert_eq!(
                    spec.placement.map(|placement| placement.menu),
                    Some(model.menu),
                    "{} is shown under the wrong menu",
                    spec.id.as_str()
                );
                let resolved = route_command(&state, spec.menu_id).expect("known menu id");
                assert_eq!(
                    resolved.id, spec.id,
                    "menu id {} routed nowhere",
                    spec.menu_id
                );
                assert_eq!(resolved.is_available(state.context), *enabled);
                routed.push(spec.menu_id);
            }
        }

        // The bar is a subset of the registry, and it shows every command the
        // registry places there — no more, no fewer, each id routed once.
        let mut unique = routed.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), routed.len(), "a menu id is routed twice");

        let placed = nc_commands::REGISTRY
            .iter()
            .filter(|spec| spec.placement.is_some())
            .count();
        assert_eq!(routed.len(), placed, "the bar shows a different set");
    }

    #[test]
    fn unknown_menu_id_routes_to_nothing() {
        assert!(route_command(&state(), u16::MAX).is_none());
    }

    /// Collects the command items in a resolved item list, submenus included.
    fn collect(items: &[MenuItem], found: &mut Vec<MenuItem>) {
        for item in items {
            match item {
                MenuItem::Separator => {}
                MenuItem::Command { .. } => found.push(item.clone()),
                MenuItem::Submenu { items, .. } => collect(items, found),
            }
        }
    }
}
