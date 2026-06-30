//! Prompt/input shaping — system prompt assembly, slash commands, and
//! templates. The agent loop consumes these to build the input it sends to the
//! model.

pub mod commands;
pub mod system;
pub mod templates;

// Umbrella re-export: system-prompt construction is the subsystem's main entry.
pub(crate) use system::{SystemPromptOpts, build_system_prompt, collect_tool_snippets};
