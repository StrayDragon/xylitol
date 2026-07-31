//! Read-only resource diagnostics — `resources list|info|doctor`.
//!
//! Aligns with c120-add-resource-diagnostics. These commands are strictly
//! read-only: they reuse `DefaultResourceLoader`'s cached discovery and never
//! create, modify, or delete files.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Subcommand;

use crate::infra::resource::{DefaultResourceLoader, ResourceDiagnostic, SkillInfo, ThemeInfo};

/// Sub-actions for `xylitol resources`.
#[derive(Subcommand, Debug)]
pub enum ResourcesAction {
    /// List all discovered resources (skills / themes).
    List,
    /// Show details for a single named resource.
    Info {
        /// Resource name (skill / theme name).
        name: String,
    },
    /// Run diagnostics; exit non-zero when issues exist.
    Doctor,
}

/// Run a `resources` sub-action using the default cwd and agent dir.
pub fn run(action: ResourcesAction) -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let agent_dir = DefaultResourceLoader::default_agent_dir();
    let (code, output) = run_with_dirs(action, &cwd, &agent_dir);
    print!("{output}");
    code
}

/// Run a `resources` sub-action against explicit directories.
///
/// Returns `(exit_code, output_text)`. Kept separate from [`run`] so tests can
/// inject temp directories and assert on output without touching real stdout.
pub fn run_with_dirs(action: ResourcesAction, cwd: &Path, agent_dir: &Path) -> (ExitCode, String) {
    let loader = DefaultResourceLoader::new(cwd.to_path_buf(), agent_dir.to_path_buf());

    match action {
        ResourcesAction::List => list(&loader, cwd, agent_dir),
        ResourcesAction::Info { name } => info(&loader, &name, cwd, agent_dir),
        ResourcesAction::Doctor => doctor(&loader),
    }
}

fn list(loader: &DefaultResourceLoader, cwd: &Path, agent_dir: &Path) -> (ExitCode, String) {
    let (skills, _) = loader.get_skills();
    let (themes, _) = loader.get_themes();

    let mut out = String::new();
    out.push_str("skills:\n");
    if skills.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for s in skills {
            out.push_str(&format!(
                "  {} [{}] {}\n",
                s.name,
                scope_of(&s.source_info.path, agent_dir, cwd),
                s.source_info.path.display()
            ));
            if let Some(desc) = &s.description {
                out.push_str(&format!("      {desc}\n"));
            }
        }
    }

    out.push_str("themes:\n");
    if themes.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for t in themes {
            out.push_str(&format!(
                "  {} [{}] {}\n",
                t.name,
                scope_of(&t.source_info.path, agent_dir, cwd),
                t.source_info.path.display()
            ));
        }
    }

    (ExitCode::SUCCESS, out)
}

fn info(
    loader: &DefaultResourceLoader,
    name: &str,
    cwd: &Path,
    agent_dir: &Path,
) -> (ExitCode, String) {
    let (skills, _) = loader.get_skills();
    let (themes, _) = loader.get_themes();

    if let Some(s) = skills.iter().find(|s| s.name == name) {
        return (ExitCode::SUCCESS, format_skill(s, agent_dir, cwd));
    }
    if let Some(t) = themes.iter().find(|t| t.name == name) {
        return (ExitCode::SUCCESS, format_theme(t, agent_dir, cwd));
    }

    (ExitCode::FAILURE, format!("resource not found: {name}\n"))
}

fn doctor(loader: &DefaultResourceLoader) -> (ExitCode, String) {
    let diags: Vec<&ResourceDiagnostic> = loader.get_all_diagnostics();
    if diags.is_empty() {
        return (ExitCode::SUCCESS, "no resource diagnostics\n".to_string());
    }

    let mut out = String::new();
    for d in &diags {
        match &d.path {
            Some(p) => out.push_str(&format!("{}: {} ({})\n", d.level, d.message, p.display())),
            None => out.push_str(&format!("{}: {}\n", d.level, d.message)),
        }
    }
    // Any diagnostic (warning or error) counts as an issue.
    (ExitCode::FAILURE, out)
}

// ── helpers ───────────────────────────────────────────────────────────

fn scope_of(path: &Path, agent_dir: &Path, cwd: &Path) -> &'static str {
    if path.starts_with(agent_dir) {
        "user"
    } else if path.starts_with(cwd) {
        "project"
    } else {
        "other"
    }
}

fn format_skill(s: &SkillInfo, agent_dir: &Path, cwd: &Path) -> String {
    let mut out = String::new();
    out.push_str("kind:    skill\n");
    out.push_str(&format!("name:    {}\n", s.name));
    out.push_str(&format!(
        "scope:   {}\n",
        scope_of(&s.source_info.path, agent_dir, cwd)
    ));
    out.push_str(&format!("path:    {}\n", s.source_info.path.display()));
    match &s.description {
        Some(d) => out.push_str(&format!("desc:    {d}\n")),
        None => out.push_str("desc:    (none)\n"),
    }
    out
}

