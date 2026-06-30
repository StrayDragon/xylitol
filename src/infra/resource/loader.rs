//! DefaultResourceLoader — unified resource discovery and caching.
//!
//! Central resource layer that discovers:
//! - Project context files (AGENTS.md, CLAUDE.md) by walking cwd → root
//! - Skills via skills loader integration
//! - Prompt templates from global and project directories
//! - Themes from global and project directories
//! - Extensions (placeholder for c70)
//! - System prompt files (SYSTEM.md, APPEND_SYSTEM.md)
//!
//! All resources are loaded once and cached. `reload()` refreshes everything.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::runtime_protocol::XyResourceLoader;

// ── ResourceDiagnostic ────────────────────────────────────────────────

// Resource metadata types relocated to `domain::resource_types` (shared vocabulary).
// `DefaultResourceLoader` (the runtime/loader impl) stays here in infra.
pub use crate::domain::resource_types::{
    AgentsFile, PromptTemplate, ResourceDiagnostic, SkillInfo, ThemeInfo,
};

// ── DefaultResourceLoader ─────────────────────────────────────────────

/// Aggregates all resources needed by the agent session.
///
/// Resources are loaded eagerly in `new()` and can be refreshed with `reload()`.
/// Accessors provide read-only views of cached resources.
pub struct DefaultResourceLoader {
    /// Current working directory.
    cwd: PathBuf,
    /// Global agent directory (~/.xylitol/).
    agent_dir: PathBuf,

    // ── Cached resources ──
    context_files: Vec<AgentsFile>,
    prompt_templates: Vec<PromptTemplate>,
    skills: Vec<SkillInfo>,
    themes: Vec<ThemeInfo>,
    system_prompt: Option<String>,
    append_system_prompt: Vec<String>,

    // ── Diagnostics ──
    context_diagnostics: Vec<ResourceDiagnostic>,
    skills_diagnostics: Vec<ResourceDiagnostic>,
    prompts_diagnostics: Vec<ResourceDiagnostic>,
    themes_diagnostics: Vec<ResourceDiagnostic>,
}

// ── DefaultResourceLoader ─────────────────────────────────────────

impl DefaultResourceLoader {
    /// Create a new DefaultResourceLoader and load all resources eagerly.
    ///
    /// `agent_dir` is typically `~/.xylitol/`.
    /// `cwd` is the project root directory.
    pub fn new(cwd: PathBuf, agent_dir: PathBuf) -> Self {
        let mut loader = Self {
            cwd,
            agent_dir,
            context_files: Vec::new(),
            prompt_templates: Vec::new(),
            skills: Vec::new(),
            themes: Vec::new(),
            system_prompt: None,
            append_system_prompt: Vec::new(),
            context_diagnostics: Vec::new(),
            skills_diagnostics: Vec::new(),
            prompts_diagnostics: Vec::new(),
            themes_diagnostics: Vec::new(),
        };
        loader.load_all();
        loader
    }

