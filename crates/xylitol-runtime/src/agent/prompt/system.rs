use crate::agent::tools::ToolSet;

use super::sandbox::render_default_base;

#[derive(Debug, Clone, Default)]
pub struct SystemPromptOpts {
    pub custom_prompt: Option<String>,
    pub selected_tools: Vec<String>,
    pub tool_snippets: Vec<(String, String)>,
    pub prompt_guidelines: Vec<String>,
    pub append_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>,
    pub system_prompt: Option<String>,
    pub append_system_prompt: Vec<String>,
    pub runtime_policy_fragments: Vec<String>,
}

pub fn build_system_prompt(opts: &SystemPromptOpts) -> String {
    let mut prompt = String::new();

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

    if let Some(ref append) = opts.append_prompt {
        prompt.push_str("\n\n");
        prompt.push_str(append);
    }

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

    if !opts.append_system_prompt.is_empty() {
        for append in &opts.append_system_prompt {
            prompt.push_str("\n\n");
            prompt.push_str(append);
        }
    }

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
            let name = crate::utils::xml_escape(&skill.name);
            let desc = crate::utils::xml_escape(skill.description.as_deref().unwrap_or(""));
            let loc = crate::utils::xml_escape(&skill.source_info.path.to_string_lossy());
            prompt.push_str(&format!(
                "  <skill>\n    <name>{name}</name>\n    <description>{desc}</description>\n    <location>{loc}</location>\n  </skill>\n"
            ));
        }
        prompt.push_str("</available_skills>\n");
    }

    if !opts.prompt_guidelines.is_empty() {
        prompt.push_str("\nGuidelines:\n");
        for g in &opts.prompt_guidelines {
            prompt.push_str(&format!("- {g}\n"));
        }
    }

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

    prompt
}

fn default_prompt_base(selected_tools: &[String], snippets: &[(String, String)]) -> String {
    let tools: Vec<(String, String)> = selected_tools
        .iter()
        .filter(|name| !xylitol_plugin_sdk::is_mcp_tool_name(name))
        .filter_map(|name| {
            snippets
                .iter()
                .find(|(n, _)| n == name)
                .map(|(n, s)| (n.clone(), s.clone()))
        })
        .collect();
    render_default_base(&tools)
}

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
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("You are an expert coding assistant"));
        assert!(prompt.contains("- read: Read file contents"));
        assert!(prompt.contains("- bash: Execute bash commands"));
        assert!(
            !prompt.contains("Current working directory:"),
            "cwd must not enter system: {prompt}"
        );
    }

    #[test]
    fn default_placement_is_omit() {
        let opts = SystemPromptOpts {
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(!prompt.contains("Current date:"));
        assert!(!prompt.contains("Current working directory:"));
    }

    #[test]
    fn test_custom_prompt() {
        let opts = SystemPromptOpts {
            custom_prompt: Some("Custom instructions here".into()),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Custom instructions here"));
    }

    #[test]
    fn test_context_files() {
        let opts = SystemPromptOpts {
            context_files: vec![("AGENTS.md".into(), "Project rules".into())],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("AGENTS.md"));
        assert!(prompt.contains("Project rules"));
    }

    #[test]
    fn test_skills_section() {
        use std::path::PathBuf;
        use xylitol_plugin_sdk::resource::SkillInfo;
        let opts = SystemPromptOpts {
            skills: vec![SkillInfo {
                name: "code-review".into(),
                description: Some("Automated code review".into()),
                source_info: xylitol_plugin_sdk::source_info::SourceInfo {
                    path: PathBuf::from("/home/u/.xylitol/skills/SKILL.md"),
                    source: "user".into(),
                    scope: xylitol_plugin_sdk::source_info::SourceScope::User,
                    origin: xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
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
        use std::path::PathBuf;
        use xylitol_plugin_sdk::resource::SkillInfo;
        let opts = SystemPromptOpts {
            skills: vec![
                SkillInfo {
                    name: "visible".into(),
                    description: Some("ok".into()),
                    source_info: xylitol_plugin_sdk::source_info::SourceInfo {
                        path: PathBuf::from("/s/visible/SKILL.md"),
                        source: "user".into(),
                        scope: xylitol_plugin_sdk::source_info::SourceScope::User,
                        origin: xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
                        base_dir: None,
                    },
                    disable_model_invocation: false,
                },
                SkillInfo {
                    name: "hidden".into(),
                    description: Some("slash only".into()),
                    source_info: xylitol_plugin_sdk::source_info::SourceInfo {
                        path: PathBuf::from("/s/hidden/SKILL.md"),
                        source: "user".into(),
                        scope: xylitol_plugin_sdk::source_info::SourceScope::User,
                        origin: xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
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
        use std::path::PathBuf;
        use xylitol_plugin_sdk::resource::SkillInfo;
        let opts = SystemPromptOpts {
            skills: vec![SkillInfo {
                name: "a&b".into(),
                description: Some("<x>".into()),
                source_info: xylitol_plugin_sdk::source_info::SourceInfo {
                    path: PathBuf::from("/tmp/a&b/SKILL.md"),
                    source: "user".into(),
                    scope: xylitol_plugin_sdk::source_info::SourceScope::User,
                    origin: xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
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
            system_prompt: Some("Custom SYSTEM.md content".into()),
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Custom SYSTEM.md content"));
        assert!(!prompt.contains("expert coding assistant"));
    }

    #[test]
    fn test_append_system_prompt_field() {
        let opts = SystemPromptOpts {
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
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        let append_i = prompt.find("USER_APPEND").expect("append");
        let policy_i = prompt.find("<runtime_policy>").expect("policy open");
        let body_i = prompt.find("POLICY_BODY").expect("policy body");
        let close_i = prompt.find("</runtime_policy>").expect("policy close");
        assert!(append_i < policy_i);
        assert!(policy_i < body_i && body_i < close_i);
        assert!(!prompt.contains("Current date:"));
    }

    #[test]
    fn test_prompt_guidelines_section() {
        let opts = SystemPromptOpts {
            prompt_guidelines: vec!["Use read to examine files instead of cat or sed.".into()],
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
                "mcp__fs__read".into(),
                "bash".into(),
                "mcp__git__status".into(),
            ],
            tool_snippets: vec![
                ("read".into(), "Read file".into()),
                ("bash".into(), "Run bash".into()),
                ("mcp__fs__read".into(), "MCP read".into()),
                ("mcp__git__status".into(), "MCP git".into()),
            ],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("- read: Read file"));
        assert!(prompt.contains("- bash: Run bash"));
        assert!(
            !prompt.contains("mcp__fs__read"),
            "Available tools MUST NOT enumerate mcp_ names: {prompt}"
        );
        assert!(!prompt.contains("mcp__git__status"));
        assert!(prompt.contains("MCP/custom tools are provided in this turn's tools list"));
        assert!(prompt.contains("`/mcp`"));
    }

    #[test]
    fn custom_prompt_still_skips_default_tools_backfill() {
        let opts = SystemPromptOpts {
            custom_prompt: Some("Only custom".into()),
            selected_tools: vec!["mcp__fs__read".into(), "read".into()],
            tool_snippets: vec![
                ("read".into(), "Read".into()),
                ("mcp__fs__read".into(), "MCP".into()),
            ],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("Only custom"));
        assert!(!prompt.contains("Available tools:"));
        assert!(!prompt.contains("MCP/custom tools are provided"));
    }
}
