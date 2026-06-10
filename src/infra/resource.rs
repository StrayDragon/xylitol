//! ResourceLoader — project context, prompt templates, and skill discovery.
//!
//! Aligns with pi's resource-loader.ts. Loads:
//! - Project context files (AGENTS.md, CLAUDE.md) by walking up from cwd
//! - Prompt templates from global (~/.xylitol/prompts/) and project (.xylitol/prompts/)
//! - Skills via existing SkillManager integration

use std::path::{Path, PathBuf};

use crate::infra::config::types::AppConfig;
use crate::infra::skills::SkillManager;

/// Result of loading prompt templates.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Template name (filename without .md extension).
    pub name: String,
    /// Template content (markdown body, after frontmatter).
    pub content: String,
    /// Optional description from frontmatter.
    pub description: Option<String>,
    /// Optional argument hint from frontmatter.
    pub argument_hint: Option<String>,
    /// Source path for display.
    pub source_path: PathBuf,
}

/// Aggregates all resources needed by the agent session.
pub struct ResourceLoader {
    /// Current working directory.
    cwd: PathBuf,
    /// Global agent directory (~/.xylitol/).
    agent_dir: PathBuf,
}

impl ResourceLoader {
    /// Create a new ResourceLoader.
    ///
    /// `agent_dir` is typically `~/.xylitol/`.
    /// `cwd` is the project root directory.
    pub fn new(cwd: PathBuf, agent_dir: PathBuf) -> Self {
        Self { cwd, agent_dir }
    }

    /// Default agent directory: ~/.xylitol/
    pub fn default_agent_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
    }

    // ── Context files ───────────────────────────────────────────────

    /// Load project context files by walking up from `cwd` to `/`.
    ///
    /// Looks for `AGENTS.md` and `CLAUDE.md` at each directory level.
    /// The first found file of each name is used (closest to cwd).
    ///
    /// Returns `Vec<(file_name, file_content)>`.
    pub fn load_context_files(&self) -> Vec<(String, String)> {
        let candidate_names = ["AGENTS.md", "CLAUDE.md"];
        let mut found: Vec<(String, String)> = Vec::new();

        // Walk up from cwd to /
        let mut current = Some(self.cwd.as_path());

        while let Some(dir) = current {
            for &name in &candidate_names {
                // Skip if we already found this file
                if found.iter().any(|(n, _)| n == name) {
                    continue;
                }
                let file_path = dir.join(name);
                if file_path.is_file()
                    && let Ok(content) = std::fs::read_to_string(&file_path)
                {
                    found.push((name.to_string(), content));
                }
            }

            current = dir.parent();
        }

        // Ensure AGENTS.md comes before CLAUDE.md (consistent ordering)
        found.sort_by_key(|(name, _)| {
            if name == "AGENTS.md" {
                0
            } else if name == "CLAUDE.md" {
                1
            } else {
                2
            }
        });

        found
    }

    // ── Prompt templates ────────────────────────────────────────────

    /// Load prompt templates from global and project directories.
    ///
    /// Searches:
    /// 1. `~/.xylitol/prompts/*.md` (global)
    /// 2. `<cwd>/.xylitol/prompts/*.md` (project)
    ///
    /// Project templates override global templates with the same name.
    pub fn load_prompt_templates(&self) -> Vec<PromptTemplate> {
        let mut templates: Vec<PromptTemplate> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Load from global dir first (lower priority)
        let global_prompts_dir = self.agent_dir.join("prompts");
        if global_prompts_dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&global_prompts_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(tmpl) = self.try_load_template(&path) {
                    seen.insert(tmpl.name.clone());
                    templates.push(tmpl);
                }
            }
        }

        // Load from project dir (higher priority — overrides)
        let project_prompts_dir = self.cwd.join(".xylitol").join("prompts");
        if project_prompts_dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&project_prompts_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(tmpl) = self.try_load_template(&path) {
                    // Remove global version if name conflicts
                    templates.retain(|t| t.name != tmpl.name);
                    seen.insert(tmpl.name.clone());
                    templates.push(tmpl);
                }
            }
        }

        templates.sort_by(|a, b| a.name.cmp(&b.name));
        templates
    }

    /// Try to load a single prompt template from a .md file.
    fn try_load_template(&self, path: &Path) -> Option<PromptTemplate> {
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            return None;
        }

        let name = path.file_stem()?.to_str()?.to_string();
        let content = std::fs::read_to_string(path).ok()?;

        let (description, argument_hint, body) = parse_template_frontmatter(&content);

        Some(PromptTemplate {
            name,
            content: body,
            description,
            argument_hint,
            source_path: path.to_path_buf(),
        })
    }

    // ── Skills ──────────────────────────────────────────────────────

    /// Load skills via the existing SkillManager.
    ///
    /// This is a convenience wrapper that creates a SkillManager,
    /// loads from config, and returns the manager.
    pub fn load_skills(&self, config: Option<&AppConfig>) -> SkillManager {
        let mut manager = SkillManager::new();
        if let Some(cfg) = config {
            manager.load(cfg);
        }
        manager
    }

    // ── All resources ───────────────────────────────────────────────

    /// Load all resources and return them as a Resources struct.
    pub fn load_all(&self, config: Option<&AppConfig>) -> Resources {
        let context_files = self.load_context_files();
        let templates = self.load_prompt_templates();
        let skills = self.load_skills(config);

        Resources {
            context_files,
            templates,
            skills,
        }
    }
}