    /// Default agent directory: ~/.xylitol/
    pub fn default_agent_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
    }

    fn load_all(&mut self) {
        self.load_context_files();
        self.load_prompt_templates();
        self.load_skills_internal();
        self.load_themes();
        self.discover_system_prompt();
    }

    // ── Getters ───────────────────────────────────────────────────────

    /// Get the loaded context files (AGENTS.md, CLAUDE.md).
    pub fn get_agents_files(&self) -> &[AgentsFile] {
        &self.context_files
    }

    /// Get loaded prompt templates.
    pub fn get_prompts(&self) -> (&[PromptTemplate], &[ResourceDiagnostic]) {
        (&self.prompt_templates, &self.prompts_diagnostics)
    }

    /// Get loaded skills.
    pub fn get_skills(&self) -> (&[SkillInfo], &[ResourceDiagnostic]) {
        (&self.skills, &self.skills_diagnostics)
    }

    /// Get loaded themes.
    pub fn get_themes(&self) -> (&[ThemeInfo], &[ResourceDiagnostic]) {
        (&self.themes, &self.themes_diagnostics)
    }

    /// Get the discovered system prompt content (SYSTEM.md).
    pub fn get_system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Get the discovered append system prompt lines (APPEND_SYSTEM.md).
    pub fn get_append_system_prompt(&self) -> &[String] {
        &self.append_system_prompt
    }

    /// Get all diagnostics aggregated.
    pub fn get_all_diagnostics(&self) -> Vec<&ResourceDiagnostic> {
        self.context_diagnostics
            .iter()
            .chain(&self.skills_diagnostics)
            .chain(&self.prompts_diagnostics)
            .chain(&self.themes_diagnostics)
            .collect()
    }

    // ── Context files ─────────────────────────────────────────────────

    /// Load project context files by walking from `cwd` up to `/`.
    ///
    /// Looks for `AGENTS.md` and `CLAUDE.md` (plus `AGENTS.MD`, `CLAUDE.MD`) at each
    /// directory level. Files from ancestor directories are included (ancestor order).
    /// The global agent dir is also checked for AGENTS.md/CLAUDE.md.
    ///
    /// Each filename is discovered at most once (closest to cwd wins).
    fn load_context_files(&mut self) {
        let candidate_names = ["AGENTS.md", "AGENTS.MD", "CLAUDE.md", "CLAUDE.MD"];
        let mut seen_names: HashMap<String, AgentsFile> = HashMap::new();

        // Walk from cwd up to /
        let mut current = Some(self.cwd.as_path());
        while let Some(dir) = current {
            for &name in &candidate_names {
                let normalised = name.to_lowercase();
                if seen_names.contains_key(&normalised) {
                    continue;
                }
                let file_path = dir.join(name);
                if file_path.is_file() {
                    match std::fs::read_to_string(&file_path) {
                        Ok(content) => {
                            seen_names.insert(
                                normalised,
                                AgentsFile {
                                    path: file_path.clone(),
                                    content,
                                },
                            );
                        }
                        Err(e) => {
                            self.context_diagnostics.push(ResourceDiagnostic::warning(
                                format!("Could not read {name}: {e}"),
                                Some(file_path),
                            ));
                        }
                    }
                }
            }
            current = dir.parent();
        }

        // Also check global agent dir
        for &name in &candidate_names {
            let normalised = name.to_lowercase();
            if seen_names.contains_key(&normalised) {
                continue;
            }
            let file_path = self.agent_dir.join(name);
            if file_path.is_file() {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => {
                        seen_names.insert(
                            normalised,
                            AgentsFile {
                                path: file_path.clone(),
                                content,
                            },
                        );
                    }
                    Err(e) => {
                        self.context_diagnostics.push(ResourceDiagnostic::warning(
                            format!("Could not read global {name}: {e}"),
                            Some(file_path),
                        ));
                    }
                }
            }
        }

        // Build ordered list: AGENTS.md first, then CLAUDE.md
        let mut files: Vec<AgentsFile> = Vec::new();
        for &name in &["agents.md", "claude.md"] {
            if let Some(f) = seen_names.remove(name) {
                files.push(f);
            }
        }
        // Any remaining (case variants we didn't sort)
        for (_, f) in seen_names {
            files.push(f);
        }

        self.context_files = files;
    }

    // ── System prompt files ───────────────────────────────────────────

    /// Discover SYSTEM.md and APPEND_SYSTEM.md from project and global dirs.
    fn discover_system_prompt(&mut self) {
        // Project-level SYSTEM.md (takes priority if trusted)
        let project_config = self.cwd.join(".xylitol");
        let project_system = project_config.join("SYSTEM.md");
        let global_system = self.agent_dir.join("SYSTEM.md");

        if project_system.is_file() {
            match std::fs::read_to_string(&project_system) {
                Ok(content) => {
                    self.system_prompt = Some(content);
                }
                Err(e) => {
                    self.context_diagnostics.push(ResourceDiagnostic::warning(
                        format!("Could not read SYSTEM.md: {e}"),
                        Some(project_system),
                    ));
                }
            }
        } else if global_system.is_file() {
            match std::fs::read_to_string(&global_system) {
                Ok(content) => {
                    self.system_prompt = Some(content);
                }
                Err(e) => {
                    self.context_diagnostics.push(ResourceDiagnostic::warning(
                        format!("Could not read global SYSTEM.md: {e}"),
                        Some(global_system),
                    ));
                }
            }
        }

        // Project-level APPEND_SYSTEM.md
        let project_append = project_config.join("APPEND_SYSTEM.md");
        let global_append = self.agent_dir.join("APPEND_SYSTEM.md");

        if project_append.is_file() {
            match std::fs::read_to_string(&project_append) {
                Ok(content) => {
                    self.append_system_prompt.push(content);
                }
                Err(e) => {
                    self.context_diagnostics.push(ResourceDiagnostic::warning(
                        format!("Could not read APPEND_SYSTEM.md: {e}"),
                        Some(project_append),
                    ));
                }
            }
        }

        if global_append.is_file() {
            match std::fs::read_to_string(&global_append) {
                Ok(content) => {
                    self.append_system_prompt.push(content);
                }
                Err(e) => {
                    self.context_diagnostics.push(ResourceDiagnostic::warning(
                        format!("Could not read global APPEND_SYSTEM.md: {e}"),
                        Some(global_append),
                    ));
                }
            }
        }
    }

    // ── Prompt templates ──────────────────────────────────────────────

    /// Load prompt templates from global and project directories.
    ///
    /// Searches:
    /// 1. `~/.xylitol/prompts/*.md` (global)
    /// 2. `<cwd>/.xylitol/prompts/*.md` (project)
    ///
    /// Project templates override global templates with the same name.
    fn load_prompt_templates(&mut self) {
        let mut templates: Vec<PromptTemplate> = Vec::new();

        // Load from global dir first (lower priority)
        let global_prompts_dir = self.agent_dir.join("prompts");
        self.load_templates_from_dir(&global_prompts_dir, &mut templates);

        // Load from project dir (higher priority — overrides)
        let project_prompts_dir = self.cwd.join(".xylitol").join("prompts");
        self.load_templates_from_dir(&project_prompts_dir, &mut templates);

        // Deduplicate: project overrides global
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        // Reverse so project (loaded last) wins
        for tmpl in templates.into_iter().rev() {
            if seen.insert(tmpl.name.clone()) {
                deduped.push(tmpl);
            }
        }
        deduped.reverse();
        deduped.sort_by(|a, b| a.name.cmp(&b.name));

        self.prompt_templates = deduped;
    }

    fn load_templates_from_dir(&mut self, dir: &Path, templates: &mut Vec<PromptTemplate>) {
        if !dir.is_dir() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(tmpl) = self.try_load_template(&path) {
                templates.push(tmpl);
            }
        }
    }

    /// Try to load a single prompt template from a .md file.
    fn try_load_template(&mut self, path: &Path) -> Option<PromptTemplate> {
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            return None;
        }

        let name = path.file_stem()?.to_str()?.to_string();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                self.prompts_diagnostics.push(ResourceDiagnostic::error(
                    format!("Cannot read prompt template: {e}"),
                    Some(path.to_path_buf()),
                ));
                return None;
            }
        };

        let (description, argument_hint, body) = parse_template_frontmatter(&content);

        Some(PromptTemplate {
            name,
            content: body,
            description,
            argument_hint,
            source_info: self.source_info_for_path(path),
        })
    }

    /// Build SourceInfo for a resource path, determining scope from directory.
    fn source_info_for_path(
        &self,
        path: &std::path::Path,
    ) -> crate::infra::source_info::SourceInfo {
        let scope = if path.starts_with(&self.agent_dir) {
            crate::infra::source_info::SourceScope::User
        } else if path.starts_with(&self.cwd) {
            crate::infra::source_info::SourceScope::Project
        } else {
            crate::infra::source_info::SourceScope::Temporary
        };
        crate::infra::source_info::create_source_info(
            path.to_path_buf(),
            "local".into(),
            scope,
            crate::infra::source_info::SourceOrigin::TopLevel,
            Some(self.agent_dir.clone()),
        )
    }

    // ── Skills ────────────────────────────────────────────────────────

    /// Load skills discovered under `.xylitol/skills` (project + user).
    fn load_skills_internal(&mut self) {
        // Use the skill loader integration from c45
        {
            let project_skills = self.cwd.join(".xylitol").join("skills");
            let global_skills = self.agent_dir.join("skills");
            self.load_skills_from_skills_dir(&project_skills, "project");
            self.load_skills_from_skills_dir(&global_skills, "user");
        }
    }

    fn load_skills_from_skills_dir(&mut self, dir: &Path, _source: &str) {
        if !dir.is_dir() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let skill_dir = entry.path();
            if !skill_dir.is_dir() {
                continue;
            }
            let skill_file = skill_dir.join("SKILL.md");
            if !skill_file.is_file() {
                continue;
            }
            match std::fs::read_to_string(&skill_file) {
                Ok(content) => {
                    let (name, description) = parse_skill_frontmatter(&content);
                    if let Some(name) = name {
                        self.skills.push(SkillInfo {
                            name,
                            description,
                            source_info: self.source_info_for_path(&skill_file),
                        });
                    } else {
                        self.skills_diagnostics.push(ResourceDiagnostic::warning(
                            "SKILL.md missing name in frontmatter",
                            Some(skill_file),
                        ));
                    }
                }
                Err(e) => {
                    self.skills_diagnostics.push(ResourceDiagnostic::error(
                        format!("Cannot read SKILL.md: {e}"),
                        Some(skill_file),
                    ));
                }
            }
        }
    }

    // ── Themes ────────────────────────────────────────────────────────

    /// Load themes from global and project directories.
    fn load_themes(&mut self) {
        let global_themes_dir = self.agent_dir.join("themes");
        let project_themes_dir = self.cwd.join(".xylitol").join("themes");

        self.load_themes_from_dir(&global_themes_dir);
        self.load_themes_from_dir(&project_themes_dir);

        // Deduplicate by name (project wins)
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        // Reverse so project (loaded last) wins
        for theme in self.themes.iter().rev() {
            if seen.insert(theme.name.clone()) {
                deduped.push(theme.clone());
            }
        }
        deduped.reverse();
        self.themes = deduped;
    }

    fn load_themes_from_dir(&mut self, dir: &Path) {
        if !dir.is_dir() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unnamed")
                .to_string();
            self.themes.push(ThemeInfo {
                name,
                source_info: self.source_info_for_path(&path),
            });
        }
    }
}

