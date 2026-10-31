use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use xylitol_official_plugin_config::types::ExternalPluginConfig;
use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};
use xylitol_plugin_sdk::tool_name::is_provider_safe_tool_name;
use xylitol_plugin_sdk::{XyTool, XyToolCtx};

pub const EXTERNAL_PLUGIN_PROTOCOL_VERSION: u64 = 1;

pub const EXTERNAL_PLUGIN_TOOL_DELIMITER: &str = "__";

pub fn external_tool_name(plugin: &str, tool: &str) -> String {
    format!("{plugin}{EXTERNAL_PLUGIN_TOOL_DELIMITER}{tool}")
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExternalToolSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExternalCommandSpec {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub argument_hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExternalManifest {
    #[serde(default)]
    pub protocol_version: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub tools: Vec<ExternalToolSpec>,
    #[serde(default)]
    pub commands: Vec<ExternalCommandSpec>,
}

impl ExternalManifest {
    pub fn has_tools(&self) -> bool {
        self.capabilities.iter().any(|cap| cap == "tools")
    }

    pub fn has_commands(&self) -> bool {
        self.capabilities.iter().any(|cap| cap == "commands")
    }
}

struct ClientState {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

pub struct ExternalClient {
    state: Mutex<Option<ClientState>>,
    next_id: AtomicU64,
}

impl ExternalClient {
    pub async fn spawn(config: &ExternalPluginConfig) -> Result<Self, String> {
        let mut command = tokio::process::Command::new(&config.command);
        command
            .args(&config.args)
            .envs(&config.env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true);
        if let Some(cwd) = config.cwd.as_deref() {
            command.current_dir(cwd);
        }
        xylitol_plugin_sdk::host_env::scrub_tokio_command(&mut command);
        let mut child = command
            .spawn()
            .map_err(|e| format!("spawn `{}`: {e}", config.command))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "external plugin stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "external plugin stdout unavailable".to_string())?;
        Ok(Self {
            state: Mutex::new(Some(ClientState {
                child,
                stdin,
                stdout: BufReader::new(stdout),
            })),
            next_id: AtomicU64::new(1),
        })
    }

    pub async fn initialize(&self, plugin: &str) -> Result<ExternalManifest, String> {
        let result = self
            .request(
                "initialize",
                json!({
                    "protocol_version": EXTERNAL_PLUGIN_PROTOCOL_VERSION,
                    "plugin": plugin,
                }),
            )
            .await?;
        let manifest: ExternalManifest = serde_json::from_value(result)
            .map_err(|e| format!("invalid external plugin manifest: {e}"))?;
        if manifest.protocol_version != EXTERNAL_PLUGIN_PROTOCOL_VERSION {
            return Err(format!(
                "external plugin protocol mismatch: expected {}, got {}",
                EXTERNAL_PLUGIN_PROTOCOL_VERSION, manifest.protocol_version
            ));
        }
        Ok(manifest)
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let mut guard = self.state.lock().await;
        let Some(state) = guard.as_mut() else {
            return Err("external plugin is not running".to_string());
        };
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&request)
            .map_err(|e| format!("serialize external plugin request: {e}"))?;
        state
            .stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("write external plugin request: {e}"))?;
        state
            .stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("write external plugin request: {e}"))?;
        state
            .stdin
            .flush()
            .await
            .map_err(|e| format!("flush external plugin request: {e}"))?;

        loop {
            let mut buffer = String::new();
            let read = state
                .stdout
                .read_line(&mut buffer)
                .await
                .map_err(|e| format!("read external plugin response: {e}"))?;
            if read == 0 {
                return Err("external plugin exited before responding".to_string());
            }
            let trimmed = buffer.trim();
            if trimmed.is_empty() {
                continue;
            }
            let response: Value = serde_json::from_str(trimmed)
                .map_err(|e| format!("invalid external plugin response: {e}"))?;
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                return Err(format!("external plugin error: {error}"));
            }
            return response
                .get("result")
                .cloned()
                .ok_or_else(|| "external plugin response missing result".to_string());
        }
    }

    pub async fn shutdown(&self) {
        let mut guard = self.state.lock().await;
        if let Some(mut state) = guard.take() {
            let _ = state.child.start_kill();
        }
    }
}

pub struct ExternalPlugin {
    config: ExternalPluginConfig,
}

