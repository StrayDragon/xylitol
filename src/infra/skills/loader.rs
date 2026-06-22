//! SKILL.md loader — aligns with pi's skills.ts loadSkillsFromDir.
//!
//! Discovery rules:
//! - If a directory contains SKILL.md, treat as skill root, don't recurse further
//! - Otherwise, load direct .md children in the root
//! - Recurse into subdirectories to find SKILL.md
//! - Respect .gitignore, .ignore, .fdignore files

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ── Skill types ───────────────────────────────────────────────────

/// Source information for a skill.
pub use crate::infra::source_info::SourceInfo;

/// YAML frontmatter parsed from SKILL.md.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillFrontmatter {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "disable-model-invocation", default)]
    pub disable_model_invocation: bool,
    // Catch remaining fields
    #[serde(flatten)]
    #[serde(skip)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A loaded skill.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub name: String,
    pub description: String,
    pub file_path: PathBuf,
    pub base_dir: PathBuf,
    pub source_info: SourceInfo,
    pub disable_model_invocation: bool,
    /// Raw SKILL.md content (for system prompt injection).
    pub content: String,
}

/// Resource diagnostic for a loading error or warning.
#[derive(Debug, Clone)]
pub struct ResourceDiagnostic {
    pub path: String,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// Result of skill loading.
#[derive(Debug, Clone)]
pub struct LoadSkillsResult {
    pub skills: Vec<DiscoveredSkill>,
    pub diagnostics: Vec<ResourceDiagnostic>,
}

// ── Constants ─────────────────────────────────────────────────────

const SKILL_FILE: &str = "SKILL.md";
const MAX_NAME_LENGTH: usize = 64;
const MAX_DESCRIPTION_LENGTH: usize = 1024;
const IGNORE_FILE_NAMES: &[&str] = &[".gitignore", ".ignore", ".fdignore"];

// ── Validation ────────────────────────────────────────────────────

/// Validate skill name per Agent Skills spec.
fn validate_name(name: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if name.len() > MAX_NAME_LENGTH {
        errors.push(format!(
            "name exceeds {} characters ({})",
            MAX_NAME_LENGTH,
            name.len()
        ));
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

/// Validate description.
fn validate_description(description: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if description.trim().is_empty() {
        errors.push("description is required".into());
    } else if description.len() > MAX_DESCRIPTION_LENGTH {
        errors.push(format!(
            "description exceeds {} characters ({})",
            MAX_DESCRIPTION_LENGTH,
            description.len()
        ));
    }
    errors
}

// ── Frontmatter parsing ───────────────────────────────────────────

/// Parse YAML frontmatter from markdown content.
/// Returns (frontmatter, remaining_body).
fn parse_frontmatter(content: &str) -> Result<(SkillFrontmatter, String), String> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err("no frontmatter found".into());
    }

    // Find closing ---
    let after_open = &content[3..];
    let close_pos = after_open.find("\n---").ok_or("unclosed frontmatter")?;

    let yaml_str = &after_open[..close_pos];
    let body = after_open[close_pos + 4..].trim().to_string();

    let fm: SkillFrontmatter =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("invalid YAML frontmatter: {e}"))?;

    Ok((fm, body))
}

// ── Ignore support ────────────────────────────────────────────────

/// Simple gitignore-style pattern matching for skills directories.
/// Supports basic patterns: literal names, * wildcard, **, negation with !.
#[derive(Clone)]
struct IgnoreMatcher {
    patterns: Vec<IgnorePattern>,
}

#[derive(Clone)]
struct IgnorePattern {
    negated: bool,
    glob: String,
}

impl IgnoreMatcher {
    fn empty() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    fn add(&mut self, patterns: &[String]) {
        for p in patterns {
            let p = p.trim();
            if p.is_empty() || p.starts_with('#') {
                continue;
            }
            let negated = p.starts_with('!');
            let glob = if negated {
                p[1..].to_string()
            } else {
                p.to_string()
            };
            self.patterns.push(IgnorePattern { negated, glob });
        }
    }

    fn is_ignored(&self, relative_path: &str) -> bool {
        let mut ignored = false;
        for pattern in &self.patterns {
            if glob_match(&pattern.glob, relative_path) {
                ignored = !pattern.negated;
            }
        }
        ignored
    }
}

/// Simple glob matching (handles *, **, literal components).
fn glob_match(pattern: &str, name: &str) -> bool {
    let pattern = pattern.trim_start_matches('/');
    if pattern.contains("**") {
        // Simple ** handling: match anything containing the rest
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 && parts[0].is_empty() {
            // **/suffix pattern
            return name.ends_with(parts[1]);
        }
        if parts.len() == 2 && parts[1].is_empty() {
            return name.starts_with(parts[0]);
        }
    }

    if pattern.contains('*') {
        // Simple * glob
        let re_pattern = format!("^{}$", regex::escape(pattern).replace("\\*", ".*"));
        return regex::Regex::new(&re_pattern)
            .map(|re| re.is_match(name))
            .unwrap_or(false);
    }

    // Literal match
    name == pattern
        || name.starts_with(&format!("{pattern}/"))
        || name.ends_with(&format!("/{pattern}"))
        || name.contains(&format!("/{pattern}/"))
}

