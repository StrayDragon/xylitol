//! Cross-platform process management.
//!
//! Provides shell discovery, process group management, and reliable
//! child process waiting. Used by `agent/runtime/bash.rs` and
//! `agent/tools/bash.rs`.

pub mod group;
pub mod shell;