impl ExternalPlugin {
    pub fn new(config: &ExternalPluginConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    fn timeout(&self) -> Duration {
        Duration::from_millis(self.config.timeout_ms)
    }
}

impl Plugin for ExternalPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        &self.config.name
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let Some(registries) = ctx.get::<HostRegistries>() else {
            log::warn!(
                "external plugin `{}`: HostRegistries missing; not assembling tools",
                self.config.name
            );
            return Ok(());
        };
        let client = match ExternalClient::spawn(&self.config).await {
            Ok(client) => Arc::new(client),
            Err(error) => {
                log::warn!("external plugin `{}`: {error}", self.config.name);
                return Ok(());
            }
        };
        let manifest = match tokio::time::timeout(
            self.timeout(),
            client.initialize(&self.config.name),
        )
        .await
        {
            Ok(Ok(manifest)) => manifest,
            Ok(Err(error)) => {
                log::warn!("external plugin `{}`: {error}", self.config.name);
                client.shutdown().await;
                return Ok(());
            }
            Err(_) => {
                log::warn!(
                    "external plugin `{}`: initialize timed out after {:?}",
                    self.config.name,
                    self.timeout()
                );
                client.shutdown().await;
                return Ok(());
            }
        };
        if !manifest.has_tools() && !manifest.has_commands() {
            log::warn!(
                "external plugin `{}`: manifest declares neither `tools` nor `commands` capability",
                self.config.name
            );
            client.shutdown().await;
            return Ok(());
        }
        let has_commands = manifest.has_commands();
        let command_description = manifest
            .commands
            .first()
            .map(|c| c.description.clone())
            .unwrap_or_default();
        let mut registered = Vec::new();
        for spec in manifest.tools {
            let public_name = external_tool_name(&self.config.name, &spec.name);
            if !is_provider_safe_tool_name(&public_name) {
                log::warn!(
                    "external plugin `{}`: tool `{}` is not provider-safe; skipped",
                    self.config.name,
                    spec.name
                );
                continue;
            }
            if registries.tools.get(&public_name).is_some() {
                log::warn!(
                    "external plugin `{}`: tool `{}` already registered; skipped",
                    self.config.name,
                    public_name
                );
                continue;
            }
            registries.tools.insert(
                public_name.clone(),
                Arc::new(ExternalTool {
                    public_name: public_name.clone(),
                    spec,
                    client: Arc::clone(&client),
                    timeout: self.timeout(),
                }),
            );
            registered.push(public_name);
        }
        let mut registered_commands = Vec::new();
        if has_commands {
            let public_name = format!("plugin:{}", self.config.name);
            registries.plugin_commands.insert(
                public_name.clone(),
                Arc::new(ExternalPluginCommand {
                    public_name: public_name.clone(),
                    description: command_description,
                    client: Arc::clone(&client),
                    timeout: self.timeout(),
                }),
            );
            registered_commands.push(public_name);
        }
        let cleanup_registries = Arc::clone(&registries);
        ctx.effect(move || async move {
            for name in registered {
                cleanup_registries.tools.remove(&name);
            }
            for name in registered_commands {
                cleanup_registries.plugin_commands.remove(&name);
            }
            client.shutdown().await;
            Ok(())
        })?;
        Ok(())
    }
}

