//! Product page layout — slot machine over package components.
//!
//! Owns the fixed shell order (scrollback → queue → status → editor slot → footer)
//! and product theme wiring. Atomic widgets live in [`crate::app::tui::widgets`];
//! generic Editor/Markdown/TreeSelector stay in `xylitol_tui`.

mod root;
mod session_tree;
mod slots;
mod theme;

pub(crate) use session_tree::map_session_tree_nodes;

#[cfg(test)]
pub(crate) use root::sample_tree_nodes_for_test;
pub use root::{UiRoot, install_ui_root_key_listeners, shared_ui_root_rebuild};
pub use slots::EditorSlot;
pub use theme::LayoutTheme;

#[cfg(test)]
pub use root::build_root;
#[cfg(test)]
pub(crate) use session_tree::FilterMode;
