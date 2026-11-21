use std::cell::RefCell;
use std::path::{Path, PathBuf};

use crate::bdd::fixtures::Workspace;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_kernel::boot::{BootOptions, Profile, boot_ctx};
use xylitol_plugin_sdk::rt::CtxHost;

pub struct InfraPluginBdd {
    pub config_path: RefCell<Option<PathBuf>>,
    pub script_path: RefCell<Option<PathBuf>>,
    pub tool_names: RefCell<Vec<String>>,
    pub mcp_servers: RefCell<Vec<xylitol_official_plugin_mcp::McpServerConfig>>,
    pub error: RefCell<Option<String>>,
    pub call_result: RefCell<Option<String>>,
}

#[fixture]
pub fn infra_plugin_bdd() -> InfraPluginBdd {
    InfraPluginBdd {
        config_path: RefCell::new(None),
        script_path: RefCell::new(None),
        tool_names: RefCell::new(Vec::new()),
        mcp_servers: RefCell::new(Vec::new()),
        error: RefCell::new(None),
        call_result: RefCell::new(None),
    }
}

fn mcp_declaring_script() -> &'static str {
    r#"#!/usr/bin/env python3
import json, sys
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    if request.get("method") == "initialize":
        result = {
            "protocol_version": 1,
            "name": "fixture",
            "version": "0.1.0",
            "capabilities": ["commands"],
            "commands": [{"name": "run", "description": "Run the fixture"}],
            "mcp": {"command": "fixture-mcp-server", "args": ["--stdio"]},
        }
    elif request.get("method") == "commands/call":
        result = {"text": "ok"}
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32601, "message": "unknown"}}), flush=True)
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#
}

fn non_mcp_script() -> &'static str {
    r#"#!/usr/bin/env python3
import json, sys
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    if request.get("method") == "initialize":
        result = {
            "protocol_version": 1,
            "name": "fixture",
            "version": "0.1.0",
            "capabilities": ["commands"],
            "commands": [{"name": "run", "description": "Run the fixture"}],
        }
    elif request.get("method") == "commands/call":
        result = {"text": "ok"}
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32601, "message": "unknown"}}), flush=True)
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#
}

fn write_script(ws: &Workspace, name: &str, body: &str) -> PathBuf {
    let path = PathBuf::from(ws.ws(name));
    std::fs::write(&path, body).expect("write fixture script");
    path
}

fn plugins_bin_dir(ws: &Workspace) -> PathBuf {
    let dir = PathBuf::from(ws.ws("plugins")).join("bin");
    std::fs::create_dir_all(&dir).expect("create plugins/bin");
    dir
}

fn write_config(ws: &Workspace, plugins_yaml: &str) {
    let path = PathBuf::from(ws.ws("config.yaml"));
    let config = format!(
        "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\n{plugins_yaml}"
    );
    std::fs::write(&path, config).expect("write config");
}

fn prepare_discovered(
    ws: &Workspace,
    bdd: &InfraPluginBdd,
    name: &str,
    body: &str,
    plugins_yaml: &str,
) {
    ws.init();
    let script = write_script(ws, "fixture.py", body);
    let bin = plugins_bin_dir(ws);
    let entry = bin.join(name);
    std::fs::copy(&script, &entry).expect("install plugin into plugins/bin");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&entry).expect("stat").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&entry, perms).expect("chmod plugin");
    }
    write_config(ws, plugins_yaml);
    bdd.config_path
        .replace(Some(PathBuf::from(ws.ws("config.yaml"))));
    bdd.script_path.replace(Some(entry));
}

#[given("发现清单声明 mcp 的外部插件且未经用户批准")]
fn g_discovered_mcp_unapproved(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare_discovered(
        ws,
        infra_plugin_bdd,
        "fixture",
        mcp_declaring_script(),
        "plugins:\n  enabled: [\"fixture\"]\n",
    );
}

#[given("发现清单声明 mcp 的外部插件且用户已批准")]
fn g_discovered_mcp_approved(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare_discovered(
        ws,
        infra_plugin_bdd,
        "fixture",
        mcp_declaring_script(),
        "plugins:\n  enabled: [\"fixture\"]\n",
    );
    let store = PathBuf::from(ws.ws("plugins")).join("approved.json");
    std::fs::write(&store, "{\"version\": 1, \"approved\": [\"fixture\"]}\n")
        .expect("write approval store");
}

