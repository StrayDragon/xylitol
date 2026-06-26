//! Domain core — zero-dependency layer containing the fundamental vocabulary
//! shared by both [`agent`](crate::agent) and [`infra`](crate::infra).
//!
//! # Layering
//!
//! ```text
//! interactive/ → agent/ → core/ ← infra/
//! ```
//!
//! - `core/` depends on **nothing** else in the crate (only external crates).
//! - Both `agent/` and `infra/` depend on `core/`.
//! - `infra/` must **never** depend on `agent/`.

pub mod error;
pub mod message;
pub mod model;
pub mod traits;
pub mod types;
