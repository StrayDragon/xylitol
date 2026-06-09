//! Skill system — declarative skill definitions and lifecycle.
//!
//! Skills are loaded from YAML config and activated/deactivated at runtime.
//! When activated, a skill injects its `system_prompt_addon` into the agent
//! context and optionally restricts available tools.
//!
//! MCP (Model Context Protocol) enables dynamic tool loading from external
//! servers via stdio or SSE transport.

use std::collections::HashMap;

use crate::infra::config::types::AppConfig;

mod mcp;
#[allow(unused_imports)]
pub use mcp::{McpClientManager, McpToolAdapter};

/// A loaded skill definition.
#[derive(Clone, Debug)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub system_prompt_addon: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
}

/// Manages skill lifecycle: loading, activation, deactivation.
#[derive(Clone)]
pub struct SkillManager {
    skills: HashMap<String, Skill>,
    active: Vec<String>,
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            active: Vec::new(),
        }
    }

    /// Load skills from the app configuration.
    pub fn load(&mut self, config: &AppConfig) {
        let Some(ref skill_configs) = config.skills else {
            return;
        };
        for sc in skill_configs {
            self.skills.insert(
                sc.name.clone(),
                Skill {
                    name: sc.name.clone(),
                    description: sc.description.clone().unwrap_or_default(),
                    system_prompt_addon: sc.system_prompt_addon.clone(),
                    allowed_tools: sc.allowed_tools.clone(),
                },
            );
        }
    }

    /// Activate a skill by name. Returns `true` if newly activated.
    pub fn activate(&mut self, name: &str) -> bool {
        if !self.skills.contains_key(name) {
            return false;
        }
        if self.active.contains(&name.to_string()) {
            return false;
        }
        self.active.push(name.to_string());
        true
    }

    /// Deactivate a skill by name.
    pub fn deactivate(&mut self, name: &str) {
        self.active.retain(|n| n != name);
    }

    /// Returns the concatenated `system_prompt_addon` of all active skills.
    pub fn get_system_prompt_addon(&self) -> String {
        self.active
            .iter()
            .filter_map(|name| self.skills.get(name))
            .filter_map(|s| s.system_prompt_addon.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Combined `allowed_tools` across active skills.
    ///
    /// Returns `None` if any active skill has no restriction (allow all).
    pub fn get_allowed_tools(&self) -> Option<Vec<String>> {
        let mut combined = Vec::new();
        for name in &self.active {
            let Some(skill) = self.skills.get(name) else {
                continue;
            };
            match &skill.allowed_tools {
                None => return None,
                Some(tools) => combined.extend(tools.iter().cloned()),
            }
        }
        Some(combined)
    }

    /// Returns the names of currently active skills.
    pub fn active_skill_names(&self) -> &[String] {
        &self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> AppConfig {
        use crate::infra::config::types::SkillConfig;
        AppConfig {
            skills: Some(vec![
                SkillConfig {
                    name: "python".into(),
                    description: Some("Python development".into()),
                    system_prompt_addon: Some("Write Python code.".into()),
                    allowed_tools: Some(vec!["bash".into(), "read".into()]),
                },
                SkillConfig {
                    name: "testing".into(),
                    description: Some("Testing helper".into()),
                    system_prompt_addon: Some("Write tests.".into()),
                    allowed_tools: None,
                },
            ]),
            ..Default::default()
        }
    }

    #[test]
    fn test_skill_manager_new() {
        let sm = SkillManager::new();
        assert!(sm.active_skill_names().is_empty());
        assert_eq!(sm.get_system_prompt_addon(), "");
    }

    #[test]
    fn test_skill_manager_load_and_activate() {
        let config = sample_config();
        let mut sm = SkillManager::new();
        sm.load(&config);

        assert!(sm.activate("python"));
        assert!(!sm.activate("python")); // already active
        assert_eq!(sm.active_skill_names(), &["python"]);
        assert_eq!(sm.get_system_prompt_addon(), "Write Python code.");
    }

    #[test]
    fn test_skill_manager_activate_nonexistent() {
        let mut sm = SkillManager::new();
        assert!(!sm.activate("nonexistent"));
    }

    #[test]
    fn test_skill_manager_deactivate() {
        let config = sample_config();
        let mut sm = SkillManager::new();
        sm.load(&config);
        sm.activate("python");
        sm.activate("testing");
        sm.deactivate("python");
        assert_eq!(sm.active_skill_names(), &["testing"]);
    }

    #[test]
    fn test_skill_manager_get_allowed_tools() {
        let config = sample_config();
        let mut sm = SkillManager::new();
        sm.load(&config);
        sm.activate("python");
        assert_eq!(
            sm.get_allowed_tools(),
            Some(vec!["bash".into(), "read".into()])
        );
    }

    #[test]
    fn test_skill_manager_unrestricted_allows_all() {
        let config = sample_config();
        let mut sm = SkillManager::new();
        sm.load(&config);
        sm.activate("testing"); // allowed_tools: None → unrestricted
        assert!(sm.get_allowed_tools().is_none());
    }

    #[test]
    fn test_skill_manager_system_prompt_addon_multiple() {
        let config = sample_config();
        let mut sm = SkillManager::new();
        sm.load(&config);
        sm.activate("python");
        sm.activate("testing");
        let addon = sm.get_system_prompt_addon();
        assert!(addon.contains("Write Python code."));
        assert!(addon.contains("Write tests."));
    }
}
