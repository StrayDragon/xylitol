use std::cell::RefCell;
use std::path::PathBuf;

use crate::bdd::fixtures::Workspace;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_kernel::boot::{BootOptions, Profile, boot_ctx};
use xylitol_plugin_sdk::rt::CtxHost;

pub struct LspDapBdd {
    pub config_path: RefCell<Option<PathBuf>>,
    pub tool_names: RefCell<Vec<String>>,
    pub error: RefCell<Option<String>>,
}

#[fixture]
pub fn lsp_dap_bdd() -> LspDapBdd {
    LspDapBdd {
        config_path: RefCell::new(None),
        tool_names: RefCell::new(Vec::new()),
        error: RefCell::new(None),
    }
}

fn base_config() -> String {
    "models:\n  default_model: q\n  models:\n    q:\n      provider: openai\n      model: m\n"
        .to_string()
}

fn write_config(ws: &Workspace, body: &str) {
    let path = ws.ws("config.yaml");
    std::fs::write(&path, body).expect("write config");
}

fn prepare(ws: &Workspace, bdd: &LspDapBdd, body: &str) {
    ws.init();
    write_config(ws, body);
    bdd.config_path
        .replace(Some(PathBuf::from(ws.ws("config.yaml"))));
}

#[given("配置不含 lsp")]
fn g_without_lsp(ws: &Workspace, lsp_dap_bdd: &LspDapBdd) {
    prepare(ws, lsp_dap_bdd, &base_config());
}

#[given("配置不含 dap")]
fn g_without_dap(ws: &Workspace, lsp_dap_bdd: &LspDapBdd) {
    prepare(ws, lsp_dap_bdd, &base_config());
}

#[given("配置含 lsp backend basedpyright")]
fn g_with_lsp(ws: &Workspace, lsp_dap_bdd: &LspDapBdd) {
    let mut body = base_config();
    body.push_str(
        "lsp:\n  backend: \"basedpyright-langserver --stdio\"\n  language: \"python\"\n  compression: true\n",
    );
    prepare(ws, lsp_dap_bdd, &body);
}

#[given("配置含 dap backend debugpy")]
fn g_with_dap(ws: &Workspace, lsp_dap_bdd: &LspDapBdd) {
    let mut body = base_config();
    body.push_str("dap:\n  backend: \"python3 -m debugpy.adapter\"\n  compression: true\n");
    prepare(ws, lsp_dap_bdd, &body);
}

#[when("内核装配 LSP DAP")]
async fn w_boot(lsp_dap_bdd: &LspDapBdd) {
    let config_path = lsp_dap_bdd
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
            lsp_dap_bdd.tool_names.replace(names);
            lsp_dap_bdd.error.replace(None);
            let _ = ctx.shutdown().await;
        }
        Err(error) => {
            lsp_dap_bdd.tool_names.replace(Vec::new());
            lsp_dap_bdd.error.replace(Some(error.to_string()));
        }
    }
}

#[then("工具注册表不含 lsp 前缀工具")]
fn t_no_lsp(lsp_dap_bdd: &LspDapBdd) {
    assert!(
        lsp_dap_bdd.error.borrow().is_none(),
        "boot failed: {:?}",
        lsp_dap_bdd.error.borrow()
    );
    let names = lsp_dap_bdd.tool_names.borrow();
    assert!(
        names.iter().all(|name| !name.starts_with("lsp_")),
        "no lsp tools expected: {names:?}"
    );
}

#[then("工具注册表不含 dap 前缀工具")]
fn t_no_dap(lsp_dap_bdd: &LspDapBdd) {
    assert!(
        lsp_dap_bdd.error.borrow().is_none(),
        "boot failed: {:?}",
        lsp_dap_bdd.error.borrow()
    );
    let names = lsp_dap_bdd.tool_names.borrow();
    assert!(
        names.iter().all(|name| !name.starts_with("dap_")),
        "no dap tools expected: {names:?}"
    );
}

#[then("工具注册表含 lsp_diagnostics 与 lsp_symbols 与 lsp_hover")]
fn t_has_lsp(lsp_dap_bdd: &LspDapBdd) {
    let names = lsp_dap_bdd.tool_names.borrow();
    for expected in ["lsp_diagnostics", "lsp_symbols", "lsp_hover"] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}: {names:?}"
        );
    }
}

#[then("工具注册表含 dap_launch 与 dap_stack 与 dap_evaluate 与 dap_disconnect")]
fn t_has_dap(lsp_dap_bdd: &LspDapBdd) {
    let names = lsp_dap_bdd.tool_names.borrow();
    for expected in ["dap_launch", "dap_stack", "dap_evaluate", "dap_disconnect"] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}: {names:?}"
        );
    }
}