impl LoadSkillsResult {
    fn empty() -> Self {
        Self {
            skills: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────

/// Load skills from a directory by scanning for SKILL.md files.
pub fn load_skills_from_dir(dir: &Path, source: &str) -> LoadSkillsResult {
    if !dir.exists() {
        return LoadSkillsResult::empty();
    }
    load_skills_from_dir_internal(dir, source, true, None, dir)
}

fn load_skills_from_dir_internal(
    dir: &Path,
    source: &str,
    include_root_files: bool,
    ignore_matcher: Option<IgnoreMatcher>,
    root_dir: &Path,
) -> LoadSkillsResult {
    let mut result = LoadSkillsResult::empty();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return result,
    };

    let mut subdirs: Vec<PathBuf> = Vec::new();
    let mut has_skill_md = false;

    // Build ignore matcher for this directory
    let mut ig = ignore_matcher.clone().unwrap_or_else(IgnoreMatcher::empty);
    for ignore_name in IGNORE_FILE_NAMES {
        let ignore_path = dir.join(ignore_name);
        if ignore_path.exists()
            && let Ok(content) = fs::read_to_string(&ignore_path)
        {
            let patterns: Vec<String> = content.lines().map(String::from).collect();
            ig.add(&patterns);
        }
    }

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Check if ignored
        let rel = pathdiff::diff_paths(entry.path(), root_dir).unwrap_or_else(|| entry.path());
        let rel_str = rel.to_string_lossy().to_string();

        if ig.is_ignored(&rel_str) {
            continue;
        }

        if file_type.is_dir() {
            subdirs.push(entry.path());
        } else if include_root_files && name_str.ends_with(".md") && name_str != SKILL_FILE {
            // Direct .md file in root — load as skill
            if let Some(skill) = load_skill_file(&entry.path(), source, dir) {
                result.skills.push(skill);
            }
        }
    }

    // Check for SKILL.md — if present, this is a skill root
    let skill_md_path = dir.join(SKILL_FILE);
    if skill_md_path.exists() {
        has_skill_md = true;
        match load_skill_file(&skill_md_path, source, dir) {
            Some(skill) => result.skills.push(skill),
            None => {
                result.diagnostics.push(ResourceDiagnostic {
                    path: skill_md_path.to_string_lossy().to_string(),
                    message: "failed to load SKILL.md".into(),
                    severity: DiagnosticSeverity::Error,
                });
            }
        }
    }

    // Only recurse if this is NOT a skill root (no SKILL.md)
    if !has_skill_md {
        for subdir in &subdirs {
            let child =
                load_skills_from_dir_internal(subdir, source, true, Some(ig.clone()), root_dir);
            result.skills.extend(child.skills);
            result.diagnostics.extend(child.diagnostics);
        }
    }

    result
}

/// Load a single SKILL.md (or .md) file as a skill.
fn load_skill_file(path: &Path, source: &str, base_dir: &Path) -> Option<DiscoveredSkill> {
    let content = fs::read_to_string(path).ok()?;

    let (fm, _body) = match parse_frontmatter(&content) {
        Ok(fm) => fm,
        Err(e) => {
            // We can't push a diagnostic here without a mutable result.
            // Callers should handle missing skills gracefully.
            tracing::warn!("SKILL.md frontmatter error ({}): {e}", path.display());
            return None;
        }
    };

    let name = fm.name.unwrap_or_default();
    let description = fm.description.unwrap_or_default();

    // Validate
    let name_errors = validate_name(&name);
    let desc_errors = validate_description(&description);

    if !name_errors.is_empty() || !desc_errors.is_empty() {
        let mut msgs = name_errors;
        msgs.extend(desc_errors);
        tracing::warn!(
            "SKILL.md validation failed ({}): {}",
            path.display(),
            msgs.join("; ")
        );
        return None;
    }

    // Determine scope
    let scope = match source {
        "user" => crate::infra::source_info::SourceScope::User,
        "project" => crate::infra::source_info::SourceScope::Project,
        _ => crate::infra::source_info::SourceScope::Temporary,
    };

    Some(DiscoveredSkill {
        name,
        description,
        file_path: path.to_path_buf(),
        base_dir: base_dir.to_path_buf(),
        source_info: SourceInfo {
            path: path.to_path_buf(),
            source: source.to_string(),
            scope,
            origin: crate::infra::source_info::SourceOrigin::TopLevel,
            base_dir: Some(base_dir.to_path_buf()),
        },
        disable_model_invocation: fm.disable_model_invocation,
        content,
    })
}

// ── Load skills from multiple dirs ────────────────────────────────

// ── XML utilities ──────────────────────────────────────────────

/// Escape XML special characters in a string.
pub fn xml_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(c),
        }
    }
    result
}

