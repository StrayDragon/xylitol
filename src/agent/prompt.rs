//! System prompt construction.
//!
//! Dynamically composes custom prompt, tool snippets, guidelines,
//! skills, context files, date, and CWD.
//!
//! Key functions:
//! - `build_system_prompt(opts)` — explicit options
//! - `build_system_prompt_from_loader(loader, tools_opts)` — integrates with ResourceLoader

#![allow(dead_code)]

use crate::agent::tools::ToolRegistry;
use crate::infra::resource::DefaultResourceLoader;

/// Options for building the system prompt.
#[derive(Debug, Clone, Default)]
pub(crate) struct SystemPromptOpts {
    /// User-provided custom prompt (replaces the default).
    pub(crate) custom_prompt: Option<String>,
    /// Selected tool names (for snippet/guideline inclusion).
    pub(crate) selected_tools: Vec<String>,
    /// One-line tool snippets keyed by tool name.
    pub(crate) tool_snippets: Vec<(String, String)>,
    /// Additional guideline bullets.
    pub(crate) prompt_guidelines: Vec<String>,
    /// Text appended to the end of the prompt.
    pub(crate) append_prompt: Option<String>,
    /// Current working directory.
    pub(crate) cwd: String,
    /// Project-specific context files (path => content).
    pub(crate) context_files: Vec<(String, String)>,
    /// Available skills (name + description + source info for XML rendering).
    pub(crate) skills: Vec<crate::infra::resource::SkillInfo>,
    /// System prompt from SYSTEM.md (will be prepended to the output).
    pub(crate) system_prompt: Option<String>,
    /// Append system prompt lines from APPEND_SYSTEM.md.
    pub(crate) append_system_prompt: Vec<String>,
}

/// Build a system prompt dynamically based on options.
pub(crate) fn build_system_prompt(opts: &SystemPromptOpts) -> String {
    let now = chrono::Utc::now();
    let date = now.format("%Y-%m-%d").to_string();

    let mut prompt = String::new();

    // Use system prompt (SYSTEM.md) if available, then custom_prompt, then default
    if let Some(ref sp) = opts.system_prompt {
        prompt.push_str(sp);
    } else if let Some(ref custom) = opts.custom_prompt {
        prompt.push_str(custom);
    } else {
        prompt.push_str(&default_prompt_base(
            &opts.selected_tools,
            &opts.tool_snippets,
        ));
    }

    // Append section (explicit append_prompt)
    if let Some(ref append) = opts.append_prompt {
        prompt.push_str("\n\n");
        prompt.push_str(append);
    }

    // Project context files
    if !opts.context_files.is_empty() {
        prompt.push_str("\n\n<project_context>\n\n");
        prompt.push_str("Project-specific instructions and guidelines:\n\n");
        for (path, content) in &opts.context_files {
            prompt.push_str(&format!(
                "<project_instructions path=\"{path}\">\n{content}\n</project_instructions>\n\n"
            ));
        }
        prompt.push_str("</project_context>\n");
    }

    // Append system prompt (APPEND_SYSTEM.md from loader)
    if !opts.append_system_prompt.is_empty() {
        for append in &opts.append_system_prompt {
            prompt.push_str("\n\n");
            prompt.push_str(append);
        }
    }

    // Skills section — XML format with name, description, location
    if !opts.skills.is_empty() {
        prompt.push_str("\n<available_skills>\n");
        for skill in &opts.skills {
            let name = crate::infra::skills::loader::xml_escape(&skill.name);
            let desc = crate::infra::skills::loader::xml_escape(
                skill.description.as_deref().unwrap_or(""),
            );
            let loc =
                crate::infra::skills::loader::xml_escape(&skill.source_info.path.to_string_lossy());
            prompt.push_str(&format!(
                "  <skill>\n    <name>{name}</name>\n    <description>{desc}</description>\n    <location>{loc}</location>\n  </skill>\n"
            ));
        }
        prompt.push_str("</available_skills>\n");
    }

    // Guidelines
    if !opts.prompt_guidelines.is_empty() {
        prompt.push_str("\nGuidelines:\n");
        for g in &opts.prompt_guidelines {
            prompt.push_str(&format!("- {g}\n"));
        }
    }

    // Date and CWD
    prompt.push_str(&format!("\nCurrent date: {date}"));
    prompt.push_str(&format!("\nCurrent working directory: {}", opts.cwd));

    prompt
}

/// Build the default prompt base with tool snippets.
fn default_prompt_base(selected_tools: &[String], snippets: &[(String, String)]) -> String {
    let tool_lines: Vec<String> = selected_tools
        .iter()
        .filter_map(|name| {
            snippets
                .iter()
                .find(|(n, _)| n == name)
                .map(|(n, s)| format!("- {n}: {s}"))
        })
        .collect();

    let tools_section = if tool_lines.is_empty() {
        String::from("(none)")
    } else {
        tool_lines.join("\n")
    };

    format!(
        "You are an expert coding assistant.\n\n\
         Available tools:\n\
         {tools_section}\n\n\
         In addition to the tools above, you may have access to other custom tools depending on the project."
    )
}