struct ExternalPluginCommand {
    public_name: String,
    description: String,
    client: Arc<ExternalClient>,
    timeout: Duration,
}

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyPluginCommand for ExternalPluginCommand {
    fn name(&self) -> &str {
        &self.public_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn invoke(&self, args: &str) -> Result<String, String> {
        let result = tokio::time::timeout(
            self.timeout,
            self.client
                .request("commands/call", json!({ "args": args })),
        )
        .await
        .map_err(|_| {
            format!(
                "external plugin command `{}` timed out after {:?}",
                self.public_name, self.timeout
            )
        })??;
        Ok(result
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }
}

struct ExternalTool {
    public_name: String,
    spec: ExternalToolSpec,
    client: Arc<ExternalClient>,
    timeout: Duration,
}

#[async_trait]
impl XyTool for ExternalTool {
    fn name(&self) -> &str {
        &self.public_name
    }

    fn description(&self) -> &str {
        &self.spec.description
    }

    fn parameters_schema(&self) -> Value {
        if self.spec.input_schema.is_null() {
            json!({ "type": "object", "properties": {} })
        } else {
            self.spec.input_schema.clone()
        }
    }

    async fn execute(&self, _ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let call = self.client.request(
            "tools/call",
            json!({
                "name": self.spec.name,
                "arguments": args,
            }),
        );
        match tokio::time::timeout(self.timeout, call).await {
            Ok(Ok(result)) => Ok(result
                .get("content")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| result.to_string())),
            Ok(Err(error)) => Err(XyToolError::ExecutionFailed(anyhow::anyhow!(error))),
            Err(_) => {
                self.client.shutdown().await;
                Err(XyToolError::Timeout(self.timeout))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn python_fixture() -> String {
        r#"
import json, sys

TOOLS = [{
    "name": "echo",
    "description": "Echo text back",
    "input_schema": {"type": "object", "properties": {"text": {"type": "string"}}},
}]

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    method = request.get("method")
    if method == "initialize":
        result = {
            "protocol_version": 1,
            "name": "fixture",
            "version": "0.1.0",
            "capabilities": ["tools"],
            "tools": TOOLS,
        }
    elif method == "tools/call":
        params = request.get("params", {})
        if params.get("name") == "echo":
            result = {"content": str(params.get("arguments", {}).get("text", ""))}
        else:
            print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32601, "message": "unknown tool"}}), flush=True)
            continue
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32601, "message": "unknown method"}}), flush=True)
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#
        .to_string()
    }

    fn fixture_config(script: &std::path::Path) -> ExternalPluginConfig {
        ExternalPluginConfig {
            name: "fixture".into(),
            command: "python3".into(),
            args: vec!["-u".into(), script.to_string_lossy().into_owned()],
            trusted: true,
            timeout_ms: 5_000,
            ..Default::default()
        }
    }

    fn write_fixture(dir: &tempfile::TempDir) -> std::path::PathBuf {
        let path = dir.path().join("external_plugin.py");
        let mut file = std::fs::File::create(&path).expect("fixture file");
        file.write_all(python_fixture().as_bytes())
            .expect("write fixture");
        path
    }

    #[tokio::test]
    async fn client_initializes_and_calls_tool() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = write_fixture(&dir);
        let client = ExternalClient::spawn(&fixture_config(&script))
            .await
            .expect("spawn fixture");
        let manifest = client.initialize("fixture").await.expect("initialize");
        assert_eq!(manifest.protocol_version, EXTERNAL_PLUGIN_PROTOCOL_VERSION);
        assert!(manifest.has_tools());
        assert_eq!(manifest.tools.len(), 1);
        let result = client
            .request(
                "tools/call",
                json!({"name": "echo", "arguments": {"text": "hello"}}),
            )
            .await
            .expect("tools/call");
        assert_eq!(result["content"], "hello");
        client.shutdown().await;
    }

    #[tokio::test]
    async fn tool_names_are_namespaced_and_provider_safe() {
        assert_eq!(external_tool_name("bun-demo", "echo"), "bun-demo__echo");
        assert!(is_provider_safe_tool_name(&external_tool_name(
            "bun-demo", "echo"
        )));
        assert!(!is_provider_safe_tool_name(&external_tool_name(
            "bun demo", "echo"
        )));
    }

    #[tokio::test]
    #[ignore = "requires bun on PATH; run locally with --ignored"]
    async fn bun_demo_plugin_roundtrip() {
        let plugin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/plugins/bun-demo/plugin.ts");
        let plugin = plugin.canonicalize().expect("bun demo plugin path");
        let config = ExternalPluginConfig {
            name: "bun-demo".into(),
            command: "bun".into(),
            args: vec![plugin.to_string_lossy().into_owned()],
            trusted: true,
            timeout_ms: 5_000,
            ..Default::default()
        };
        let client = ExternalClient::spawn(&config)
            .await
            .expect("spawn bun demo");
        let manifest = client.initialize("bun-demo").await.expect("initialize");
        assert_eq!(manifest.name, "bun-demo");
        assert_eq!(manifest.tools.len(), 2);
        let result = client
            .request(
                "tools/call",
                json!({"name": "add", "arguments": {"a": 2, "b": 40}}),
            )
            .await
            .expect("tools/call");
        assert_eq!(result["content"], "42");
        client.shutdown().await;
    }

    #[tokio::test]
    async fn protocol_mismatch_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bad_protocol.py");
        std::fs::write(
            &path,
            r#"
import json, sys
for line in sys.stdin:
    request = json.loads(line)
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": {"protocol_version": 99, "capabilities": ["tools"]}}), flush=True)
"#,
        )
        .expect("write fixture");
        let client = ExternalClient::spawn(&fixture_config(&path))
            .await
            .expect("spawn fixture");
        let error = client.initialize("fixture").await.expect_err("mismatch");
        assert!(error.contains("protocol mismatch"), "{error}");
        client.shutdown().await;
    }
}
