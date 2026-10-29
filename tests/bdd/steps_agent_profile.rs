use crate::bdd::fixtures::Workspace;
use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_kernel::bootstrap::{BootstrapInput, ResolvedAssembly, resolve_assembly_with};

pub struct AgentProfileBdd {
    pub assembly: RefCell<Option<ResolvedAssembly>>,
    pub error: RefCell<Option<String>>,
    pub config_path: RefCell<Option<String>>,
}

#[fixture]
pub fn agent_profile_bdd() -> AgentProfileBdd {
    AgentProfileBdd {
        assembly: RefCell::new(None),
        error: RefCell::new(None),
        config_path: RefCell::new(None),
    }
}

fn profile_yaml() -> &'static str {
    "models:\n  default_model: fake\n  models:\n    fake:\n      provider: fake\n      model: fake-1\nagents:\n  profiles:\n    review:\n      model: fake\n      persona: \"You are a reviewer.\"\n      system_prompt: \"Focus on correctness.\"\n      tools: [read, -bash]\n"
}

#[given("YAML 含 agents.profiles.review 的 persona 与 tools")]
fn g_profile_with_persona(ws: &Workspace, agent_profile_bdd: &AgentProfileBdd) {
    ws.init();
    let path = ws.ws("agent-profile.yaml");
    std::fs::write(&path, profile_yaml()).expect("write profile config");
    agent_profile_bdd.config_path.replace(Some(path));
}

#[given("YAML 不含 agents.profiles.missing")]
fn g_profile_without_missing(ws: &Workspace, agent_profile_bdd: &AgentProfileBdd) {
    ws.init();
    let path = ws.ws("agent-profile-missing.yaml");
    std::fs::write(
        &path,
        "models:\n  default_model: fake\n  models:\n    fake:\n      provider: fake\n      model: fake-1\n",
    )
    .expect("write profile config");
    agent_profile_bdd.config_path.replace(Some(path));
}

fn load_profile(ws: &Workspace, agent_profile_bdd: &AgentProfileBdd, name: &str) {
    let path = agent_profile_bdd
        .config_path
        .borrow()
        .clone()
        .expect("profile config path prepared");
    let isolated = ws.ws("isolated-global-config");
    std::fs::create_dir_all(&isolated).expect("create isolated config dir");
    let env_dir = isolated.clone();
    let env_probe = move |key: &str| match key {
        "XYLITOL_CONFIG_DIR" | "HOME" => Some(env_dir.clone()),
        _ => None,
    };
    let input = BootstrapInput {
        config_path: Some(path.into()),
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "bdd",
        ephemeral_sessions: false,
        static_tools: Vec::new(),
        agent_profile: Some(name.to_string()),
    };
    let result = resolve_assembly_with(&input, env_probe, Some(std::path::Path::new(&ws.root())));
    match result {
        Ok(assembly) => {
            agent_profile_bdd.assembly.replace(Some(assembly));
            agent_profile_bdd.error.replace(None);
        }
        Err(e) => {
            agent_profile_bdd.assembly.replace(None);
            agent_profile_bdd.error.replace(Some(e.to_string()));
        }
    }
}

#[when("以 --agent-profile review 解析装配")]
fn w_load_review_profile(ws: &Workspace, agent_profile_bdd: &AgentProfileBdd) {
    load_profile(ws, agent_profile_bdd, "review");
}

#[then("系统提示按 persona 在前拼接且工具选择为 profile tools")]
fn t_profile_persona_and_tools(agent_profile_bdd: &AgentProfileBdd) {
    let assembly = agent_profile_bdd.assembly.borrow();
    let assembly = assembly.as_ref().expect("assembly resolved");
    assert_eq!(
        assembly.system_prompt.as_deref(),
        Some("You are a reviewer.\n\nFocus on correctness.")
    );
    assert_eq!(
        assembly.tool_features.tool_selection,
        vec!["read".to_string(), "-bash".to_string()]
    );
}

#[when("以 --agent-profile missing 解析装配")]
fn w_load_missing_profile(ws: &Workspace, agent_profile_bdd: &AgentProfileBdd) {
    load_profile(ws, agent_profile_bdd, "missing");
}

#[then("装配失败且错误指向 agents.profiles")]
fn t_profile_missing_fails(agent_profile_bdd: &AgentProfileBdd) {
    assert!(
        agent_profile_bdd.assembly.borrow().is_none(),
        "missing profile must not resolve"
    );
    let error = agent_profile_bdd.error.borrow();
    let error = error.as_ref().expect("profile error");
    assert!(
        error.contains("missing"),
        "error must name profile: {error}"
    );
    assert!(
        error.contains("agents.profiles"),
        "error must point at agents.profiles: {error}"
    );
}
