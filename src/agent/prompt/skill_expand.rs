//! Expand `$skill` references into SKILL.md bodies for the model (c1130 / A10).
//!
//! Session history and scrollback keep the raw `$name` text; only the LLM-bound
//! projection is expanded.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};
use crate::protocol::resource::SkillInfo;

/// Collect unique `$name` tokens left-to-right (`[A-Za-z0-9_-]+`).
pub fn dollar_skill_names(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
            {
                end += 1;
            }
            if end > start {
                let name = text[start..end].to_string();
                if seen.insert(name.clone()) {
                    out.push(name);
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Strip YAML frontmatter (`---` … `---`) from SKILL.md; body trimmed.
pub fn strip_skill_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content.trim().to_string();
    }
    let rest = &trimmed[3..];
    if let Some(end) = rest.find("\n---") {
        let after = rest[end + "\n---".len()..].trim_start_matches('\n');
        return after.trim().to_string();
    }
    content.trim().to_string()
}

/// Expand known `$name` refs using `skills` catalog. Unknown names left as-is.
pub fn expand_skill_refs(text: &str, skills: &[SkillInfo]) -> String {
    let by_name: HashMap<&str, &SkillInfo> = skills.iter().map(|s| (s.name.as_str(), s)).collect();
    let names = dollar_skill_names(text);
    let mut blocks = Vec::new();
    for name in names {
        let Some(skill) = by_name.get(name.as_str()) else {
            continue;
        };
        match read_skill_body(&skill.source_info.path) {
            Ok(body) if !body.is_empty() => {
                blocks.push(format!(
                    "<skill name=\"{}\">\n{}\n</skill>",
                    skill.name, body
                ));
            }
            Ok(_) => {
                log::warn!("skill expand empty body name={}", skill.name);
            }
            Err(e) => {
                log::warn!(
                    "skill expand read failed name={} path={} error={e}",
                    skill.name,
                    skill.source_info.path.display()
                );
            }
        }
    }
    if blocks.is_empty() {
        return text.to_string();
    }
    format!("{}\n\n{}", text.trim_end(), blocks.join("\n\n"))
}

fn read_skill_body(path: &Path) -> Result<String, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(strip_skill_frontmatter(&raw))
}

/// Expand `$skill` inside LLM-visible user messages (in place).
pub fn expand_skills_in_llm_messages(messages: &mut [LlmMessage], skills: &[SkillInfo]) {
    if skills.is_empty() {
        return;
    }
    for msg in messages {
        let LlmMessage::UserMessage { content, .. } = msg else {
            continue;
        };
        for part in content.iter_mut() {
            if let AgentPart::Text { text } = part {
                let expanded = expand_skill_refs(text, skills);
                if expanded != *text {
                    *text = expanded;
                }
            }
        }
    }
}

/// Expand `$skill` on a cloned history destined for the model (session stays raw).
pub fn expand_skills_in_agent_messages(messages: &mut [AgentMessage], skills: &[SkillInfo]) {
    if skills.is_empty() {
        return;
    }
    for msg in messages {
        let AgentMessage::Llm(llm) = msg else {
            continue;
        };
        let LlmMessage::UserMessage { content, .. } = llm else {
            continue;
        };
        for part in content.iter_mut() {
            if let AgentPart::Text { text } = part {
                let expanded = expand_skill_refs(text, skills);
                if expanded != *text {
                    *text = expanded;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::source_info::{SourceInfo, SourceOrigin, SourceScope};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn skill(name: &str, path: PathBuf) -> SkillInfo {
        SkillInfo {
            name: name.into(),
            description: Some("d".into()),
            source_info: SourceInfo {
                path,
                source: "test".into(),
                scope: SourceScope::User,
                origin: SourceOrigin::TopLevel,
                base_dir: None,
            },
            disable_model_invocation: false,
        }
    }

    #[test]
    fn parses_and_dedupes_dollar_names() {
        assert_eq!(
            dollar_skill_names("use $demo and $other then $demo again"),
            vec!["demo".to_string(), "other".to_string()]
        );
    }

    #[test]
    fn strip_frontmatter_keeps_body() {
        let body = strip_skill_frontmatter("---\nname: x\n---\n\n# Hello\n");
        assert_eq!(body, "# Hello");
    }

    #[test]
    fn expand_injects_known_skills_keeps_unknown() {
        let dir = TempDir::new().unwrap();
        let skill_path = dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: demo\ndescription: d\n---\n\nUNIQUE_SKILL_BODY_MARKER\n",
        )
        .unwrap();
        let skills = vec![skill("demo", skill_path)];
        let out = expand_skill_refs("Please run $demo and $nosuch", &skills);
        assert!(out.contains("Please run $demo and $nosuch"));
        assert!(out.contains("UNIQUE_SKILL_BODY_MARKER"));
        assert!(out.contains("<skill name=\"demo\">"));
        assert!(!out.contains("nosuch</skill>") && !out.contains("name=\"nosuch\""));
    }

    #[test]
    fn expand_skills_in_llm_user_message() {
        let dir = TempDir::new().unwrap();
        let skill_path = dir.path().join("SKILL.md");
        std::fs::write(&skill_path, "---\nname: demo\n---\n\nBODY_X\n").unwrap();
        let skills = vec![skill("demo", skill_path)];
        let mut msgs = vec![LlmMessage::user("hi $demo")];
        expand_skills_in_llm_messages(&mut msgs, &skills);
        let text = msgs[0].text();
        assert!(text.contains("hi $demo"));
        assert!(text.contains("BODY_X"));
    }
}
