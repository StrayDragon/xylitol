use std::cell::RefCell;
use std::path::PathBuf;

use crate::bdd::fixtures::Workspace;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_kernel::boot::{BootOptions, Profile, boot_ctx};
use xylitol_plugin_sdk::rt::CtxHost;

pub struct AgentSubagentBdd {
    pub config_path: RefCell<Option<PathBuf>>,
    pub tool_names: RefCell<Vec<String>>,
    pub error: RefCell<Option<String>>,
}

#[fixture]
pub fn agent_subagent_bdd() -> AgentSubagentBdd {
    AgentSubagentBdd {
        config_path: RefCell::new(None),
        tool_names: RefCell::new(Vec::new()),
        error: RefCell::new(None),
    }
}

fn base_config() -> String {
    "models:\n  default_model: q\n  models:\n    q:\n      provider: openai\n      model: m\n"
        .to_string()
}

fn prepare(ws: &Workspace, bdd: &AgentSubagentBdd, body: &str) {
    ws.init();
    let path = ws.ws("config.yaml");
    std::fs::write(&path, body).expect("write config");
    bdd.config_path.replace(Some(PathBuf::from(path)));
}

#[given("配置不含 subagent")]
fn g_without_subagent(ws: &Workspace, agent_subagent_bdd: &AgentSubagentBdd) {
    prepare(ws, agent_subagent_bdd, &base_config());
}

#[given("配置含 subagent tools")]
fn g_with_subagent(ws: &Workspace, agent_subagent_bdd: &AgentSubagentBdd) {
    let mut body = base_config();
    body.push_str("subagent:\n  tools: [read, grep, find, ls]\n");
    prepare(ws, agent_subagent_bdd, &body);
}

#[when("内核装配子代理")]
async fn w_boot(agent_subagent_bdd: &AgentSubagentBdd) {
    let config_path = agent_subagent_bdd
        .config_path
        .borrow()
        .clone()
        .expect("config prepared");
    let opts = BootOptions {
        profile: Profile::Print,
        config_path: Some(config_path),
        session: None,
        model: None,
        trust_override: Some(false),
        interactive: false,
        caller: "bdd",
        ephemeral_sessions: true,
    };
    match boot_ctx(&opts).await {
        Ok((ctx, _loader)) => {
            let names = ctx
                .get::<xylitol_plugin_sdk::HostRegistries>()
                .map(|regs| {
                    regs.tools
                        .snapshot()
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            agent_subagent_bdd.tool_names.replace(names);
            agent_subagent_bdd.error.replace(None);
            let _ = ctx.shutdown().await;
        }
        Err(error) => {
            agent_subagent_bdd.tool_names.replace(Vec::new());
            agent_subagent_bdd.error.replace(Some(error.to_string()));
        }
    }
}

#[then("工具注册表不含 subagent_readonly")]
fn t_no_subagent(agent_subagent_bdd: &AgentSubagentBdd) {
    assert!(
        agent_subagent_bdd.error.borrow().is_none(),
        "boot failed: {:?}",
        agent_subagent_bdd.error.borrow()
    );
    let names = agent_subagent_bdd.tool_names.borrow();
    assert!(
        names.iter().all(|name| name != "subagent_readonly"),
        "no subagent tool expected: {names:?}"
    );
}

#[then("工具注册表含 subagent_readonly")]
fn t_has_subagent(agent_subagent_bdd: &AgentSubagentBdd) {
    let names = agent_subagent_bdd.tool_names.borrow();
    assert!(
        names.iter().any(|name| name == "subagent_readonly"),
        "subagent tool expected: {names:?}"
    );
}
