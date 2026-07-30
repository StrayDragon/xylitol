//! System prompt construction.
//!
//! Dynamically composes custom prompt, tool snippets, guidelines,
//! skills, context files, date, and CWD.
//!
//! Key functions:
//! - `build_system_prompt(opts)` — explicit options
//! - default body path uses sandboxed minijinja ([`super::sandbox`])

use crate::agent::tools::ToolSet;

use super::sandbox::render_default_base;

/// Options for building the system prompt.
#[derive(Debug, Clone, Default)]
pub struct SystemPromptOpts {
    /// User-provided custom prompt (replaces the default).
    pub custom_prompt: Option<String>,
    /// Selected tool names (for snippet/guideline inclusion).
    pub selected_tools: Vec<String>,
    /// One-line tool snippets keyed by tool name.
    pub tool_snippets: Vec<(String, String)>,
    /// Additional guideline bullets.
    pub prompt_guidelines: Vec<String>,
    /// Text appended to the end of the prompt.
    pub append_prompt: Option<String>,
    /// Current working directory.
    pub cwd: String,
    /// Project-specific context files (path => content).
    pub context_files: Vec<(String, String)>,
    /// Available skills (name + description + source info for XML rendering).
    pub skills: Vec<crate::protocol::resource::SkillInfo>,
    /// System prompt from SYSTEM.md (will be prepended to the output).
    pub system_prompt: Option<String>,
    /// Append system prompt lines from APPEND_SYSTEM.md.
    pub append_system_prompt: Vec<String>,
    /// Built-in runtime policy fragments (c1605); injected as `<runtime_policy>`.
    pub runtime_policy_fragments: Vec<String>,
    /// Optional fixed calendar date (`YYYY-MM-DD`). `None` uses `Utc::now()`.
    pub date: Option<String>,
}

/// Build a system prompt dynamically based on options.
pub fn build_system_prompt(opts: &SystemPromptOpts) -> String {
    let date = opts
        .date
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());

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
            let name = crate::agent::text::xml_escape(&skill.name);
            let desc = crate::agent::text::xml_escape(skill.description.as_deref().unwrap_or(""));
            let loc = crate::agent::text::xml_escape(&skill.source_info.path.to_string_lossy());
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

    // Runtime policy fragments (c1605) — after user APPEND / guidelines, before date.
    // Bodies are Session-deduped by fragment id; still unique-by-content here.
    if !opts.runtime_policy_fragments.is_empty() {
        prompt.push_str("\n\n<runtime_policy>\n");
        let mut seen = std::collections::HashSet::new();
        let mut first = true;
        for frag in &opts.runtime_policy_fragments {
            let t = frag.trim();
            if t.is_empty() || !seen.insert(t) {
                continue;
            }
            if !first {
                prompt.push('\n');
            }
            first = false;
            prompt.push_str(t);
            prompt.push('\n');
        }
        prompt.push_str("</runtime_policy>\n");
    }

    // Date and CWD
    prompt.push_str(&format!("\nCurrent date: {date}"));
    prompt.push_str(&format!("\nCurrent working directory: {}", opts.cwd));

    prompt
}