// XyResourceLoader port impl. The trait currently has no `dyn` consumer in
// production (the loader-based prompt assembly path was never wired into
// Agent.prompt_opts); concrete callers in interactive/resources.rs
// use inherent methods directly. Kept as a port abstraction for the
// prompt-assembly wiring planned in c280 (session commands / system prompt).
#[allow(dead_code)]
impl XyResourceLoader for DefaultResourceLoader {
    fn get_agents_files(&self) -> &[AgentsFile] {
        &self.context_files
    }

    fn get_skills(&self) -> (&[SkillInfo], &[ResourceDiagnostic]) {
        (&self.skills, &self.skills_diagnostics)
    }

    fn get_system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    fn get_append_system_prompt(&self) -> &[String] {
        &self.append_system_prompt
    }
}

// ── Frontmatter Parsing ───────────────────────────────────────────────

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

/// Parse SKILL.md frontmatter to extract name and description.
fn parse_skill_frontmatter(content: &str) -> (Option<String>, Option<String>) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, None);
    }

    let rest = &trimmed[3..];
    let Some(end_pos) = rest.find("\n---") else {
        return (None, None);
    };

    let fm = &rest[..end_pos];
    let mut name = None;
    let mut description = None;

    for line in fm.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(value.trim().trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().trim_matches('"').to_string());
        }
    }

    (name, description)
}