/// Format a skill invocation XML block for injection into the prompt.
///
/// ```xml
/// <skill name="skill-name" location="/path/to/SKILL.md">
/// References are relative to /path/to.
///
/// <skill body>
/// </skill>
/// ```
pub fn format_skill_invocation(
    skill: &DiscoveredSkill,
    additional_instructions: Option<&str>,
) -> String {
    let base_dir = skill.base_dir.to_string_lossy();
    let escaped_name = xml_escape(&skill.name);
    let escaped_location = xml_escape(&skill.file_path.to_string_lossy());

    let mut result = format!(
        r#"<skill name="{escaped_name}" location="{escaped_location}">
References are relative to {base_dir}.

"#,
    );

    result.push_str(&skill.content);

    if let Some(extra) = additional_instructions {
        result.push('\n');
        result.push('\n');
        result.push_str(extra);
    }

    result.push_str("\n</skill>");
    result
}

/// Load skills from multiple directories.
pub fn load_skills(dirs: &[PathBuf], source: &str) -> LoadSkillsResult {
    let mut result = LoadSkillsResult::empty();
    for dir in dirs {
        let r = load_skills_from_dir(dir, source);
        result.skills.extend(r.skills);
        result.diagnostics.extend(r.diagnostics);
    }
    result
}

/// Load skills from multiple (path, source) pairs.
/// Each entry in `inputs` is a `(directory_path, source_label)` tuple.
pub fn load_sourced_skills(inputs: &[(PathBuf, &str)]) -> LoadSkillsResult {
    let mut result = LoadSkillsResult::empty();
    for (dir, source) in inputs {
        let r = load_skills_from_dir(dir, source);
        result.skills.extend(r.skills);
        result.diagnostics.extend(r.diagnostics);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(dir: &Path, filename: &str, name: &str, description: &str) {
        let content = format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n\nSkill content here.\n"
        );
        fs::write(dir.join(filename), content).expect("write skill file");
    }

    #[test]
    fn test_validate_name() {
        assert!(validate_name("python").is_empty());
        assert!(validate_name("my-skill").is_empty());
        assert!(validate_name("test-123").is_empty());

        // Invalid
        assert!(!validate_name("My-Skill").is_empty());
        assert!(!validate_name("-leading").is_empty());
        assert!(!validate_name("trailing-").is_empty());
        assert!(!validate_name("double--hyphen").is_empty());
        assert!(!validate_name(&"a".repeat(100)).is_empty());
    }

    #[test]
    fn test_validate_description() {
        assert!(validate_description("A useful skill").is_empty());
        assert!(!validate_description("").is_empty());
        assert!(!validate_description(&"x".repeat(2000)).is_empty());
    }

    #[test]
    fn test_load_skill_from_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let skills_dir = dir.path().join("skills");
        let python_dir = skills_dir.join("python");
        fs::create_dir_all(&python_dir).unwrap();

        write_skill(&python_dir, "SKILL.md", "python", "Python development");

        let result = load_skills_from_dir(&skills_dir, "project");
        assert_eq!(result.skills.len(), 1);
        assert_eq!(result.skills[0].name, "python");
        assert_eq!(result.skills[0].description, "Python development");
        assert_eq!(
            result.skills[0].source_info.scope,
            crate::infra::source_info::SourceScope::Project
        );
    }

    #[test]
    fn test_skill_dir_is_root_no_recurse() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let skills_dir = dir.path().join("skills");
        let parent_dir = skills_dir.join("parent");
        let child_dir = parent_dir.join("child");
        fs::create_dir_all(&child_dir).unwrap();

        write_skill(&parent_dir, "SKILL.md", "parent", "Parent skill");
        write_skill(&child_dir, "SKILL.md", "child", "Child skill");

        let result = load_skills_from_dir(&skills_dir, "project");
        // parent_dir has SKILL.md, so child should NOT be scanned
        assert_eq!(result.skills.len(), 1);
        assert_eq!(result.skills[0].name, "parent");
    }

    #[test]
    fn test_direct_md_in_root() {
        let dir = tempfile::tempdir().expect("create temp dir");
        // A .md file directly in the root — treated as skill
        write_skill(dir.path(), "my-skill.md", "my-skill", "A direct skill");

        let result = load_skills_from_dir(dir.path(), "user");
        assert_eq!(result.skills.len(), 1);
        assert_eq!(result.skills[0].name, "my-skill");
    }

    #[test]
    fn test_invalid_skill_excluded() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let skills_dir = dir.path().join("skills");
        let bad_dir = skills_dir.join("bad-skill");
        fs::create_dir_all(&bad_dir).unwrap();

        // Write a SKILL.md with invalid name (uppercase)
        let content = "---\nname: Bad-Name\ndescription: Does something\n---\n\n# Bad\n";
        fs::write(bad_dir.join("SKILL.md"), content).unwrap();

        let result = load_skills_from_dir(&skills_dir, "project");
        assert!(result.skills.is_empty());
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\nname: test\ndescription: A test skill\n---\n\nContent here.\n";
        let (fm, body) = parse_frontmatter(content).expect("parse frontmatter");
        assert_eq!(fm.name.unwrap(), "test");
        assert_eq!(fm.description.unwrap(), "A test skill");
        assert_eq!(body, "Content here.");
    }
}
