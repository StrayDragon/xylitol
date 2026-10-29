use std::cell::RefCell;
use std::path::PathBuf;

use crate::bdd::fixtures::Workspace;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_kernel::boot::{BootOptions, Profile, boot_ctx};
use xylitol_kernel::config::types::ExternalPluginConfig;
use xylitol_kernel::external_plugin::ExternalClient;
use xylitol_plugin_sdk::rt::CtxHost;

pub struct InfraPluginBdd {
    pub config_path: RefCell<Option<PathBuf>>,
    pub script_path: RefCell<Option<PathBuf>>,
    pub tool_names: RefCell<Vec<String>>,
    pub error: RefCell<Option<String>>,
    pub call_result: RefCell<Option<String>>,
}

#[fixture]
pub fn infra_plugin_bdd() -> InfraPluginBdd {
    InfraPluginBdd {
        config_path: RefCell::new(None),
        script_path: RefCell::new(None),
        tool_names: RefCell::new(Vec::new()),
        error: RefCell::new(None),
        call_result: RefCell::new(None),
    }
}

fn fixture_script() -> &'static str {
    r#"
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
            "capabilities": ["tools"],
            "tools": [{"name": "echo", "description": "Echo", "input_schema": {"type": "object"}}],
        }
    elif request.get("method") == "tools/call":
        result = {"content": str(request.get("params", {}).get("arguments", {}).get("text", ""))}
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32601, "message": "unknown"}}), flush=True)
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#
}

fn bad_protocol_script() -> &'static str {
    r#"
import json, sys
for line in sys.stdin:
    request = json.loads(line)
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": {"protocol_version": 99, "capabilities": ["tools"]}}), flush=True)
"#
}

fn write_script(ws: &Workspace, name: &str, body: &str) -> PathBuf {
    let path = PathBuf::from(ws.ws(name));
    std::fs::write(&path, body).expect("write fixture script");
    path
}

fn write_config(ws: &Workspace, trusted: bool, name: &str, command: &str, script: &PathBuf) {
    let path = PathBuf::from(ws.ws("config.yaml"));
    let config = format!(
        "models:\n  default_model: q\n  models:\n    q:\n      provider: openai\n      model: m\nexternal_plugins:\n  - name: \"{name}\"\n    command: \"{command}\"\n    args: [\"-u\", \"{}\"]\n    trusted: {trusted}\n    timeout_ms: 5000\n",
        script.to_string_lossy()
    );
    std::fs::write(&path, config).expect("write config");
}

fn prepare(ws: &Workspace, bdd: &InfraPluginBdd, trusted: bool, name: &str, command: &str) {
    ws.init();
    let script = write_script(ws, "fixture.py", fixture_script());
    write_config(ws, trusted, name, command, &script);
    bdd.config_path
        .replace(Some(PathBuf::from(ws.ws("config.yaml"))));
    bdd.script_path.replace(Some(script));
}

#[given("配置含受信外部插件 fixture")]
fn g_trusted_fixture(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare(ws, infra_plugin_bdd, true, "fixture", "python3");
}

#[given("配置含未受信外部插件 fixture")]
fn g_untrusted_fixture(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare(ws, infra_plugin_bdd, false, "fixture", "python3");
}

#[given("配置含名为 tools-builtin 的受信外部插件")]
fn g_reserved_name(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare(ws, infra_plugin_bdd, true, "tools-builtin", "python3");
}

#[given("配置含命令不存在的受信外部插件")]
fn g_missing_command(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    prepare(
        ws,
        infra_plugin_bdd,
        true,
        "missing-command",
        "definitely-not-a-real-command-xyz",
    );
}

#[when("内核装配")]
async fn w_boot(infra_plugin_bdd: &InfraPluginBdd) {
    let config_path = infra_plugin_bdd
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
            infra_plugin_bdd.tool_names.replace(names);
            infra_plugin_bdd.error.replace(None);
            let _ = ctx.shutdown().await;
        }
        Err(error) => {
            infra_plugin_bdd.tool_names.replace(Vec::new());
            infra_plugin_bdd.error.replace(Some(error.to_string()));
        }
    }
}

