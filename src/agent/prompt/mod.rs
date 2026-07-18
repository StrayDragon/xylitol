//! Prompt/input shaping — system prompt assembly, slash commands, and
//! templates. The agent loop consumes these to build the input it sends to the
//! model.

pub mod commands;
pub mod product_commands;
pub mod skill_expand;
pub mod system;
pub mod templates;

// Umbrella re-export: system-prompt construction is the subsystem's main entry.
pub(crate) use skill_expand::expand_skills_in_agent_messages;
pub use system::{SystemPromptOpts, build_system_prompt};
pub(crate) use system::{collect_tool_guidelines, collect_tool_snippets};
