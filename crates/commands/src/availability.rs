//! When each command is enabled.
//!
//! Enablement is state, not a property of the command alone, so the registry
//! declares a rule per command and the shell supplies the state it knows. This
//! keeps the "why is this greyed out?" logic testable without a window.

/// The condition under which a command may be invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Availability {
    /// Always enabled.
    Always,
    /// Never enabled; used for placeholder entries that only hold a place.
    Never,
    /// Enabled when a document exists.
    Edited,
    /// Enabled only when the current document has unsaved changes.
    Dirty,
    /// Enabled only when there is a selection.
    Selection,
    /// Enabled only when the document is a Markdown document.
    Markdown,
    /// Enabled only when the AI service is reachable and configured.
    Service,
    /// Enabled only when more than one tab is open.
    MultipleTabs,
}

/// The shell state that availability rules are evaluated against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvailabilityContext {
    /// Whether at least one document is open.
    pub document_present: bool,
    /// Whether the current document has unsaved changes.
    pub dirty: bool,
    /// Whether the current selection is non-empty.
    pub has_selection: bool,
    /// Whether the current document is a Markdown document.
    pub markdown_enabled: bool,
    /// Whether the AI service is configured and reachable.
    pub service_available: bool,
    /// How many tabs are open on the current window.
    pub tab_count: usize,
    /// Whether the status bar is currently shown (the `View ▸ Status bar` check).
    pub status_bar_visible: bool,
    /// Whether word wrap is currently on (the `View ▸ Word wrap` check).
    pub word_wrap: bool,
    /// Whether a Markdown document is shown formatted rather than as source.
    pub formatted_view: bool,
}

impl AvailabilityContext {
    /// The state at first launch: one empty, clean, unselected plain document.
    pub const fn initial() -> Self {
        Self {
            document_present: true,
            dirty: false,
            has_selection: false,
            markdown_enabled: false,
            service_available: false,
            tab_count: 1,
            status_bar_visible: true,
            word_wrap: true,
            formatted_view: false,
        }
    }
}

impl Default for AvailabilityContext {
    fn default() -> Self {
        Self::initial()
    }
}

impl Availability {
    /// Whether a command with this rule may be invoked in `context`.
    pub const fn allows(self, context: AvailabilityContext) -> bool {
        match self {
            Availability::Always => true,
            Availability::Never => false,
            Availability::Edited => context.document_present,
            Availability::Dirty => context.document_present && context.dirty,
            Availability::Selection => context.document_present && context.has_selection,
            Availability::Markdown => context.document_present && context.markdown_enabled,
            Availability::Service => context.service_available,
            Availability::MultipleTabs => context.tab_count > 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_context_disables_document_dependent_commands() {
        let ctx = AvailabilityContext::initial();
        assert!(Availability::Always.allows(ctx));
        assert!(!Availability::Never.allows(ctx));
        assert!(Availability::Edited.allows(ctx));
        assert!(!Availability::Dirty.allows(ctx));
        assert!(!Availability::Selection.allows(ctx));
        assert!(!Availability::Markdown.allows(ctx));
        assert!(!Availability::Service.allows(ctx));
        assert!(!Availability::MultipleTabs.allows(ctx));
    }

    #[test]
    fn tab_and_selection_rules_track_state() {
        let ctx = AvailabilityContext {
            has_selection: true,
            tab_count: 3,
            ..AvailabilityContext::initial()
        };
        assert!(Availability::Selection.allows(ctx));
        assert!(Availability::MultipleTabs.allows(ctx));
    }

    #[test]
    fn rules_require_a_document() {
        let ctx = AvailabilityContext {
            document_present: false,
            has_selection: true,
            dirty: true,
            ..AvailabilityContext::initial()
        };
        assert!(!Availability::Edited.allows(ctx));
        assert!(!Availability::Selection.allows(ctx));
        assert!(!Availability::Dirty.allows(ctx));
        assert!(Availability::Always.allows(ctx));
    }
}