#[then("工具注册表含 fixture__echo")]
fn t_has_fixture_echo(infra_plugin_bdd: &InfraPluginBdd) {
    let names = infra_plugin_bdd.tool_names.borrow();
    assert!(
        names.iter().any(|name| name == "fixture__echo"),
        "trusted external plugin tool must be registered: {names:?}"
    );
}

#[then("工具注册表不含 fixture__echo")]
fn t_no_fixture_echo(infra_plugin_bdd: &InfraPluginBdd) {
    let names = infra_plugin_bdd.tool_names.borrow();
    assert!(
        names.iter().all(|name| name != "fixture__echo"),
        "untrusted external plugin must not be assembled: {names:?}"
    );
}

#[then("工具注册表不含 tools-builtin__echo")]
fn t_no_reserved_echo(infra_plugin_bdd: &InfraPluginBdd) {
    let names = infra_plugin_bdd.tool_names.borrow();
    assert!(
        names.iter().all(|name| name != "tools-builtin__echo"),
        "reserved external plugin name must not be registered: {names:?}"
    );
}

#[given("外部插件 fixture 声明 echo 工具")]
fn g_echo_fixture(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    ws.init();
    let script = write_script(ws, "fixture.py", fixture_script());
    infra_plugin_bdd.script_path.replace(Some(script));
}

#[given("外部插件 fixture 声明 protocol_version 99")]
fn g_bad_protocol_fixture(ws: &Workspace, infra_plugin_bdd: &InfraPluginBdd) {
    ws.init();
    let script = write_script(ws, "bad-protocol.py", bad_protocol_script());
    infra_plugin_bdd.script_path.replace(Some(script));
}

fn fixture_config(script: &PathBuf, name: &str) -> ExternalPluginConfig {
    ExternalPluginConfig {
        name: name.into(),
        command: "python3".into(),
        args: vec!["-u".into(), script.to_string_lossy().into_owned()],
        trusted: true,
        timeout_ms: 5_000,
        ..Default::default()
    }
}

#[when("初始化并调用 echo")]
async fn w_init_and_call(infra_plugin_bdd: &InfraPluginBdd) {
    let script = infra_plugin_bdd
        .script_path
        .borrow()
        .clone()
        .expect("script prepared");
    let client = ExternalClient::spawn(&fixture_config(&script, "fixture"))
        .await
        .expect("spawn fixture");
    client.initialize("fixture").await.expect("initialize");
    let result = client
        .request(
            "tools/call",
            serde_json::json!({"name": "echo", "arguments": {"text": "hello"}}),
        )
        .await
        .expect("tools/call");
    infra_plugin_bdd.call_result.replace(Some(
        result["content"].as_str().unwrap_or_default().to_string(),
    ));
    client.shutdown().await;
}

#[then("返回文本 hello")]
fn t_call_result(infra_plugin_bdd: &InfraPluginBdd) {
    assert_eq!(
        infra_plugin_bdd.call_result.borrow().as_deref(),
        Some("hello")
    );
}

#[when("初始化外部插件")]
async fn w_init(infra_plugin_bdd: &InfraPluginBdd) {
    let script = infra_plugin_bdd
        .script_path
        .borrow()
        .clone()
        .expect("script prepared");
    let client = ExternalClient::spawn(&fixture_config(&script, "fixture"))
        .await
        .expect("spawn fixture");
    let error = client
        .initialize("fixture")
        .await
        .expect_err("protocol mismatch must fail");
    infra_plugin_bdd.error.replace(Some(error));
    client.shutdown().await;
}

#[then("握手失败且错误含 protocol mismatch")]
fn t_protocol_mismatch(infra_plugin_bdd: &InfraPluginBdd) {
    let error = infra_plugin_bdd.error.borrow();
    let error = error.as_ref().expect("error captured");
    assert!(error.contains("protocol mismatch"), "{error}");
}

#[then("装配成功且无该插件工具")]
fn t_boot_survives_failure(infra_plugin_bdd: &InfraPluginBdd) {
    assert!(
        infra_plugin_bdd.error.borrow().is_none(),
        "broken external plugin must not fail boot: {:?}",
        infra_plugin_bdd.error.borrow()
    );
    let names = infra_plugin_bdd.tool_names.borrow();
    assert!(
        names
            .iter()
            .all(|name| !name.starts_with("missing-command__")),
        "broken external plugin must not register tools: {names:?}"
    );
}
