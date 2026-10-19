//! Leaf helpers used by the ReAct loop (no plugin / envelope deps).

pub(crate) mod sync;
pub mod text;
pub(crate) mod thought_clock;

pub(crate) use sync::lock_mutex;
pub use text::xml_escape;
#[allow(dead_code)]
pub(crate) use thought_clock::{StreamNode, StreamNodeClock};
