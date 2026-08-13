//! # Hand-test debug fixtures (c710)
//!
//! **Delete this whole module** to remove `/debug` scenes: also drop
//! `PendingSlash::DebugScene`, `XyDriver::load_debug_scene`, TUI slash/completion
//! wiring in `layout/root.rs` / `commands.rs` / `effects.rs`, and E2E cases that
//! call `/debug …`.
//!
//! Slash + inline completion are registered only under `cfg(debug_assertions)`.
//!
//! Scene table: [`catalog::DEBUG_SCENES`].

mod catalog;
mod seed;

#[cfg(debug_assertions)]
pub use catalog::completion_catalog;
pub use catalog::{ChromeOp, PreviewInject, find_scene, list_note, resolve_scene_id};
#[cfg(test)]
pub use seed::activity_fold_resume_stamped_entries;
pub use seed::seed_scene;
