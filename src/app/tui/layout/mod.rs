//! Product page layout — slot machine over package components.
//!
//! Owns the fixed shell order (scrollback → queue → 待办栏 → toast → status → editor slot → footer)
//! and product theme wiring. Atomic widgets live in [`crate::app::tui::widgets`];
//! generic Editor/Markdown/TreeSelector stay in `xylitol_tui`.

mod dollar_skill_source;
mod fixed_zone_footprint;
mod models_picker;
mod root;
mod session_tree;
mod slash_catalog;
mod slots;
mod theme;

pub(crate) use fixed_zone_footprint::{
    DEFAULT_MAX_VISIBLE, IMPORT_SLOT, MCP_SLOT_BASE, MODELS_SLOT, RESUME_SLOT, THEMES_SLOT,
    TREE_SLOT, queue_strip_line_count, reserved_lower_fixed_zone, slot_body_budget,
};

pub(crate) use models_picker::{ModelPickerRow, PendingModelChoice, status_next_turn_cue_text};
pub(crate) use session_tree::map_session_tree_nodes;
#[cfg(test)]
pub(crate) use slash_catalog::product_slash_commands_for_editor;

#[cfg(test)]
pub(crate) use root::sample_tree_nodes_for_test;
pub use root::{UiRoot, install_ui_root_key_listeners, shared_ui_root_rebuild};
pub use slots::{EditorSlot, EditorSlotKind, ImportConfirmDecision};
pub use theme::LayoutTheme;

#[cfg(test)]
pub use root::build_root;
pub(crate) use session_tree::FilterMode;
