use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
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

#[derive(Debug, Clone, Deserialize, PartialEq)]
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
    #[serde(default)]
    pub config_key: Option<String>,
    #[serde(default)]
    pub config_schema: Option<Value>,
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

const EXTERNAL_PLUGIN_DESYNC_LIMIT: u32 = 2;

pub struct ExternalClient {
    state: Mutex<Option<ClientState>>,
    dead_reason: Mutex<Option<String>>,
    next_id: AtomicU64,
    desync_streak: AtomicU32,
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
            dead_reason: Mutex::new(None),
            next_id: AtomicU64::new(1),
            desync_streak: AtomicU32::new(0),
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
            let reason = self.dead_reason.lock().await.clone();
            return Err(match reason {
                Some(reason) => format!("external plugin unavailable: {reason}"),
                None => "external plugin is not running".to_string(),
            });
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
        let written = async {
            state.stdin.write_all(line.as_bytes()).await?;
            state.stdin.write_all(b"\n").await?;
            state.stdin.flush().await
        }
        .await;
        if let Err(e) = written {
            guard.take();
            let reason = format!("write failed: {e}");
            *self.dead_reason.lock().await = Some(reason.clone());
            return Err(format!("external plugin unavailable: {reason}"));
        }

        loop {
            let mut buffer = String::new();
            let read = state.stdout.read_line(&mut buffer).await;
            match read {
                Ok(0) => {
                    guard.take();
                    let reason = format!("process exited before responding to {method}");
                    *self.dead_reason.lock().await = Some(reason.clone());
                    return Err(format!("external plugin unavailable: {reason}"));
                }
                Ok(_) => {}
                Err(e) => {
                    guard.take();
                    let reason = format!("read failed: {e}");
                    *self.dead_reason.lock().await = Some(reason.clone());
                    return Err(format!("external plugin unavailable: {reason}"));
                }
            }
            let trimmed = buffer.trim();
            if trimmed.is_empty() {
                continue;
            }
            let response: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    if let Some(unavailable) = self.note_desync(&mut guard).await {
                        return Err(unavailable);
                    }
                    return Err(format!("invalid external plugin response: {e}"));
                }
            };
            let response_id = response.get("id").and_then(Value::as_u64);
            if response_id.is_none() {
                continue;
            }
            if response_id != Some(id) {
                if let Some(unavailable) = self.note_desync(&mut guard).await {
                    return Err(unavailable);
                }
                return Err("external plugin response stream desynced".to_string());
            }
            self.desync_streak.store(0, Ordering::SeqCst);
            if let Some(error) = response.get("error") {
                return Err(format!("external plugin error: {error}"));
            }
            return response
                .get("result")
                .cloned()
                .ok_or_else(|| "external plugin response missing result".to_string());
        }
    }

    async fn note_desync(&self, guard: &mut Option<ClientState>) -> Option<String> {
        let streak = self.desync_streak.fetch_add(1, Ordering::SeqCst) + 1;
        if streak < EXTERNAL_PLUGIN_DESYNC_LIMIT {
            return None;
        }
        guard.take();
        let reason =
            format!("response stream desynced after {streak} malformed or mismatched lines");
        *self.dead_reason.lock().await = Some(reason.clone());
        Some(format!("external plugin unavailable: {reason}"))
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
        if let (Some(config_key), Some(config_schema)) = (
            manifest.config_key.as_deref(),
            manifest.config_schema.as_ref(),
        ) {
            let Some(config_service) =
                ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>()
            else {
                log::error!(
                    "external plugin `{}`: ConfigService missing; cannot validate config section `{config_key}`",
                    self.config.name
                );
                client.shutdown().await;
                return Ok(());
            };
            if let Some(value) = config_service.app.sections.get(config_key) {
                match jsonschema::Validator::new(config_schema) {
                    Ok(validator) => {
                        if let Err(error) = validator.validate(value) {
                            log::error!(
                                "external plugin `{}`: config section `{config_key}` violates config_schema: {error}",
                                self.config.name
                            );
                            client.shutdown().await;
                            return Ok(());
                        }
                    }
                    Err(error) => {
                        log::error!(
                            "external plugin `{}`: invalid config_schema for `{config_key}`: {error}",
                            self.config.name
                        );
                        client.shutdown().await;
                        return Ok(());
                    }
                }
            }
            let duplicate = {
                let mut keys = config_service
                    .external_config_keys
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if keys.iter().any(|key| key == config_key) {
                    true
                } else {
                    keys.push(config_key.to_string());
                    false
                }
            };
            if duplicate {
                log::error!(
                    "external plugin `{}`: config_key `{config_key}` already claimed by another plugin",
                    self.config.name
                );
                client.shutdown().await;
                return Ok(());
            }
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

    async fn execute_outcome(
        &self,
        _ctx: &XyToolCtx,
        args: Value,
    ) -> Result<xylitol_plugin_sdk::XyToolOutcome, XyToolError> {
        let call = self.client.request(
            "tools/call",
            json!({
                "name": self.spec.name,
                "arguments": args,
            }),
        );
        match tokio::time::timeout(self.timeout, call).await {
            Ok(Ok(result)) => Ok(xylitol_plugin_sdk::XyToolOutcome::text(
                result
                    .get("content")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| result.to_string()),
            )),
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

    fn desync_fixture() -> String {
        r#"
import json, sys

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    method = request.get("method")
    if method == "initialize":
        result = {
            "protocol_version": 1,
            "name": "desync-fixture",
            "version": "0.1.0",
            "capabilities": [],
            "tools": [],
        }
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
    elif method == "tools/call":
        name = request.get("params", {}).get("name", "")
        if name == "garbage1":
            print("this is not json at all", flush=True)
        elif name == "garbage2":
            print(json.dumps({"jsonrpc": "2.0", "id": request["id"] + 100, "result": {}}), flush=True)
        else:
            print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": {}}), flush=True)
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
    async fn desync_streak_marks_client_dead_after_two_corrupt_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("desync_plugin.py");
        let mut file = std::fs::File::create(&script).expect("fixture file");
        file.write_all(desync_fixture().as_bytes())
            .expect("write fixture");
        let client = ExternalClient::spawn(&fixture_config(&script))
            .await
            .expect("spawn desync fixture");
        client
            .initialize("desync-fixture")
            .await
            .expect("initialize");

        let first = client
            .request("tools/call", json!({"name": "garbage1"}))
            .await;
        assert!(
            first
                .as_ref()
                .err()
                .is_some_and(|e| e.contains("invalid external plugin response")),
            "first malformed line must fail the request without killing the client: {first:?}"
        );

        let second = client
            .request("tools/call", json!({"name": "garbage2"}))
            .await;
        assert!(
            second
                .as_ref()
                .err()
                .is_some_and(|e| e.contains("response stream desynced")),
            "the second strike must mark the client dead: {second:?}"
        );

        let third = client
            .request("tools/call", json!({"name": "anything"}))
            .await;
        assert!(
            third
                .as_ref()
                .err()
                .is_some_and(|e| e.contains("external plugin unavailable")),
            "a dead client must fail fast: {third:?}"
        );
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
    async fn request_after_process_death_reports_unavailable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dying.py");
        std::fs::write(
            &path,
            r#"
import json, sys
line = sys.stdin.readline()
request = json.loads(line)
print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": {"protocol_version": 1, "capabilities": ["tools"], "tools": []}}), flush=True)
"#,
        )
        .expect("write fixture");
        let client = ExternalClient::spawn(&fixture_config(&path))
            .await
            .expect("spawn fixture");
        client.initialize("fixture").await.expect("initialize");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let error = client
            .request("tools/call", json!({"name": "echo"}))
            .await
            .expect_err("dead client must fail");
        assert!(error.contains("unavailable"), "{error}");
        assert!(
            error.contains("process exited") || error.contains("write failed"),
            "unavailable must carry the death cause: {error}"
        );
        let again = client
            .request("tools/call", json!({"name": "echo"}))
            .await
            .expect_err("dead client must stay unavailable");
        assert!(again.contains("unavailable"), "{again}");
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
