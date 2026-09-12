//! Cross-platform process management.
//!
//! Provides shell discovery, process group management, and reliable
//! child process waiting. Used by `infra/tools/bash.rs` and
//! `infra/bash_exec`.

pub mod group;
pub mod shell;
