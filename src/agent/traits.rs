//! Agent-level tool presentation.
//!
//! Contains [`ToolDefinition`] — a presentation wrapper around
//! [`XyTool`](crate::core::traits::XyTool) that includes prompt metadata
//! and source information. The core abstractions ([`XyModel`](crate::core::traits::XyModel),
//! [`XyTool`](crate::core::traits::XyTool), etc.) live in [`crate::core::traits`].

use serde::Serialize;
use serde_json::Value;

use crate::core::traits::{ToolExecutionMode, XyTool};

// ── ToolDefinition ──────────────────────────────────────────────────

/// Unified tool definition — standardises prompt display for all tools.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub prompt_snippet: Option<String>,
    pub prompt_guidelines: Vec<String>,
    pub execution_mode: ToolExecutionMode,
    pub source_info: Option<crate::infra::source_info::SourceInfo>,
}

impl Default for ToolDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            parameters: Value::Null,
            prompt_snippet: None,
            prompt_guidelines: Vec::new(),
            execution_mode: ToolExecutionMode::Parallel,
            source_info: None,
        }
    }
}

impl<'a> From<&'a dyn XyTool> for ToolDefinition {
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
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters: tool.parameters_schema(),
            prompt_snippet,
            prompt_guidelines,
            execution_mode: tool.execution_mode(),
            source_info: None,
        }
    }
}