/// Build the default prompt base with tool snippets via sandboxed minijinja.
fn default_prompt_base(selected_tools: &[String], snippets: &[(String, String)]) -> String {
    let tools: Vec<(String, String)> = selected_tools
        .iter()
        .filter(|name| !name.starts_with("mcp:"))
        .filter_map(|name| {
            snippets
                .iter()
                .find(|(n, _)| n == name)
                .map(|(n, s)| (n.clone(), s.clone()))
        })
        .collect();
    render_default_base(&tools)
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

/// Flatten non-empty `XyTool::prompt_guidelines` for selected tools (pi Guidelines section).
pub(crate) fn collect_tool_guidelines(tool_registry: &ToolSet, selected: &[String]) -> Vec<String> {
    selected
        .iter()
        .filter_map(|name| tool_registry.get(name))
        .flat_map(|tool| {
            tool.prompt_guidelines()
                .iter()
                .map(|g| (*g).to_string())
                .collect::<Vec<_>>()
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
            date: Some("2026-07-31".into()),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("You are an expert coding assistant"));
        assert!(prompt.contains("- read: Read file contents"));
        assert!(prompt.contains("- bash: Execute bash commands"));
        assert!(prompt.contains("/tmp"));
        assert!(prompt.contains("Current date: 2026-07-31"));
    }

    #[test]
    fn injected_date_is_stable() {
        let opts = SystemPromptOpts {
            cwd: "/x".into(),
            date: Some("2099-01-02".into()),
            ..Default::default()
        };
        let a = build_system_prompt(&opts);
        let b = build_system_prompt(&opts);
        assert_eq!(a, b);
        assert!(a.contains("Current date: 2099-01-02"));
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
        use crate::protocol::resource::SkillInfo;
        use std::path::PathBuf;
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec![SkillInfo {
                name: "code-review".into(),
                description: Some("Automated code review".into()),
                source_info: crate::protocol::source_info::SourceInfo {
                    path: PathBuf::from("/home/u/.xylitol/skills/SKILL.md"),
                    source: "user".into(),
                    scope: crate::protocol::source_info::SourceScope::User,
                    origin: crate::protocol::source_info::SourceOrigin::TopLevel,
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
        use crate::protocol::resource::SkillInfo;
        use std::path::PathBuf;
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec![
                SkillInfo {
                    name: "visible".into(),
                    description: Some("ok".into()),
                    source_info: crate::protocol::source_info::SourceInfo {
                        path: PathBuf::from("/s/visible/SKILL.md"),
                        source: "user".into(),
                        scope: crate::protocol::source_info::SourceScope::User,
                        origin: crate::protocol::source_info::SourceOrigin::TopLevel,
                        base_dir: None,
                    },
                    disable_model_invocation: false,
                },
                SkillInfo {
                    name: "hidden".into(),
                    description: Some("slash only".into()),
                    source_info: crate::protocol::source_info::SourceInfo {
                        path: PathBuf::from("/s/hidden/SKILL.md"),
                        source: "user".into(),
                        scope: crate::protocol::source_info::SourceScope::User,
                        origin: crate::protocol::source_info::SourceOrigin::TopLevel,
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
        use crate::protocol::resource::SkillInfo;
        use std::path::PathBuf;
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec![SkillInfo {
                name: "a&b".into(),
                description: Some("<x>".into()),
                source_info: crate::protocol::source_info::SourceInfo {
                    path: PathBuf::from("/tmp/a&b/SKILL.md"),
                    source: "user".into(),
                    scope: crate::protocol::source_info::SourceScope::User,
                    origin: crate::protocol::source_info::SourceOrigin::TopLevel,
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

    #[test]
    fn test_custom_prompt_no_silent_tools_backfill() {
        let opts = SystemPromptOpts {
            custom_prompt: Some("Only custom body".into()),
            cwd: ".".into(),
            selected_tools: vec!["read".into()],
            tool_snippets: vec![("read".into(), "Read file".into())],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Only custom body"));
        assert!(!prompt.contains("Available tools:"));
    }

    #[test]
    fn test_runtime_policy_fragments_section() {
        let opts = SystemPromptOpts {
            append_system_prompt: vec!["USER_APPEND".into()],
            runtime_policy_fragments: vec!["POLICY_BODY".into()],
            cwd: ".".into(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        let append_i = prompt.find("USER_APPEND").expect("append");
        let policy_i = prompt.find("<runtime_policy>").expect("policy open");
        let body_i = prompt.find("POLICY_BODY").expect("policy body");
        let close_i = prompt.find("</runtime_policy>").expect("policy close");
        assert!(append_i < policy_i);
        assert!(policy_i < body_i && body_i < close_i);
        assert!(prompt.contains("Current date:"));
    }

    #[test]
    fn test_prompt_guidelines_section() {
        let opts = SystemPromptOpts {
            prompt_guidelines: vec!["Use read to examine files instead of cat or sed.".into()],
            cwd: ".".into(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Guidelines:"));
        assert!(prompt.contains("Use read to examine files instead of cat or sed."));
    }

    #[test]
    fn default_available_tools_omits_mcp_prefix_and_adds_discover() {
        let opts = SystemPromptOpts {
            selected_tools: vec![
                "read".into(),
                "mcp:fs:read".into(),
                "bash".into(),
                "mcp:git:status".into(),
            ],
            tool_snippets: vec![
                ("read".into(), "Read file".into()),
                ("bash".into(), "Run bash".into()),
                ("mcp:fs:read".into(), "MCP read".into()),
                ("mcp:git:status".into(), "MCP git".into()),
            ],
            cwd: "/tmp".into(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("- read: Read file"));
        assert!(prompt.contains("- bash: Run bash"));
        assert!(
            !prompt.contains("mcp:fs:read"),
            "Available tools MUST NOT enumerate mcp: names: {prompt}"
        );
        assert!(!prompt.contains("mcp:git:status"));
        assert!(prompt.contains("MCP/custom tools are provided in this turn's tools list"));
        assert!(prompt.contains("`/mcp`"));
    }

    #[test]
    fn custom_prompt_still_skips_default_tools_backfill() {
        let opts = SystemPromptOpts {
            custom_prompt: Some("Only custom".into()),
            selected_tools: vec!["mcp:fs:read".into(), "read".into()],
            tool_snippets: vec![
                ("read".into(), "Read".into()),
                ("mcp:fs:read".into(), "MCP".into()),
            ],
            cwd: ".".into(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Only custom"));
        assert!(!prompt.contains("Available tools:"));
        assert!(!prompt.contains("MCP/custom tools are provided"));
    }
}
