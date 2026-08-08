use super::*;
use crate::app::core::composition::{BuildAgentOptions, build_agent};
use crate::app::core::driver::XyInProcessDriver;
use crate::protocol::ports::XySessionStore;
use std::sync::Arc;

fn make_driver() -> XyInProcessDriver {
    let agent = build_agent(BuildAgentOptions::default()).expect("build");
    let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
        tempfile::tempdir().unwrap().path().join("sessions"),
    ));
    XyInProcessDriver::new(agent, store)
}

#[test]
fn reload_prompt_context_trusted_injects_agents() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "TRUSTED_AGENTS_BODY").unwrap();

    let mut driver = make_driver();
    let report = reload_prompt_context(&mut driver, project.path(), agent_dir.path(), true, None);
    assert!(report.context_file_count >= 1);
    let sp = driver.system_prompt_for_test().unwrap_or_default();
    assert!(
        sp.contains("TRUSTED_AGENTS_BODY"),
        "trusted reload must inject project AGENTS.md"
    );
}

#[test]
fn reload_prompt_context_untrusted_skips_project_agents() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "SECRET_PROJECT_AGENTS").unwrap();

    let mut driver = make_driver();
    let _ = reload_prompt_context(&mut driver, project.path(), agent_dir.path(), false, None);
    let sp = driver.system_prompt_for_test().unwrap_or_default();
    assert!(
        !sp.contains("SECRET_PROJECT_AGENTS"),
        "untrusted reload must not inject project AGENTS.md"
    );
}

fn write_skill(dir: &std::path::Path, name: &str) {
    let skill_dir = dir.join(".xylitol").join("skills").join(name);
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: test\n---\n\nbody\n"),
    )
    .unwrap();
}

#[test]
fn reload_skills_trusted_injects_into_system_prompt() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    write_skill(project.path(), "reload-demo");

    let mut driver = make_driver();
    let before_len = driver.system_prompt_for_test().unwrap_or_default().len();
    let report = reload_skills(&mut driver, project.path(), agent_dir.path(), true);
    assert!(report.names.iter().any(|n| n == "reload-demo"));
    let sp = driver.system_prompt_for_test().unwrap_or_default();
    assert!(sp.contains("reload-demo") || sp.contains("<available_skills>"));
    assert!(sp.len() >= before_len);
    assert!(
        driver
            .loaded_skill_names()
            .iter()
            .any(|n| n == "reload-demo")
    );
}

#[test]
fn reload_skills_untrusted_skips_project_skill() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    write_skill(project.path(), "secret-reload");

    let mut driver = make_driver();
    let _ = reload_skills(&mut driver, project.path(), agent_dir.path(), false);
    let sp = driver.system_prompt_for_test().unwrap_or_default();
    assert!(!sp.contains("secret-reload"));
    assert!(
        !driver
            .loaded_skill_names()
            .iter()
            .any(|n| n == "secret-reload")
    );
}

/// RAII env restore for bootstrap path tests.
struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, val: &str) -> Self {
        let prev = std::env::var(key).ok();
        unsafe { std::env::set_var(key, val) };
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => unsafe { std::env::set_var(self.key, v) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

#[test]
fn config_template_error_is_hard_fail() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    let proj_xy = project.join(".xylitol");
    std::fs::create_dir_all(&proj_xy).unwrap();
    let global = home.path().join(".config").join("xylitol");
    std::fs::create_dir_all(&global).unwrap();

    let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
    let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
    let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
    let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");

    std::fs::write(
        proj_xy.join("config.yaml"),
        "# doc {{ secret.KEY }}\nmodels:\n  default_model: q\n  models:\n    q:\n      provider: openai\n      model: m\n",
    )
    .unwrap();

    let err = match resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    }) {
        Ok(_) => panic!("template in comment must fail closed"),
        Err(e) => e,
    };
    assert!(
        matches!(err, BootstrapError::ConfigLoadFailed(_)),
        "got {err}"
    );
}

