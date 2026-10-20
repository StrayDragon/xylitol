use std::path::PathBuf;

use tempfile::TempDir;
use xylitol_runtime::resource::DefaultResourceLoader;

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
        xylitol_plugin_sdk::source_info::SourceScope::Project
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
        xylitol_plugin_sdk::source_info::SourceScope::User
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