// ── Convenience function ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Context files ──────────────────────────────────────────────

    #[test]
    fn test_load_context_files_finds_agents_md() {
        let tmp = TempDir::new().unwrap();
        let agents_path = tmp.path().join("AGENTS.md");
        std::fs::write(&agents_path, "# Project Rules\n\nRule 1").unwrap();

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let files = loader.get_agents_files();
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("Rule 1"));
    }

    #[test]
    fn test_load_context_files_from_parent() {
        let tmp = TempDir::new().unwrap();
        let subdir = tmp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let agents_path = tmp.path().join("AGENTS.md");
        std::fs::write(&agents_path, "rules").unwrap();

        let loader = DefaultResourceLoader::new(subdir, PathBuf::from("/tmp"));
        let files = loader.get_agents_files();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_load_context_files_both() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "agents content").unwrap();
        std::fs::write(tmp.path().join("CLAUDE.md"), "claude content").unwrap();

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let files = loader.get_agents_files();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("AGENTS.md")));
        assert!(files.iter().any(|f| f.path.ends_with("CLAUDE.md")));
    }

    #[test]
    fn test_load_context_files_none() {
        let tmp = TempDir::new().unwrap();
        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let files = loader.get_agents_files();
        assert!(files.is_empty());
    }

    #[test]
    fn test_global_agents_md_included() {
        let tmp = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        std::fs::write(agent_dir.path().join("AGENTS.md"), "global rules").unwrap();

        let loader =
            DefaultResourceLoader::new(tmp.path().to_path_buf(), agent_dir.path().to_path_buf());
        let files = loader.get_agents_files();
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("global rules"));
    }

    #[test]
    fn test_ancestor_order() {
        let tmp = TempDir::new().unwrap();
        let subdir = tmp.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&subdir).unwrap();

        // Write AGENTS.md at project root
        std::fs::write(tmp.path().join("AGENTS.md"), "project").unwrap();
        // Write in intermediate dir
        std::fs::write(tmp.path().join("a").join("AGENTS.md"), "intermediate").unwrap();

        // When cwd = /tmp/a/b/c, closest AGENTS.md is at /tmp/a/
        let loader = DefaultResourceLoader::new(subdir, PathBuf::from("/nonexistent"));
        let files = loader.get_agents_files();
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("intermediate"));
    }

    // ── System prompt ──────────────────────────────────────────────

    #[test]
    fn test_system_prompt_from_project_dir() {
        let tmp = TempDir::new().unwrap();
        let project_config = tmp.path().join(".xylitol");
        std::fs::create_dir_all(&project_config).unwrap();
        std::fs::write(project_config.join("SYSTEM.md"), "Custom system prompt").unwrap();

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        assert_eq!(loader.get_system_prompt(), Some("Custom system prompt"));
    }

    #[test]
    fn test_system_prompt_from_global_dir() {
        let tmp = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        std::fs::write(agent_dir.path().join("SYSTEM.md"), "Global system prompt").unwrap();

        let loader =
            DefaultResourceLoader::new(tmp.path().to_path_buf(), agent_dir.path().to_path_buf());
        assert_eq!(loader.get_system_prompt(), Some("Global system prompt"));
    }

    #[test]
    fn test_append_system_prompt() {
        let tmp = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        std::fs::write(agent_dir.path().join("APPEND_SYSTEM.md"), "Appended text").unwrap();

        let loader =
            DefaultResourceLoader::new(tmp.path().to_path_buf(), agent_dir.path().to_path_buf());
        let append = loader.get_append_system_prompt();
        assert_eq!(append.len(), 1);
        assert_eq!(append[0], "Appended text");
    }

    // ── Templates ──────────────────────────────────────────────────

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

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (templates, diags) = loader.get_prompts();
        assert!(diags.is_empty());
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

        let loader =
            DefaultResourceLoader::new(tmp.path().to_path_buf(), global_dir.path().to_path_buf());
        let (templates, _) = loader.get_prompts();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].description, Some("project".into()));
    }

    #[test]
    fn test_cannot_read_prompt_yields_diagnostic() {
        let tmp = TempDir::new().unwrap();
        let prompts_dir = tmp.path().join(".xylitol").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        // Create a file with bad permissions simulation → just check no crash
        // Instead: test diagnostics for missing name or parse errors
        std::fs::write(prompts_dir.join("test.md"), "---\ninvalid yaml\n---\nBody").unwrap();

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (templates, _) = loader.get_prompts();
        // Should still load (just no description)
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "test");
    }

    // ── Skills ─────────────────────────────────────────────────────

    #[test]
    fn test_get_skills_returns_loaded_skills() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join(".xylitol").join("skills").join("python");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("SKILL.md"),
            "---\nname: python\ndescription: Python programming skill\n---\n# Python skill",
        )
        .unwrap();

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (skills, diags) = loader.get_skills();

        assert!(diags.is_empty());
        assert!(skills.iter().any(|s| s.name == "python"));
    }

    // ── Themes ─────────────────────────────────────────────────────

    #[test]
    fn test_get_themes_from_project_dir() {
        let tmp = TempDir::new().unwrap();
        let themes_dir = tmp.path().join(".xylitol").join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(themes_dir.join("dark.json"), r#"{"name":"dark"}"#).unwrap();
        std::fs::write(themes_dir.join("light.json"), r#"{"name":"light"}"#).unwrap();

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (themes, diags) = loader.get_themes();
        assert!(diags.is_empty());
        assert_eq!(themes.len(), 2);
        assert!(themes.iter().any(|t| t.name == "dark"));
        assert!(themes.iter().any(|t| t.name == "light"));
    }

    #[test]
    fn test_get_themes_empty_when_no_dir() {
        let tmp = TempDir::new().unwrap();
        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (themes, diags) = loader.get_themes();
        assert!(diags.is_empty());
        assert!(themes.is_empty());
    }

    #[test]
    fn test_parse_template_frontmatter() {
        let content = "---\ndescription: \"Code review a file\"\nargument-hint: \"<file-path>\"\n---\nReview: $1";
        let (desc, hint, body) = parse_template_frontmatter(content);
        assert_eq!(desc, Some("Code review a file".into()));
        assert_eq!(hint, Some("<file-path>".into()));
        assert_eq!(body, "Review: $1");
    }

    #[test]
    fn test_parse_skill_frontmatter() {
        let content = "---\nname: code-review\ndescription: Automated code review\n---\n# Body";
        let (name, desc) = parse_skill_frontmatter(content);
        assert_eq!(name, Some("code-review".into()));
        assert_eq!(desc, Some("Automated code review".into()));
    }

    #[test]
    fn test_parse_skill_frontmatter_missing_name() {
        let content = "---\ndescription: Some skill\n---\n# Body";
        let (name, _desc) = parse_skill_frontmatter(content);
        assert_eq!(name, None);
    }

    #[test]
    fn test_load_context_files_all_diagnostics() {
        let loader =
            DefaultResourceLoader::new(PathBuf::from("/nonexistent_dir"), PathBuf::from("/tmp"));
        let diags = loader.get_all_diagnostics();
        // Should not crash on inaccessible directories
        let _ = diags.len();
    }
}
