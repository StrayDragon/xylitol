//! System prompt construction.
//!
//! Aligns with pi's buildSystemPrompt() — dynamically composes
//! custom prompt, tool snippets, guidelines, skills, context files, date, and CWD.

use crate::agent::tools::ToolRegistry;

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
    /// Available skills.
    pub skills: Vec<String>,
}

/// Build a system prompt dynamically based on options.
pub fn build_system_prompt(opts: &SystemPromptOpts) -> String {
    let now = chrono::Utc::now();
    let date = now.format("%Y-%m-%d").to_string();

    let mut prompt = String::new();

    // Use custom prompt if provided, otherwise use default
    if let Some(ref custom) = opts.custom_prompt {
        prompt.push_str(custom);
    } else {
        prompt.push_str(&default_prompt_base(
            &opts.selected_tools,
            &opts.tool_snippets,
        ));
    }

    // Append section
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

    // Skills section
    if !opts.skills.is_empty() {
        prompt.push_str("\n<available_skills>\n");
        for skill in &opts.skills {
            prompt.push_str(&format!("  <skill>\n    {skill}\n  </skill>\n"));
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
pub fn collect_tool_snippets(
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
        let opts = SystemPromptOpts {
            cwd: ".".into(),
            skills: vec!["rust-cli-tui-developer".into()],
            ..Default::default()
        };
        let prompt = build_system_prompt(&opts);
        assert!(prompt.contains("rust-cli-tui-developer"));
    }
}