fn format_theme(t: &ThemeInfo, agent_dir: &Path, cwd: &Path) -> String {
    let mut out = String::new();
    out.push_str("kind:    theme\n");
    out.push_str(&format!("name:    {}\n", t.name));
    out.push_str(&format!(
        "scope:   {}\n",
        scope_of(&t.source_info.path, agent_dir, cwd)
    ));
    out.push_str(&format!("path:    {}\n", t.source_info.path.display()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Write a valid global skill and theme under `agent_dir`.
    fn fixture(agent_dir: &Path) {
        // skill with valid frontmatter
        let skill_dir = agent_dir.join("skills").join("demo-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: demo-skill\ndescription: a demo\n---\nbody\n",
        )
        .unwrap();

        // leftover prompts dir (must not appear in list output)
        fs::create_dir_all(agent_dir.join("prompts")).unwrap();
        fs::write(
            agent_dir.join("prompts").join("greeting.md"),
            "---\nname: greeting\n---\nhi\n",
        )
        .unwrap();

        // theme
        fs::create_dir_all(agent_dir.join("themes")).unwrap();
        fs::write(agent_dir.join("themes").join("dark.json"), "{}").unwrap();
    }

    /// A realistic two-directory layout: global `agent_dir` separate from `cwd`.
    fn layout() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path().join("project");
        let agent_dir = tmp.path().join("home").join(".xylitol");
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&agent_dir).unwrap();
        (tmp, cwd, agent_dir)
    }

    #[test]
    fn list_shows_all_resources_with_scope() {
        let (_tmp, cwd, agent_dir) = layout();
        fixture(&agent_dir);

        let (code, out) = run_with_dirs(ResourcesAction::List, &cwd, &agent_dir);
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("skills:"));
        assert!(out.contains("demo-skill [user]"));
        assert!(!out.contains("prompts:"));
        assert!(!out.contains("greeting"));
        assert!(out.contains("themes:"));
        assert!(out.contains("dark [user]"));
    }

    #[test]
    fn list_ignores_leftover_prompts_dir() {
        let (_tmp, cwd, agent_dir) = layout();
        fs::create_dir_all(agent_dir.join("prompts")).unwrap();
        fs::write(agent_dir.join("prompts").join("greet.md"), "hi").unwrap();

        let (code, out) = run_with_dirs(ResourcesAction::List, &cwd, &agent_dir);
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(!out.contains("prompts:"));
        assert!(!out.contains("greet"));
    }

    #[test]
    fn list_empty_when_nothing_installed() {
        let (_tmp, cwd, agent_dir) = layout();
        let (code, out) = run_with_dirs(ResourcesAction::List, &cwd, &agent_dir);
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("(none)"));
    }

    #[test]
    fn info_found_exits_zero() {
        let (_tmp, cwd, agent_dir) = layout();
        fixture(&agent_dir);
        let (code, out) = run_with_dirs(
            ResourcesAction::Info {
                name: "demo-skill".into(),
            },
            &cwd,
            &agent_dir,
        );
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("kind:    skill"));
        assert!(out.contains("name:    demo-skill"));
        assert!(out.contains("desc:    a demo"));
    }

    #[test]
    fn info_missing_exits_failure() {
        let (_tmp, cwd, agent_dir) = layout();
        fixture(&agent_dir);
        let (code, out) = run_with_dirs(
            ResourcesAction::Info {
                name: "does-not-exist".into(),
            },
            &cwd,
            &agent_dir,
        );
        assert_eq!(code, ExitCode::FAILURE);
        assert!(out.contains("resource not found: does-not-exist"));
    }

    #[test]
    fn doctor_clean_exits_zero() {
        let (_tmp, cwd, agent_dir) = layout();
        fixture(&agent_dir); // all valid
        let (code, out) = run_with_dirs(ResourcesAction::Doctor, &cwd, &agent_dir);
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("no resource diagnostics"));
    }

    #[test]
    fn doctor_with_issue_exits_failure() {
        let (_tmp, cwd, agent_dir) = layout();
        // SKILL.md without frontmatter: dir name fills `name`, but description is still missing.
        let broken = agent_dir.join("skills").join("broken");
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join("SKILL.md"), "no frontmatter here\n").unwrap();
        let (code, out) = run_with_dirs(ResourcesAction::Doctor, &cwd, &agent_dir);
        assert_eq!(code, ExitCode::FAILURE);
        assert!(
            out.contains("missing description"),
            "expected description diagnostic; got {out}"
        );
    }

    #[test]
    fn project_scope_is_detected() {
        let (_tmp, cwd, agent_dir) = layout();
        // resource placed under cwd/.xylitol/skills (project scope)
        let proj_skill = cwd.join(".xylitol").join("skills").join("proj");
        fs::create_dir_all(&proj_skill).unwrap();
        fs::write(
            proj_skill.join("SKILL.md"),
            "---\nname: proj\ndescription: p\n---\nx\n",
        )
        .unwrap();
        let (_, out) = run_with_dirs(ResourcesAction::List, &cwd, &agent_dir);
        assert!(out.contains("proj [project]"));
    }
}
