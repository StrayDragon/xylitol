//! # Preview scene fixtures
//!
//! Process-local catalog for `xylitol tui designing` (debug builds). Inject
//! lives in `app::tui::effects::preview` — not a product slash, not on the wire.
//!
//! Scene table: [`catalog::DEBUG_SCENES`].

mod catalog;
mod seed;

pub use catalog::{
    FixedZoneOp, PreviewInject, find_scene, list_note, resolve_scene_id, scene_catalog,
};
#[cfg(test)]
pub use seed::activity_fold_resume_stamped_entries;
pub use seed::seed_scene;
