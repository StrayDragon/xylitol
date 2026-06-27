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

pub mod bash;
pub mod compaction_config;
pub mod error;
pub mod lifecycle;
pub mod message;
pub mod model;
pub mod ports;
pub mod resource_types;
pub mod session_export;
pub mod session_types;
pub mod source_info;
pub mod types;
