//! Extension system — custom tools and event hooks.
//!
//! Aligns with pi's extensions/types.ts and extensions/loader.ts.
//!
//! Key components:
//! - `ToolDefinition` trait — LLM-callable custom tools
//! - `Extension` trait — tool registration + lifecycle event hooks
//! - `ExtensionContext` — cwd, session_manager, model_registry, signal, abort
//! - `ExtensionLoader` — load extensions from directories or factories
//! - `ExtensionRunner` — lifecycle management
//!
//! Scope (c70): custom tools + event hooks only.
//! Not implemented: UI extensions, commands, keyboard shortcuts, CLI flags,
//! message renderers, provider registration.

#![allow(dead_code)]

pub mod loader;
pub mod types;

pub use loader::{Extension, ExtensionEventResult, ExtensionLoader, ExtensionRunner};
pub use types::{ExtensionContext, ExtensionEvent, RegisteredTool, ToolDefinition, ToolSourceInfo};