#[given("已批准的外部插件被 config 显式停用")]
fn g_discovered_mcp_disabled(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare_discovered(
        ws,
        infra_plugin_bdd,
        "fixture",
        mcp_declaring_script(),
        "plugins:\n  enabled: [\"fixture\"]\n  disabled: [\"fixture\"]\n",
    );
}

#[given("发现清单未声明 mcp 的外部插件")]
fn g_discovered_no_mcp(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare_discovered(
        ws,
        infra_plugin_bdd,
        "fixture",
        non_mcp_script(),
        "plugins:\n  enabled: [\"fixture\"]\n",
    );
}

#[when("内核装配")]
async fn w_boot(infra_plugin_bdd: &InfraPluginBdd) {
    let config_path = infra_plugin_bdd
        .config_path
        .borrow()
        .clone()
        .expect("config prepared");
    let plugin_dir = infra_plugin_bdd
        .script_path
        .borrow()
        .clone()
        .and_then(|script| script.parent().map(Path::to_path_buf))
        .expect("plugin installed");
    if let Some(parent) = config_path.parent() {
        unsafe {
            std::env::set_var("XYLITOL_CONFIG_DIR", parent.join(".config").join("xylitol"));
            std::env::set_var("HOME", parent);
            std::env::set_var("XYLITOL_PLUGINS_DIR", &plugin_dir);
        }
    }
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
            let servers = ctx
                .get::<xylitol_kernel::external_plugin::ExternalMcpRegistry>()
                .map(|registry| registry.servers())
                .unwrap_or_default();
            infra_plugin_bdd.tool_names.replace(names);
            infra_plugin_bdd.mcp_servers.replace(servers);
            infra_plugin_bdd.error.replace(None);
            let _ = ctx.shutdown().await;
        }
        Err(error) => {
            infra_plugin_bdd.tool_names.replace(Vec::new());
            infra_plugin_bdd.mcp_servers.replace(Vec::new());
            infra_plugin_bdd.error.replace(Some(error.to_string()));
        }
    }
}

fn mcp_tool_names(names: &[String], server: &str) -> Vec<String> {
    let prefix = format!("mcp__{server}__");
    names
        .iter()
        .filter(|name| name.starts_with(&prefix))
        .cloned()
        .collect()
}

#[then("工具注册表不含该插件的 mcp__ 工具")]
fn t_unapproved_no_mcp_tools(infra_plugin_bdd: &InfraPluginBdd) {
    let names = infra_plugin_bdd.tool_names.borrow();
    assert!(
        mcp_tool_names(&names, "fixture").is_empty(),
        "an unapproved plugin must not register any mcp__ tool: {names:?}"
    );
    let servers = infra_plugin_bdd.mcp_servers.borrow();
    assert!(
        servers.iter().all(|server| server.name != "fixture"),
        "an unapproved plugin must not register an MCP server: {servers:?}"
    );
}

#[then("工具注册表含该插件的 mcp__ 工具")]
fn t_approved_mcp_tools(infra_plugin_bdd: &InfraPluginBdd) {
    let servers = infra_plugin_bdd.mcp_servers.borrow();
    let server = servers
        .iter()
        .find(|server| server.name == "fixture")
        .unwrap_or_else(|| {
            panic!(
                "an approved plugin declaring mcp must register as an MCP server: {servers:?}; error={:?}",
                infra_plugin_bdd.error.borrow()
            )
        });
    assert_eq!(
        server.command.as_deref(),
        Some("fixture-mcp-server"),
        "the declared mcp command must be the registered server command"
    );
    assert_eq!(
        server.args.as_deref(),
        Some(["--stdio".to_string()].as_slice()),
        "the declared mcp args must reach the registered server"
    );
}

#[then("工具注册表不含该插件的工具")]
fn t_disabled_no_tools(infra_plugin_bdd: &InfraPluginBdd) {
    let names = infra_plugin_bdd.tool_names.borrow();
    assert!(
        mcp_tool_names(&names, "fixture").is_empty(),
        "a config-disabled plugin must not register any tool: {names:?}"
    );
    let servers = infra_plugin_bdd.mcp_servers.borrow();
    assert!(
        servers.iter().all(|server| server.name != "fixture"),
        "config disabled must override approval and drop the MCP server: {servers:?}"
    );
}

#[then("该插件不注册为 MCP server")]
fn t_no_mcp_declaration(infra_plugin_bdd: &InfraPluginBdd) {
    let servers = infra_plugin_bdd.mcp_servers.borrow();
    assert!(
        servers.iter().all(|server| server.name != "fixture"),
        "a plugin without an mcp declaration must not register an MCP server: {servers:?}"
    );
}