/// Collect tool snippets from a ToolRegistry.
pub(crate) fn collect_tool_snippets(
    tool_registry: &ToolRegistry,
    selected: &[String],
) -> Vec<(String, String)> {
    selected
        .iter()
        .filter_map(|name| {
            tool_registry.get(name).map(|tool| {
                let snippet = tool.prompt_snippet().unwrap_or(tool.description());
                (name.clone(), snippet.to_string())
            })
        })
        .collect()
}

/// Options for the tools/skills part of prompt construction.
#[derive(Debug, Clone, Default)]
pub(crate) struct PromptToolsOpts {
    pub(crate) custom_prompt: Option<String>,
    pub(crate) selected_tools: Vec<String>,
    pub(crate) tool_snippets: Vec<(String, String)>,
    pub(crate) prompt_guidelines: Vec<String>,
    pub(crate) append_prompt: Option<String>,
    pub(crate) cwd: String,
}

/// Build a system prompt from a ResourceLoader and tools options.
///
/// Assembles:
/// 1. System prompt (SYSTEM.md) from the loader
/// 2. AGENTS.md/CLAUDE.md context files
/// 3. APPEND_SYSTEM.md
/// 4. Active skills
/// 5. Tools and guidelines
pub(crate) fn build_system_prompt_from_loader(
    loader: &DefaultResourceLoader,
    tools_opts: &PromptToolsOpts,
) -> String {
    let opts = SystemPromptOpts {
        custom_prompt: tools_opts.custom_prompt.clone(),
        selected_tools: tools_opts.selected_tools.clone(),
        tool_snippets: tools_opts.tool_snippets.clone(),
        prompt_guidelines: tools_opts.prompt_guidelines.clone(),
        append_prompt: tools_opts.append_prompt.clone(),
        cwd: tools_opts.cwd.clone(),
        context_files: loader
            .get_agents_files()
            .iter()
            .map(|f| (f.path.to_string_lossy().to_string(), f.content.clone()))
            .collect(),
        skills: loader.get_skills().0.to_vec(),
        system_prompt: loader.get_system_prompt().map(String::from),
        append_system_prompt: loader.get_append_system_prompt().to_vec(),
    };
    build_system_prompt(&opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_basic_prompt() {
        let opts = SystemPromptOpts {
            selected_tools: vec!["read".into(), "bash".into()],
            tool_snippets: vec![
                ("read".into(), "Read file contents".into()),
                ("bash".into(), "Execute bash commands".into()),
            ],
            cwd: "/tmp".into(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("You are an expert coding assistant"));
        assert!(prompt.contains("- read: Read file contents"));
        assert!(prompt.contains("- bash: Execute bash commands"));
        assert!(prompt.contains("/tmp"));
    }

    #[test]
    fn test_custom_prompt() {
        let opts = SystemPromptOpts {
            custom_prompt: Some("Custom instructions here".into()),
            cwd: ".".into(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Custom instructions here"));
    }

    #[test]
    fn test_context_files() {
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            context_files: vec![("AGENTS.md".into(), "Project rules".into())],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("AGENTS.md"));
        assert!(prompt.contains("Project rules"));
    }

    #[test]
    fn test_skills_section() {
        use crate::infra::resource::SkillInfo;
        use std::path::PathBuf;
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec![SkillInfo {
                name: "rust-cli-tui-developer".into(),
                description: Some("Build Rust CLI tools".into()),
                source_info: crate::infra::source_info::SourceInfo {
                    path: PathBuf::from("/home/u/.xylitol/skills/SKILL.md"),
                    source: "user".into(),
                    scope: crate::infra::source_info::SourceScope::User,
                    origin: crate::infra::source_info::SourceOrigin::TopLevel,
                    base_dir: Some(PathBuf::from("/home/u/.xylitol/skills")),
                },
            }],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("rust-cli-tui-developer"));
        assert!(prompt.contains("Build Rust CLI tools"));
        assert!(prompt.contains("SKILL.md"));
        assert!(prompt.contains("<available_skills>"));
        assert!(prompt.contains("</available_skills>"));
    }

    #[test]
    fn test_system_prompt_field() {
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            system_prompt: Some("Custom SYSTEM.md content".into()),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Custom SYSTEM.md content"));
        // system_prompt takes priority over custom_prompt
        assert!(!prompt.contains("expert coding assistant"));
    }

    #[test]
    fn test_append_system_prompt_field() {
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            append_system_prompt: vec!["Extra safety rules".into()],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Extra safety rules"));
    }

    #[test]
    fn test_build_from_loader() {
        use crate::infra::resource::DefaultResourceLoader;
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "# Project rules").unwrap();

        let loader = DefaultResourceLoader::new(tmp.path().to_path_buf(), PathBuf::from("/tmp"));
        let tools_opts = PromptToolsOpts {
            selected_tools: vec!["read".into()],
            tool_snippets: vec![("read".into(), "Read files".into())],
            cwd: tmp.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt_from_loader(&loader, &tools_opts);
        // Should include project rules from AGENTS.md
        assert!(prompt.contains("Project rules"));
        // Should include tools
        assert!(prompt.contains("read: Read files"));
    }
}
