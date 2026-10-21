use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use xylitol_plugin_sdk::resource::{AgentsFile, ResourceDiagnostic, SkillInfo, ThemeInfo};

pub struct DefaultResourceLoader {
    cwd: PathBuf,
    agent_dir: PathBuf,

    context_files: Vec<AgentsFile>,
    skills: Vec<SkillInfo>,
    themes: Vec<ThemeInfo>,
    system_prompt: Option<String>,
    append_system_prompt: Vec<String>,

    context_diagnostics: Vec<ResourceDiagnostic>,
    skills_diagnostics: Vec<ResourceDiagnostic>,
    themes_diagnostics: Vec<ResourceDiagnostic>,
}

impl DefaultResourceLoader {
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

    pub fn get_agents_files(&self) -> &[AgentsFile] {
        &self.context_files
    }

    pub fn get_skills(&self) -> (&[SkillInfo], &[ResourceDiagnostic]) {
        (&self.skills, &self.skills_diagnostics)
    }

    pub fn get_themes(&self) -> (&[ThemeInfo], &[ResourceDiagnostic]) {
        (&self.themes, &self.themes_diagnostics)
    }

    pub fn get_system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn get_append_system_prompt(&self) -> &[String] {
        &self.append_system_prompt
    }

    pub fn get_all_diagnostics(&self) -> Vec<&ResourceDiagnostic> {
        self.context_diagnostics
            .iter()
            .chain(&self.skills_diagnostics)
            .chain(&self.themes_diagnostics)
            .collect()
    }

    fn load_context_files(&mut self) {
        let candidate_names = ["AGENTS.md", "AGENTS.MD", "CLAUDE.md", "CLAUDE.MD"];
        let mut seen_names: HashMap<String, AgentsFile> = HashMap::new();

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

        let mut files: Vec<AgentsFile> = Vec::new();
        for &name in &["agents.md", "claude.md"] {
            if let Some(f) = seen_names.remove(name) {
                files.push(f);
            }
        }
        for (_, f) in seen_names {
            files.push(f);
        }

        self.context_files = files;
    }

    fn discover_system_prompt(&mut self) {
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

    fn source_info_for_path(
        &self,
        path: &std::path::Path,
    ) -> xylitol_plugin_sdk::source_info::SourceInfo {
        let under_user_agents = self
            .agent_dir
            .parent()
            .map(|p| p.join(".agents"))
            .is_some_and(|root| path.starts_with(root));
        let scope = if path.starts_with(&self.agent_dir) || under_user_agents {
            xylitol_plugin_sdk::source_info::SourceScope::User
        } else if path.starts_with(&self.cwd) {
            xylitol_plugin_sdk::source_info::SourceScope::Project
        } else {
            xylitol_plugin_sdk::source_info::SourceScope::Temporary
        };
        xylitol_plugin_sdk::source_info::create_source_info(
            path.to_path_buf(),
            "local".into(),
            scope,
            xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
            Some(self.agent_dir.clone()),
        )
    }

    fn user_agents_skills_dir(&self) -> PathBuf {
        self.agent_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".agents")
            .join("skills")
    }

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

    fn load_themes(&mut self) {
        let global_themes_dir = self.agent_dir.join("themes");
        let project_themes_dir = self.cwd.join(".xylitol").join("themes");

        self.load_themes_from_dir(&global_themes_dir);
        self.load_themes_from_dir(&project_themes_dir);

        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
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