#[test]
fn yaml_zero_models_hard_fail_no_env_gpt4o() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    let proj_xy = project.join(".xylitol");
    std::fs::create_dir_all(&proj_xy).unwrap();
    let global = home.path().join(".config").join("xylitol");
    std::fs::create_dir_all(&global).unwrap();

    let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
    let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
    let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
    let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");

    // Explicit empty models map (config present, zero registerable models).
    std::fs::write(
        proj_xy.join("config.yaml"),
        "models:\n  default_model: x\n  models: {}\n",
    )
    .unwrap();

    let err = match resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    }) {
        Ok(_) => panic!("zero models from yaml must hard fail"),
        Err(e) => e,
    };
    assert!(
        matches!(err, BootstrapError::ConfigLoadedZeroModels),
        "got {err}"
    );
}

#[test]
fn env_only_registers_but_bootstrap_does_not_select() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    let global = home.path().join(".config").join("xylitol");
    std::fs::create_dir_all(&global).unwrap();

    let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
    let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
    let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
    let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");
    unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };

    let assembly = resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    })
    .expect("env-only assembly");
    assert!(
        !assembly.model_registry.list().is_empty(),
        "env key should populate registry for discovery"
    );

    let boot = bootstrap(BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    })
    .expect("bootstrap without --model");
    assert!(
        boot.agent.current_model().is_none(),
        "must not silent-select gpt-4o"
    );
    assert_eq!(UNSET_MODEL_DISPLAY, "NOT-SET");
}

#[test]
fn yaml_model_api_is_honored_in_registry_config() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    let proj_xy = project.join(".xylitol");
    std::fs::create_dir_all(&proj_xy).unwrap();
    let global = home.path().join(".config").join("xylitol");
    std::fs::create_dir_all(&global).unwrap();

    let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
    let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
    let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
    let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");

    std::fs::write(
        proj_xy.join("config.yaml"),
        r#"models:
  default_model: completions-model
  models:
    completions-model:
      provider: openai
      model: m-completions
      api: openai-completions
    responses-model:
      provider: openai
      model: m-responses
      api: openai-responses
    default-api-model:
      provider: openai
      model: m-default
"#,
    )
    .unwrap();

    let assembly = resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    })
    .expect("assembly");

    let list = assembly.model_registry.list();
    let by_id = |id: &str| {
        list.iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("missing {id}"))
    };

    assert_eq!(
        by_id("completions-model").config.api.as_deref(),
        Some("openai-completions")
    );
    assert_eq!(by_id("completions-model").api, "openai-completions");
    // c1940: explicit Completions is first-class
    assert_eq!(
        crate::infra::provider::adapter::factory::resolve_adapter_kind(
            &by_id("completions-model").config
        ),
        crate::infra::provider::adapter::AdapterKind::OpenAiCompletions
    );

    assert_eq!(
        by_id("responses-model").config.api.as_deref(),
        Some("openai-responses")
    );
    assert_eq!(
        crate::infra::provider::adapter::factory::resolve_adapter_kind(
            &by_id("responses-model").config
        ),
        crate::infra::provider::adapter::AdapterKind::OpenAiResponses
    );

    // Omitted api → default_for(OpenAi) = Responses
    assert_eq!(by_id("default-api-model").config.api, None);
    assert_eq!(
        crate::infra::provider::adapter::factory::resolve_adapter_kind(
            &by_id("default-api-model").config
        ),
        crate::infra::provider::adapter::AdapterKind::OpenAiResponses
    );
}

#[test]
fn build_agent_with_skills_injects_available_skills_section() {
    use crate::protocol::resource::SkillInfo;
    use crate::protocol::source_info::{SourceInfo, SourceOrigin, SourceScope};

    let opts = BuildAgentOptions {
        skills: vec![SkillInfo {
            name: "boot-skill".into(),
            description: Some("from build options".into()),
            source_info: SourceInfo {
                path: std::path::PathBuf::from("/tmp/boot/SKILL.md"),
                source: "user".into(),
                scope: SourceScope::User,
                origin: SourceOrigin::TopLevel,
                base_dir: None,
            },
            disable_model_invocation: false,
        }],
        ..Default::default()
    };
    let agent = build_agent(opts).expect("build");
    let sp = agent.system_prompt().unwrap_or("");
    assert!(sp.contains("<available_skills>"));
    assert!(sp.contains("boot-skill"));
    assert!(sp.contains("Use the read tool"));
    assert_eq!(agent.loaded_skill_names(), vec!["boot-skill".to_string()]);
}

