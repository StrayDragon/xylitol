//! System prompt construction.
//!
//! Dynamically composes custom prompt, tool snippets, guidelines,
//! skills, context files, date, and CWD.
//!
//! Key functions:
//! - `build_system_prompt(opts)` — explicit options

use crate::agent::tools::ToolSet;

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
    pub(crate) skills: Vec<crate::domain::resource_types::SkillInfo>,
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

    // Skills section — XML format aligned with pi / agentskills.io
    // (`disable-model-invocation` skills omitted; intro tells model to read SKILL.md).
    let visible_skills: Vec<_> = opts
        .skills
        .iter()
        .filter(|s| !s.disable_model_invocation)
        .collect();
    if !visible_skills.is_empty() {
        prompt.push_str(
            "\n\nThe following skills provide specialized instructions for specific tasks.\n\
             Use the read tool to load a skill's file when the task matches its description.\n\
             When a skill file references a relative path, resolve it against the skill directory \
             (parent of SKILL.md / dirname of the path) and use that absolute path in tool commands.\n\n\
             <available_skills>\n",
        );
        for skill in visible_skills {
            let name = crate::domain::text::xml_escape(&skill.name);
            let desc = crate::domain::text::xml_escape(skill.description.as_deref().unwrap_or(""));
            let loc = crate::domain::text::xml_escape(&skill.source_info.path.to_string_lossy());
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

/// Collect tool snippets from a ToolSet.
pub(crate) fn collect_tool_snippets(
    tool_registry: &ToolSet,
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

#[cfg(test)]
mod tests {
    use super::*;

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
        use crate::domain::resource_types::SkillInfo;
        use std::path::PathBuf;
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec![SkillInfo {
                name: "code-review".into(),
                description: Some("Automated code review".into()),
                source_info: crate::domain::source_info::SourceInfo {
                    path: PathBuf::from("/home/u/.xylitol/skills/SKILL.md"),
                    source: "user".into(),
                    scope: crate::domain::source_info::SourceScope::User,
                    origin: crate::domain::source_info::SourceOrigin::TopLevel,
                    base_dir: Some(PathBuf::from("/home/u/.xylitol/skills")),
                },
                disable_model_invocation: false,
            }],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("code-review"));
        assert!(prompt.contains("Automated code review"));
        assert!(prompt.contains("SKILL.md"));
        assert!(prompt.contains("<available_skills>"));
        assert!(prompt.contains("</available_skills>"));
        assert!(
            prompt.contains("Use the read tool"),
            "pi-aligned intro prose required"
        );
    }

    #[test]
    fn test_skills_disable_model_invocation_omitted_from_prompt() {
        use crate::domain::resource_types::SkillInfo;
        use std::path::PathBuf;
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec![
                SkillInfo {
                    name: "visible".into(),
                    description: Some("ok".into()),
                    source_info: crate::domain::source_info::SourceInfo {
                        path: PathBuf::from("/s/visible/SKILL.md"),
                        source: "user".into(),
                        scope: crate::domain::source_info::SourceScope::User,
                        origin: crate::domain::source_info::SourceOrigin::TopLevel,
                        base_dir: None,
                    },
                    disable_model_invocation: false,
                },
                SkillInfo {
                    name: "hidden".into(),
                    description: Some("slash only".into()),
                    source_info: crate::domain::source_info::SourceInfo {
                        path: PathBuf::from("/s/hidden/SKILL.md"),
                        source: "user".into(),
                        scope: crate::domain::source_info::SourceScope::User,
                        origin: crate::domain::source_info::SourceOrigin::TopLevel,
                        base_dir: None,
                    },
                    disable_model_invocation: true,
                },
            ],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("visible"));
        assert!(!prompt.contains("hidden"));
        assert!(prompt.contains("<available_skills>"));
    }

    #[test]
    fn test_skills_xml_escapes_special_chars() {
        use crate::domain::resource_types::SkillInfo;
        use std::path::PathBuf;
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec![SkillInfo {
                name: "a&b".into(),
                description: Some("<x>".into()),
                source_info: crate::domain::source_info::SourceInfo {
                    path: PathBuf::from("/tmp/a&b/SKILL.md"),
                    source: "user".into(),
                    scope: crate::domain::source_info::SourceScope::User,
                    origin: crate::domain::source_info::SourceOrigin::TopLevel,
                    base_dir: None,
                },
                disable_model_invocation: false,
            }],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("a&amp;b"));
        assert!(prompt.contains("&lt;x&gt;"));
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
}
