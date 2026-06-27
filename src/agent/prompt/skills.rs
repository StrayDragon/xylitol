//! SkillManager — skill activation, expansion, and command registration.
//!
//! Extracted from [`AgentSession`](super::session::AgentSession) to isolate
//! skill-related responsibilities into a focused component.

use crate::agent::prompt::commands::SlashCommandInfo;
use crate::core::resource_types::SkillInfo;

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

        let escaped_name = crate::core::source_info::xml_escape(&skill.name);
        let escaped_location =
            crate::core::source_info::xml_escape(&skill.source_info.path.to_string_lossy());
        let base_dir = skill
            .source_info
            .base_dir
            .as_ref()
            .map(|d| d.to_string_lossy().to_string())
            .unwrap_or_default();
        let escaped_base = crate::core::source_info::xml_escape(&base_dir);

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
                    crate::agent::prompt::commands::SlashCommandSource::Skill,
                )
            })
            .collect()
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SkillManager;
    use crate::core::resource_types::SkillInfo;
    use crate::core::source_info::{SourceInfo, SourceOrigin, SourceScope};

    fn make_skill(name: &str, path: &str) -> SkillInfo {
        SkillInfo {
            name: name.into(),
            description: Some(format!("{name} description")),
            source_info: SourceInfo {
                path: std::path::PathBuf::from(path),
                source: "test".into(),
                scope: SourceScope::Project,
                origin: SourceOrigin::TopLevel,
                base_dir: Some(std::path::PathBuf::from("/base")),
            },
        }
    }

    #[test]
    fn new_skill_manager_empty() {
        let sm = SkillManager::new();
        assert!(sm.skills().is_empty());
    }

    #[test]
    fn set_skills_and_access() {
        let mut sm = SkillManager::new();
        let skills = vec![
            make_skill("review", "/path/to/review.md"),
            make_skill("test", "/path/to/test.md"),
        ];
        sm.set_skills(skills);
        assert_eq!(sm.skills().len(), 2);
        assert_eq!(sm.skills()[0].name, "review");
        assert_eq!(sm.skills()[1].name, "test");
    }

    #[test]
    fn expand_nonexistent_skill_returns_none() {
        let sm = SkillManager::new();
        assert!(sm.expand_command("nonexistent", "args").is_none());
    }

    #[test]
    fn expand_unreadable_skill_returns_none() {
        let mut sm = SkillManager::new();
        sm.set_skills(vec![make_skill("broken", "/nonexistent/path.md")]);
        // File doesn't exist, so expand returns None
        let result = sm.expand_command("broken", "some args");
        assert!(result.is_none());
    }

    #[test]
    fn register_commands_empty() {
        let sm = SkillManager::new();
        let cmds = sm.register_commands();
        assert!(cmds.is_empty());
    }

    #[test]
    fn register_commands_with_skills() {
        let mut sm = SkillManager::new();
        sm.set_skills(vec![make_skill("my-skill", "/p.md")]);
        let cmds = sm.register_commands();
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].name.contains("my-skill"));
        assert!(cmds[0].description.contains("my-skill description"));
    }

    #[test]
    fn register_commands_uses_skill_source() {
        let mut sm = SkillManager::new();
        sm.set_skills(vec![make_skill("x", "/x.md")]);
        let cmds = sm.register_commands();
        assert_eq!(cmds[0].name, "skill:x");
    }
}
