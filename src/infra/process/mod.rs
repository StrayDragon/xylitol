//! Cross-platform process management.
//!
//! Provides shell discovery, process group management, and reliable
//! child process waiting. Used by `agent/bash_executor.rs` and
//! `agent/tools/bash.rs`.

pub mod child;
pub mod group;
pub mod shell;