/// All loaded resources bundled together.
pub struct Resources {
    pub context_files: Vec<(String, String)>,
    pub templates: Vec<PromptTemplate>,
    pub skills: SkillManager,
}

// ── Frontmatter Parsing ─────────────────────────────────────────────

/// Parse YAML-style frontmatter from a markdown file.
///
/// Looks for `---` delimited block at the start.
/// Returns `(description, argument_hint, body)`.
fn parse_template_frontmatter(content: &str) -> (Option<String>, Option<String>, String) {
    let trimmed = content.trim_start();

    if !trimmed.starts_with("---") {
        return (None, None, content.to_string());
    }

    // Find closing ---
    let rest = &trimmed[3..];
    let Some(end_pos) = rest.find("\n---") else {
        return (None, None, content.to_string());
    };

    let fm = &rest[..end_pos];
    let body_start = end_pos + 4; // past \n---
    let body = rest[body_start..].trim_start().to_string();

    let mut description = None;
    let mut argument_hint = None;

    for line in fm.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("argument-hint:") {
            argument_hint = Some(value.trim().trim_matches('"').to_string());
        }
    }

    (description, argument_hint, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_template_frontmatter() {
        let content = "---\ndescription: \"Code review a file\"\nargument-hint: \"<file-path>\"\n---\nReview: $1";
        let (desc, hint, body) = parse_template_frontmatter(content);
        assert_eq!(desc, Some("Code review a file".into()));
        assert_eq!(hint, Some("<file-path>".into()));
        assert_eq!(body, "Review: $1");
    }

    #[test]
    fn test_parse_template_no_frontmatter() {
        let content = "Just a template body $1";
        let (desc, hint, body) = parse_template_frontmatter(content);
        assert_eq!(desc, None);
        assert_eq!(hint, None);
        assert_eq!(body, content);
    }

    #[test]
    fn test_load_context_files_finds_agents_md() {
        let tmp = TempDir::new().unwrap();
        let agents_path = tmp.path().join("AGENTS.md");
        std::fs::write(&agents_path, "# Project Rules\n\nRule 1").unwrap();

        let loader = ResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let files = loader.load_context_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "AGENTS.md");
        assert!(files[0].1.contains("Rule 1"));
    }

    #[test]
    fn test_load_context_files_from_parent() {
        let tmp = TempDir::new().unwrap();
        let subdir = tmp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let agents_path = tmp.path().join("AGENTS.md");
        std::fs::write(&agents_path, "rules").unwrap();

        let loader = ResourceLoader::new(subdir, PathBuf::from("/tmp"));
        let files = loader.load_context_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "AGENTS.md");
    }

    #[test]
    fn test_load_context_files_both() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "agents content").unwrap();
        std::fs::write(tmp.path().join("CLAUDE.md"), "claude content").unwrap();

        let loader = ResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let files = loader.load_context_files();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "AGENTS.md");
        assert_eq!(files[1].0, "CLAUDE.md");
    }

    #[test]
    fn test_load_context_files_none() {
        let tmp = TempDir::new().unwrap();
        let loader = ResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let files = loader.load_context_files();
        assert!(files.is_empty());
    }

    #[test]
    fn test_load_prompt_templates_from_project_dir() {
        let tmp = TempDir::new().unwrap();
        let prompts_dir = tmp.path().join(".xylitol").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(
            prompts_dir.join("review.md"),
            "---\ndescription: review\n---\nReview: $1",
        )
        .unwrap();

        let loader = ResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let templates = loader.load_prompt_templates();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "review");
        assert_eq!(templates[0].description, Some("review".into()));
    }

    #[test]
    fn test_project_templates_override_global() {
        let tmp = TempDir::new().unwrap();
        let global_dir = TempDir::new().unwrap();

        // Global template
        let global_prompts = global_dir.path().join("prompts");
        std::fs::create_dir_all(&global_prompts).unwrap();
        std::fs::write(
            global_prompts.join("review.md"),
            "---\ndescription: global\n---\nGlobal review",
        )
        .unwrap();

        // Project template (same name)
        let project_prompts = tmp.path().join(".xylitol").join("prompts");
        std::fs::create_dir_all(&project_prompts).unwrap();
        std::fs::write(
            project_prompts.join("review.md"),
            "---\ndescription: project\n---\nProject review",
        )
        .unwrap();

        let loader = ResourceLoader::new(tmp.path().to_path_buf(), global_dir.path().to_path_buf());
        let templates = loader.load_prompt_templates();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].description, Some("project".into()));
    }
}
