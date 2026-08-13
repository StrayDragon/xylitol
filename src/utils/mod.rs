//! Leaf utilities shared across layers.
//!
//! Pure helpers with **no** dependency on `agent` / `infra` / `app` / `protocol`.
//! Not part of the curated `Xy*` library surface.

pub(crate) mod sync;
pub mod text;

pub(crate) use sync::lock_mutex;
pub use text::{today_yyyy_mm_dd, xml_escape};
