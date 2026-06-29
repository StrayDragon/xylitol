//! Runtime boundary for resource loading.

use crate::domain::resource_types::{AgentsFile, ResourceDiagnostic, SkillInfo};

/// Resource loader port — abstracts discovery of project context files,
/// prompt templates, skills, themes, and system prompts.
pub trait XyResourceLoader: Send + Sync {
    /// Get the loaded context files (AGENTS.md, CLAUDE.md).
    fn get_agents_files(&self) -> &[AgentsFile];
    /// Get loaded skills and their diagnostics.
    fn get_skills(&self) -> (&[SkillInfo], &[ResourceDiagnostic]);
    /// Get the discovered system prompt content.
    fn get_system_prompt(&self) -> Option<&str>;
    /// Get the discovered append system prompt content.
    fn get_append_system_prompt(&self) -> &[String];
}
