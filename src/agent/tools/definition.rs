//! Agent-level tool presentation.
//!
//! Contains [`XyToolDefinition`] — a presentation wrapper around
//! [`XyTool`](crate::runtime_protocol::XyTool) that includes prompt metadata
//! and source information. The core abstractions ([`XyModel`](crate::runtime_protocol::XyModel),
//! [`XyTool`](crate::runtime_protocol::XyTool), etc.) live in [`crate::runtime_protocol`].

use serde::Serialize;

use crate::domain::source_info::SourceInfo;
use crate::domain::types::XyToolSchema;
use crate::runtime_protocol::{XyTool, XyToolExecutionMode};

// ── XyToolDefinition ──────────────────────────────────────────────────

/// Unified tool definition — standardises prompt display for all tools.
#[derive(Debug, Clone, Serialize)]
pub struct XyToolDefinition {
    pub schema: XyToolSchema,
    pub prompt_snippet: Option<String>,
    pub prompt_guidelines: Vec<String>,
    pub execution_mode: XyToolExecutionMode,
    pub source_info: Option<SourceInfo>,
}

impl Default for XyToolDefinition {
    fn default() -> Self {
        Self {
            schema: XyToolSchema {
                name: String::new(),
                description: String::new(),
                parameters: serde_json::Value::Null,
            },
            prompt_snippet: None,
            prompt_guidelines: Vec::new(),
            execution_mode: XyToolExecutionMode::Parallel,
            source_info: None,
        }
    }
}

impl<'a> From<&'a dyn XyTool> for XyToolDefinition {
    fn from(tool: &'a dyn XyTool) -> Self {
        let prompt_snippet = tool.prompt_snippet().map(|s| s.to_string()).or_else(|| {
            let desc = tool.description();
            if desc.is_empty() {
                None
            } else {
                let snippet: String = desc.chars().take(80).collect();
                if desc.len() > 80 {
                    Some(format!("{snippet}…"))
                } else {
                    Some(snippet)
                }
            }
        });

        let prompt_guidelines = tool
            .prompt_guidelines()
            .iter()
            .map(|s| s.to_string())
            .collect();

        Self {
            schema: XyToolSchema {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters_schema(),
            },
            prompt_snippet,
            prompt_guidelines,
            execution_mode: tool.execution_mode(),
            source_info: None,
        }
    }
}