#[test]
fn untrusted_reload_still_loads_user_global_skills() {
    let project = tempfile::tempdir().unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    write_skill(project.path(), "project-only");
    let user_skill = agent_dir.path().join("skills").join("user-global");
    std::fs::create_dir_all(&user_skill).unwrap();
    std::fs::write(
        user_skill.join("SKILL.md"),
        "---\nname: user-global\ndescription: always\n---\n",
    )
    .unwrap();

    let mut driver = make_driver();
    let report = reload_skills(&mut driver, project.path(), agent_dir.path(), false);
    assert!(
        report.names.iter().any(|n| n == "user-global"),
        "untrusted must still load user skills; got {:?}",
        report.names
    );
    assert!(!report.names.iter().any(|n| n == "project-only"));
    let sp = driver.system_prompt_for_test().unwrap_or_default();
    assert!(sp.contains("user-global"));
    assert!(!sp.contains("project-only"));
}

#[test]
#[serial_test::serial(bootstrap_cwd)]
fn trust_override_loads_or_skips_project_xylitol_skills() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    write_skill(&project, "c1620-proj-skill");
    let global = home.path().join(".config").join("xylitol");
    std::fs::create_dir_all(&global).unwrap();

    let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
    let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
    let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
    let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");
    let prev_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project).unwrap();
    struct CwdRestore(std::path::PathBuf);
    impl Drop for CwdRestore {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }
    let _cwd = CwdRestore(prev_cwd);

    let trusted = resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    })
    .expect("trusted assembly");
    assert!(
        trusted.skills.iter().any(|s| s.name == "c1620-proj-skill"),
        " --trust must discover .xylitol/skills; got {:?}",
        trusted.skills.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    let denied = resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(false),
        interactive: false,
        caller: "test",
    })
    .expect("untrusted assembly");
    assert!(
        denied
            .warnings
            .iter()
            .any(|w| matches!(w, BootstrapWarning::ProjectNotTrusted { .. })),
        " --no-trust must warn ProjectNotTrusted; got {:?}",
        denied.warnings
    );
    assert!(
        !denied.skills.iter().any(|s| s.name == "c1620-proj-skill"),
        "--no-trust must skip project .xylitol skills"
    );
}

#[test]
fn resolve_assembly_reads_session_max_turns() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    let proj_xy = project.join(".xylitol");
    std::fs::create_dir_all(&proj_xy).unwrap();
    let global = home.path().join(".config").join("xylitol");
    std::fs::create_dir_all(&global).unwrap();

    let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
    let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
    let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
    let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");

    std::fs::write(
        proj_xy.join("config.yaml"),
        r#"models:
  default_model: m1
  models:
    m1:
      provider: openai
      model: m
session:
  storage: {}
  max_turns: 7
"#,
    )
    .unwrap();

    let assembly = resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    })
    .expect("assembly");
    assert_eq!(assembly.max_turns, Some(7));
}

#[test]
fn bootstrap_block_on_runs_from_current_thread_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime");
    let result =
        runtime.block_on(async { block_on_bootstrap_task("test current-thread", async { 7 }) });
    assert_eq!(result, Some(7));
}

#[test]
fn missing_session_still_applies_settings_thinking_default() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    let project_xylitol = project.join(".xylitol");
    let global_config = home.path().join(".config").join("xylitol");
    let agent_dir = home.path().join(".xylitol");
    std::fs::create_dir_all(&project_xylitol).unwrap();
    std::fs::create_dir_all(&global_config).unwrap();
    std::fs::create_dir_all(&agent_dir).unwrap();

    std::fs::write(
        project_xylitol.join("config.yaml"),
        r#"models:
  default_model: m1
  models:
    m1:
      provider: fake
      model: m1
      thinking: true
      thinking_levels: [off, high, max]
"#,
    )
    .unwrap();
    std::fs::write(
        agent_dir.join("settings.json"),
        r#"{"defaultThinkingLevel":"high"}"#,
    )
    .unwrap();

    let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
    let _project = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
    let _config = EnvGuard::set(
        "XYLITOL_CONFIG_DIR",
        global_config.to_str().expect("UTF-8 config path"),
    );

    let input = || BootstrapInput {
        config_path: None,
        session: Some("missing-session".into()),
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "test",
    };
    let assembly = resolve_assembly(&input()).expect("assembly");
    assert!(!assembly.session_file_exists);

    let boot = bootstrap(input()).expect("bootstrap");
    assert_eq!(boot.agent.thinking_level(), "high");
}
