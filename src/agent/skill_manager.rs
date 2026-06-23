//! SkillManager — skill activation, expansion, and command registration.
//!
//! Extracted from [`AgentSession`](super::session::AgentSession) to isolate
//! skill-related responsibilities into a focused component.

use crate::agent::commands::SlashCommandInfo;
use crate::infra::resource::SkillInfo;

/// Manages loaded skills — lookup, XML expansion, and slash command registration.
pub struct SkillManager {
    skills: Vec<SkillInfo>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// Register loaded skills.
    pub fn set_skills(&mut self, skills: Vec<SkillInfo>) {
        self.skills = skills;
    }

    /// Get the underlying skill list.
    pub fn skills(&self) -> &[SkillInfo] {
        &self.skills
    }

    /// Expand a skill invocation into an XML block for prompt injection.
    pub fn expand_command(&self, skill_name: &str, args: &str) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.name == skill_name)?;

        let content = std::fs::read_to_string(&skill.source_info.path).ok()?;

        // Strip YAML frontmatter
        let body = if content.starts_with("---") {
            if let Some(pos) = content.find("\n---") {
                content[pos + 4..].trim().to_string()
            } else {
                content.clone()
            }
        } else {
            content.clone()
        };

        let escaped_name = crate::infra::skills::loader::xml_escape(&skill.name);
        let escaped_location =
            crate::infra::skills::loader::xml_escape(&skill.source_info.path.to_string_lossy());
        let base_dir = skill
            .source_info
            .base_dir
            .as_ref()
            .map(|d| d.to_string_lossy().to_string())
            .unwrap_or_default();
        let escaped_base = crate::infra::skills::loader::xml_escape(&base_dir);

        let mut result = format!(
            r##"<skill name="{escaped_name}" location="{escaped_location}">
References are relative to {escaped_base}.

{body}"##,
        );

        if !args.is_empty() {
            result.push_str("\n\n");
            result.push_str(args);
        }

        result.push_str("\n</skill>");
        Some(result)
    }

    /// Register slash commands for all loaded skills.
    pub(crate) fn register_commands(&self) -> Vec<SlashCommandInfo> {
        self.skills
            .iter()
            .map(|skill| {
                let name = skill.name.clone();
                let desc = skill.description.clone().unwrap_or_default();
                SlashCommandInfo::new(
                    format!("skill:{name}"),
                    format!("Activate skill: {desc}"),
                    crate::agent::commands::SlashCommandSource::Skill,
                )
            })
            .collect()
    }
}
