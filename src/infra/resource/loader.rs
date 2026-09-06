//! DefaultResourceLoader — unified resource discovery and caching.
//!
//! Central resource layer that discovers:
//! - Project context files (AGENTS.md, CLAUDE.md) by walking cwd → root
//! - Skills via skills loader integration
//! - Themes from global and project directories
//! - System prompt files (SYSTEM.md, APPEND_SYSTEM.md)
//!
//! All resources are loaded once and cached.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use crate::protocol::resource::{AgentsFile, ResourceDiagnostic, SkillInfo, ThemeInfo};

// ── DefaultResourceLoader ─────────────────────────────────────────────

/// Aggregates all resources needed by the agent session.
///
/// Resources are loaded eagerly in `new()`.
/// Accessors provide read-only views of cached resources.
pub struct DefaultResourceLoader {
    /// Current working directory.
    cwd: PathBuf,
    /// Global agent directory (~/.xylitol/).
    agent_dir: PathBuf,

    // ── Cached resources ──
    context_files: Vec<AgentsFile>,
    skills: Vec<SkillInfo>,
    themes: Vec<ThemeInfo>,
    system_prompt: Option<String>,
    append_system_prompt: Vec<String>,

    // ── Diagnostics ──
    context_diagnostics: Vec<ResourceDiagnostic>,
    skills_diagnostics: Vec<ResourceDiagnostic>,
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
            skills: Vec::new(),
            themes: Vec::new(),
            system_prompt: None,
            append_system_prompt: Vec::new(),
            context_diagnostics: Vec::new(),
            skills_diagnostics: Vec::new(),
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
        self.load_skills_internal();
        self.load_themes();
        self.discover_system_prompt();
    }

    // ── Getters ───────────────────────────────────────────────────────

    /// Get the loaded context files (AGENTS.md, CLAUDE.md).
    pub fn get_agents_files(&self) -> &[AgentsFile] {
        &self.context_files
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

    // ── Skills ────────────────────────────────────────────────────────
    fn source_info_for_path(
        &self,
        path: &std::path::Path,
    ) -> crate::protocol::source_info::SourceInfo {
        let under_user_agents = self
            .agent_dir
            .parent()
            .map(|p| p.join(".agents"))
            .is_some_and(|root| path.starts_with(root));
        let scope = if path.starts_with(&self.agent_dir) || under_user_agents {
            crate::protocol::source_info::SourceScope::User
        } else if path.starts_with(&self.cwd) {
            crate::protocol::source_info::SourceScope::Project
        } else {
            crate::protocol::source_info::SourceScope::Temporary
        };
        crate::protocol::source_info::create_source_info(
            path.to_path_buf(),
            "local".into(),
            scope,
            crate::protocol::source_info::SourceOrigin::TopLevel,
            Some(self.agent_dir.clone()),
        )
    }

    // ── Skills ────────────────────────────────────────────────────────

    /// User-level `~/.agents/skills` (sibling of `agent_dir` / `~/.xylitol`).
    fn user_agents_skills_dir(&self) -> PathBuf {
        self.agent_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".agents")
            .join("skills")
    }

    /// Load skills from `.xylitol/skills` and `.agents/skills` (project + user).
    ///
    /// Load order (low → high precedence; name collision → later wins):
    /// `~/.agents` → `~/.xylitol` → `{cwd}/.agents` → `{cwd}/.xylitol`.
    /// So **`.xylitol` beats `.agents`**, and **project beats user**.
    fn load_skills_internal(&mut self) {
        let user_agents = self.user_agents_skills_dir();
        let user_xylitol = self.agent_dir.join("skills");
        let project_agents = self.cwd.join(".agents").join("skills");
        let project_xylitol = self.cwd.join(".xylitol").join("skills");
        self.load_skills_from_skills_dir(&user_agents);
        self.load_skills_from_skills_dir(&user_xylitol);
        self.load_skills_from_skills_dir(&project_agents);
        self.load_skills_from_skills_dir(&project_xylitol);
        self.dedup_skills_project_wins();
    }

    /// Keep first occurrence when iterating reverse (project loaded last → wins).
    fn dedup_skills_project_wins(&mut self) {
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        let mut collisions = Vec::new();
        for skill in self.skills.iter().rev() {
            if seen.insert(skill.name.clone()) {
                deduped.push(skill.clone());
            } else {
                collisions.push(skill.name.clone());
            }
        }
        deduped.reverse();
        for name in collisions {
            self.skills_diagnostics.push(ResourceDiagnostic::warning(
                format!("duplicate skill name '{name}'; keeping higher-precedence entry"),
                None,
            ));
        }
        self.skills = deduped;
    }

    fn load_skills_from_skills_dir(&mut self, dir: &Path) {
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
            let fallback_name = skill_dir
                .file_name()
                .and_then(|s| s.to_str())
                .map(str::to_string);
            match std::fs::read_to_string(&skill_file) {
                Ok(content) => {
                    let parsed = parse_skill_frontmatter(&content);
                    let name = parsed.name.or(fallback_name);
                    let Some(name) = name else {
                        self.skills_diagnostics.push(ResourceDiagnostic::warning(
                            "SKILL.md missing name in frontmatter and directory name",
                            Some(skill_file),
                        ));
                        continue;
                    };
                    for err in validate_skill_name(&name) {
                        self.skills_diagnostics.push(ResourceDiagnostic::warning(
                            format!("skill '{name}': {err}"),
                            Some(skill_file.clone()),
                        ));
                    }
                    if parsed
                        .description
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .is_none()
                    {
                        self.skills_diagnostics.push(ResourceDiagnostic::warning(
                            format!("skill '{name}': missing description"),
                            Some(skill_file.clone()),
                        ));
                    }
                    self.skills.push(SkillInfo {
                        name,
                        description: parsed.description,
                        source_info: self.source_info_for_path(&skill_file),
                        disable_model_invocation: parsed.disable_model_invocation,
                    });
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

// ── Frontmatter Parsing ───────────────────────────────────────────────

/// Parse SKILL.md frontmatter to extract name and description.
/// Parsed SKILL.md frontmatter (pi-aligned subset).
struct ParsedSkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    disable_model_invocation: bool,
}

fn parse_skill_frontmatter(content: &str) -> ParsedSkillFrontmatter {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return ParsedSkillFrontmatter {
            name: None,
            description: None,
            disable_model_invocation: false,
        };
    }

    let rest = &trimmed[3..];
    let Some(end_pos) = rest.find("\n---") else {
        return ParsedSkillFrontmatter {
            name: None,
            description: None,
            disable_model_invocation: false,
        };
    };

    let fm = &rest[..end_pos];
    let mut name = None;
    let mut description = None;
    let mut disable_model_invocation = false;

    for line in fm.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(value.trim().trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("disable-model-invocation:") {
            let v = value.trim().trim_matches('"').to_ascii_lowercase();
            disable_model_invocation = matches!(v.as_str(), "true" | "yes" | "1");
        }
    }

    ParsedSkillFrontmatter {
        name,
        description,
        disable_model_invocation,
    }
}

/// Agent Skills name rules (pi / agentskills.io subset) — warnings only.
fn validate_skill_name(name: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if name.len() > 64 {
        errors.push(format!("name exceeds 64 characters ({})", name.len()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        errors.push(
            "name contains invalid characters (must be lowercase a-z, 0-9, hyphens only)".into(),
        );
    }
    if name.starts_with('-') || name.ends_with('-') {
        errors.push("name must not start or end with a hyphen".into());
    }
    if name.contains("--") {
        errors.push("name must not contain consecutive hyphens".into());
    }
    errors
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

    #[test]
    fn test_skills_project_overrides_user_on_name_collision() {
        let project = TempDir::new().unwrap();
        let agent = TempDir::new().unwrap();
        let proj_skill = project
            .path()
            .join(".xylitol")
            .join("skills")
            .join("shared");
        let user_skill = agent.path().join("skills").join("shared");
        std::fs::create_dir_all(&proj_skill).unwrap();
        std::fs::create_dir_all(&user_skill).unwrap();
        std::fs::write(
            proj_skill.join("SKILL.md"),
            "---\nname: shared\ndescription: from-project\n---\n",
        )
        .unwrap();
        std::fs::write(
            user_skill.join("SKILL.md"),
            "---\nname: shared\ndescription: from-user\n---\n",
        )
        .unwrap();

        let loader =
            DefaultResourceLoader::new(project.path().to_path_buf(), agent.path().to_path_buf());
        let (skills, diags) = loader.get_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "shared");
        assert_eq!(skills[0].description.as_deref(), Some("from-project"));
        assert!(
            diags.iter().any(|d| d.message.contains("duplicate")),
            "collision should warn; got {diags:?}"
        );
    }

    #[test]
    fn test_skills_loads_from_project_agents_dir() {
        let project = TempDir::new().unwrap();
        let skill = project
            .path()
            .join(".agents")
            .join("skills")
            .join("from-agents");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(
            skill.join("SKILL.md"),
            "---\nname: from-agents\ndescription: agents layout\n---\nbody\n",
        )
        .unwrap();
        let loader =
            DefaultResourceLoader::new(project.path().to_path_buf(), PathBuf::from("/tmp/xylitol"));
        let (skills, _) = loader.get_skills();
        assert!(
            skills.iter().any(|s| s.name == "from-agents"),
            "expected .agents/skills discovery; got {skills:?}"
        );
        let s = skills.iter().find(|s| s.name == "from-agents").unwrap();
        assert_eq!(
            s.source_info.scope,
            crate::protocol::source_info::SourceScope::Project
        );
    }

    #[test]
    fn test_skills_xylitol_overrides_agents_on_name_collision() {
        let project = TempDir::new().unwrap();
        let agents_skill = project.path().join(".agents").join("skills").join("shared");
        let xylitol_skill = project
            .path()
            .join(".xylitol")
            .join("skills")
            .join("shared");
        std::fs::create_dir_all(&agents_skill).unwrap();
        std::fs::create_dir_all(&xylitol_skill).unwrap();
        std::fs::write(
            agents_skill.join("SKILL.md"),
            "---\nname: shared\ndescription: from-agents\n---\n",
        )
        .unwrap();
        std::fs::write(
            xylitol_skill.join("SKILL.md"),
            "---\nname: shared\ndescription: from-xylitol\n---\n",
        )
        .unwrap();
        let loader =
            DefaultResourceLoader::new(project.path().to_path_buf(), PathBuf::from("/tmp/xylitol"));
        let (skills, diags) = loader.get_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].description.as_deref(), Some("from-xylitol"));
        assert!(
            diags.iter().any(|d| d.message.contains("duplicate")),
            "collision should warn; got {diags:?}"
        );
    }

    #[test]
    fn test_skills_loads_from_user_agents_dir() {
        let home = TempDir::new().unwrap();
        let agent_dir = home.path().join(".xylitol");
        let user_agents = home.path().join(".agents").join("skills").join("ua");
        std::fs::create_dir_all(&agent_dir).unwrap();
        std::fs::create_dir_all(&user_agents).unwrap();
        std::fs::write(
            user_agents.join("SKILL.md"),
            "---\nname: ua\ndescription: user agents\n---\n",
        )
        .unwrap();
        let loader =
            DefaultResourceLoader::new(TempDir::new().unwrap().path().to_path_buf(), agent_dir);
        let (skills, _) = loader.get_skills();
        let s = skills
            .iter()
            .find(|s| s.name == "ua")
            .expect("user .agents skill");
        assert_eq!(
            s.source_info.scope,
            crate::protocol::source_info::SourceScope::User
        );
    }

    #[test]
    fn test_skills_disable_model_invocation_flag_parsed() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join(".xylitol").join("skills").join("quiet");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(
            skill.join("SKILL.md"),
            "---\nname: quiet\ndescription: hush\ndisable-model-invocation: true\n---\n",
        )
        .unwrap();
        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (skills, _) = loader.get_skills();
        let s = skills.iter().find(|s| s.name == "quiet").unwrap();
        assert!(s.disable_model_invocation);
    }

    #[test]
    fn test_skills_name_falls_back_to_directory() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp
            .path()
            .join(".xylitol")
            .join("skills")
            .join("fallback-dir");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(
            skill.join("SKILL.md"),
            "---\ndescription: no name key\n---\nbody\n",
        )
        .unwrap();
        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (skills, _) = loader.get_skills();
        assert!(
            skills.iter().any(|s| s.name == "fallback-dir"),
            "expected dir-name fallback; got {skills:?}"
        );
    }

    #[test]
    fn test_skills_missing_description_warns() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join(".xylitol").join("skills").join("nodesc");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), "---\nname: nodesc\n---\n").unwrap();
        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let (_, diags) = loader.get_skills();
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("missing description")),
            "expected description warning; got {diags:?}"
        );
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
    fn test_parse_skill_frontmatter() {
        let content = "---\nname: code-review\ndescription: Automated code review\n---\n# Body";
        let parsed = parse_skill_frontmatter(content);
        assert_eq!(parsed.name, Some("code-review".into()));
        assert_eq!(parsed.description, Some("Automated code review".into()));
        assert!(!parsed.disable_model_invocation);
    }

    #[test]
    fn test_parse_skill_frontmatter_missing_name() {
        let content = "---\ndescription: Some skill\n---\n# Body";
        let parsed = parse_skill_frontmatter(content);
        assert_eq!(parsed.name, None);
    }

    #[test]
    fn test_parse_skill_frontmatter_disable_model_invocation() {
        let content =
            "---\nname: quiet\ndescription: hush\ndisable-model-invocation: true\n---\n# Body";
        let parsed = parse_skill_frontmatter(content);
        assert!(parsed.disable_model_invocation);
    }

    #[test]
    fn test_load_context_files_all_diagnostics() {
        // Inaccessible cwd (and its missing AGENTS.md/CLAUDE.md) must yield no
        // files and no diagnostics — the walk tolerates unreadable directories.
        let loader =
            DefaultResourceLoader::new(PathBuf::from("/nonexistent_dir"), PathBuf::from("/tmp"));
        let diags = loader.get_all_diagnostics();
        assert!(
            loader.get_agents_files().is_empty(),
            "missing dirs must yield no context files"
        );
        assert!(
            diags.is_empty(),
            "missing dirs must not produce diagnostics: {diags:?}"
        );
    }
}
