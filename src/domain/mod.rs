//! Domain vocabulary — fundamental data types, errors, and pure utilities
//! shared by [`agent`](crate::agent), [`infra`](crate::infra), and
//! [`runtime_protocol`](crate::runtime_protocol).
//!
//! # Layering
//!
//! ```text
//! interactive/ → agent/ → runtime_protocol/ ← infra/
//!                          ↑
//!                        domain/
//! ```
//!
//! - `domain/` has **zero crate-internal** layer deps; MAY depend on workspace
//!   package **DTO only** (`xylitol_ai_bridge::dto`) to compose LLM leaves.
//! - `runtime_protocol/` depends only on `domain/`.
//! - Both `agent/` and `infra/` depend on `domain/` + `runtime_protocol/`.
//! - `infra/` must **never** depend on `agent/`.
//! - domain/agent MUST NOT depend on bridge HTTP / vendor SDK types.

pub mod compaction_config;
pub mod error;
pub mod lifecycle;
pub mod llm_project;
pub mod message;
pub mod model;
pub mod resource_types;
pub mod session_types;
pub mod source_info;
pub mod text;
pub mod tool_result_quiet;
pub mod types;
