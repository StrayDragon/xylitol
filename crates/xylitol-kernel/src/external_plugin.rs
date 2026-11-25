use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use xylitol_official_plugin_config::types::ExternalPluginConfig;
use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub const EXTERNAL_PLUGIN_PROTOCOL_VERSION: u64 = 1;

pub const EXTERNAL_PLUGIN_DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
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
    #[serde(default)]
    pub mcp: Option<ExternalMcpSpec>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExternalMcpSpec {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

impl ExternalManifest {
    pub fn has_tools(&self) -> bool {
        self.capabilities.iter().any(|cap| cap == "tools")
    }

    pub fn has_commands(&self) -> bool {
        self.capabilities.iter().any(|cap| cap == "commands")
    }

    pub fn config_schema(&self) -> Option<Value> {
        self.config_schema.clone()
    }

    pub fn first_command_description(&self) -> String {
        self.commands
            .first()
            .map(|command| command.description.clone())
            .unwrap_or_default()
    }

    pub fn has_mcp(&self) -> bool {
        self.mcp.is_some()
    }

    pub fn mcp(&self) -> Option<ExternalMcpSpec> {
        self.mcp.clone()
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
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Self, String> {
        let mut child_command = tokio::process::Command::new(command);
        child_command
            .args(args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true);
        xylitol_plugin_sdk::host_env::scrub_tokio_command(&mut child_command);
        let mut child = child_command
            .spawn()
            .map_err(|e| format!("spawn `{command}`: {e}"))?;
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
    name: String,
    timeout: Duration,
    bin_dir_override: Option<std::path::PathBuf>,
}

impl ExternalPlugin {
    pub fn new(config: &ExternalPluginConfig) -> Result<Self> {
        Ok(Self {
            name: config.name.clone(),
            timeout: EXTERNAL_PLUGIN_DEFAULT_TIMEOUT,
            bin_dir_override: None,
        })
    }

    pub fn discovered_at(name: &str, bin_dir: &std::path::Path) -> Self {
        Self {
            name: name.to_string(),
            timeout: EXTERNAL_PLUGIN_DEFAULT_TIMEOUT,
            bin_dir_override: Some(bin_dir.to_path_buf()),
        }
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn launch_command(
        &self,
    ) -> Option<(
        String,
        Vec<String>,
        std::collections::HashMap<String, String>,
    )> {
        let bin_dir = self.bin_dir_override.clone().or_else(plugin_bin_dir)?;
        let executable = bin_dir.join(&self.name);
        if !executable.is_file() {
            return None;
        }
        Some((
            executable.to_string_lossy().into_owned(),
            Vec::new(),
            std::collections::HashMap::new(),
        ))
    }
}

impl Plugin for ExternalPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        &self.name
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let Some(registries) = ctx.get::<HostRegistries>() else {
            log::warn!(
                "external plugin `{}`: HostRegistries missing; not assembling tools",
                self.name
            );
            return Ok(());
        };
        let Some((command, args, env)) = self.launch_command() else {
            log::warn!(
                "external plugin `{}`: executable missing under plugins/bin; skipped",
                self.name
            );
            return Ok(());
        };
        let client = match ExternalClient::spawn(&command, &args, &env).await {
            Ok(client) => Arc::new(client),
            Err(error) => {
                log::warn!("external plugin `{}`: {error}", self.name);
                return Ok(());
            }
        };
        let manifest =
            match tokio::time::timeout(self.timeout(), client.initialize(&self.name)).await {
                Ok(Ok(manifest)) => manifest,
                Ok(Err(error)) => {
                    log::warn!("external plugin `{}`: {error}", self.name);
                    client.shutdown().await;
                    return Ok(());
                }
                Err(_) => {
                    log::warn!(
                        "external plugin `{}`: initialize timed out after {:?}",
                        self.name,
                        self.timeout()
                    );
                    client.shutdown().await;
                    return Ok(());
                }
            };
        let has_commands = manifest.has_commands();
        if !has_commands && !manifest.has_mcp() {
            log::warn!(
                "external plugin `{}`: manifest declares neither `commands` capability nor `mcp` declaration",
                self.name
            );
            client.shutdown().await;
            return Ok(());
        }
        if let (Some(config_key), Some(config_schema)) =
            (manifest.config_key.as_deref(), manifest.config_schema())
        {
            let Some(config_service) =
                ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>()
            else {
                log::error!(
                    "external plugin `{}`: ConfigService missing; cannot validate config section `{config_key}`",
                    self.name
                );
                client.shutdown().await;
                return Ok(());
            };
            if let Some(value) = config_service.app.sections.get(config_key) {
                match jsonschema::Validator::new(&config_schema) {
                    Ok(validator) => {
                        if let Err(error) = validator.validate(value) {
                            log::error!(
                                "external plugin `{}`: config section `{config_key}` violates config_schema: {error}",
                                self.name
                            );
                            client.shutdown().await;
                            return Ok(());
                        }
                    }
                    Err(error) => {
                        log::error!(
                            "external plugin `{}`: invalid config_schema for `{config_key}`: {error}",
                            self.name
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
                    self.name
                );
                client.shutdown().await;
                return Ok(());
            }
        }
        let command_description = manifest.first_command_description();
        let mut registered_commands = Vec::new();
        if has_commands {
            let public_name = format!("plugin:{}", self.name);
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
            for name in registered_commands {
                cleanup_registries.plugin_commands.remove(&name);
            }
            client.shutdown().await;
            Ok(())
        })?;
        if let Some(mcp) = manifest.mcp() {
            if mcp.command.trim().is_empty() {
                log::warn!(
                    "external plugin `{}`: mcp declaration has empty command; not registered",
                    self.name
                );
                return Ok(());
            }
            if let Some(registry) = ctx.get::<ExternalMcpRegistry>() {
                registry.register(mcp_server_from_declaration(&self.name, mcp));
            } else {
                log::warn!(
                    "external plugin `{}`: ExternalMcpRegistry missing; mcp declaration not registered",
                    self.name
                );
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct ExternalMcpRegistry {
    declared: std::sync::Mutex<Vec<xylitol_official_plugin_mcp::McpServerConfig>>,
    approved: std::sync::Mutex<Vec<String>>,
}

impl ExternalMcpRegistry {
    pub fn register(&self, server: xylitol_official_plugin_mcp::McpServerConfig) {
        let mut guard = self
            .declared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.retain(|existing| existing.name != server.name);
        guard.push(server);
    }

    pub fn declared(&self) -> Vec<xylitol_official_plugin_mcp::McpServerConfig> {
        self.declared
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn approve(&self, name: &str) {
        let mut guard = self
            .approved
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !guard.iter().any(|approved| approved == name) {
            guard.push(name.to_string());
        }
    }

    pub fn is_approved(&self, name: &str) -> bool {
        self.approved
            .lock()
            .map(|guard| guard.iter().any(|approved| approved == name))
            .unwrap_or(false)
    }

    pub fn approved_names(&self) -> Vec<String> {
        self.approved
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn servers(&self) -> Vec<xylitol_official_plugin_mcp::McpServerConfig> {
        let approved = self.approved_names();
        self.declared()
            .into_iter()
            .filter(|server| approved.iter().any(|name| name == &server.name))
            .collect()
    }
}

pub fn plugin_bin_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("XYLITOL_PLUGINS_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                std::path::PathBuf::from(home)
                    .join(".xylitol")
                    .join("plugins")
                    .join("bin")
            })
        })
}

pub fn approval_store_path() -> Option<std::path::PathBuf> {
    plugin_bin_dir().and_then(|bin| bin.parent().map(|root| root.join("approved.json")))
}

pub fn load_approved_plugins(path: &std::path::Path) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        log::warn!(
            "plugin approval store `{}` is not valid JSON; ignoring it",
            path.display()
        );
        return Vec::new();
    };
    value
        .get("approved")
        .and_then(serde_json::Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn persist_approved_plugins(path: &std::path::Path, names: &[String]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = serde_json::json!({ "version": 1, "approved": names });
    std::fs::write(path, format!("{payload}\n"))
}

fn mcp_server_from_declaration(
    name: &str,
    mcp: ExternalMcpSpec,
) -> xylitol_official_plugin_mcp::McpServerConfig {
    xylitol_official_plugin_mcp::McpServerConfig {
        name: name.to_string(),
        transport: xylitol_official_plugin_mcp::McpTransportKind::Stdio,
        command: Some(mcp.command),
        args: Some(mcp.args),
        url: None,
        env: (!mcp.env.is_empty()).then_some(mcp.env),
        headers: None,
        scope: xylitol_official_plugin_mcp::McpScope::default(),
        credential: None,
        connect_timeout_ms: 30_000,
        call_timeout_ms: 60_000,
        keep_alive_ms: 0,
        idle_ttl_ms: 0,
        custom_transport: None,
        oauth: None,
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

#[cfg(test)]
mod approval_tests {
    use super::*;
    use std::path::PathBuf;

    fn stdio(name: &str) -> xylitol_official_plugin_mcp::McpServerConfig {
        mcp_server_from_declaration(
            name,
            ExternalMcpSpec {
                command: format!("{name}-server"),
                args: vec!["--stdio".into()],
                env: Default::default(),
            },
        )
    }

    #[test]
    fn declared_but_unapproved_server_is_not_served() {
        let registry = ExternalMcpRegistry::default();
        registry.register(stdio("demo"));
        assert_eq!(registry.declared().len(), 1, "the declaration is recorded");
        assert!(
            registry.servers().is_empty(),
            "an unapproved declaration must not reach the MCP pipeline"
        );
        assert!(!registry.is_approved("demo"));
    }

    #[test]
    fn approved_server_is_served() {
        let registry = ExternalMcpRegistry::default();
        registry.register(stdio("demo"));
        registry.approve("demo");
        let servers = registry.servers();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "demo");
        assert_eq!(servers[0].command.as_deref(), Some("demo-server"));
    }

    #[test]
    fn approval_round_trips_through_the_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("approved.json");
        assert!(
            load_approved_plugins(&path).is_empty(),
            "a missing store approves nothing"
        );
        persist_approved_plugins(&path, &["demo".to_string()]).expect("persist approvals");
        assert_eq!(load_approved_plugins(&path), vec!["demo".to_string()]);
        std::fs::write(&path, "not json").expect("corrupt store");
        assert!(
            load_approved_plugins(&path).is_empty(),
            "a corrupt store must fail closed"
        );
    }

    #[test]
    fn approval_store_lives_beside_the_plugin_bin_dir() {
        let bin = PathBuf::from("/home/u/.xylitol/plugins/bin");
        assert_eq!(
            bin.parent().map(|root| root.join("approved.json")),
            Some(PathBuf::from("/home/u/.xylitol/plugins/approved.json"))
        );
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

    fn fixture_spawn(
        script: &std::path::Path,
    ) -> (
        String,
        Vec<String>,
        std::collections::HashMap<String, String>,
    ) {
        (
            "python3".to_string(),
            vec!["-u".to_string(), script.to_string_lossy().into_owned()],
            std::collections::HashMap::new(),
        )
    }

    async fn spawn_fixture(script: &std::path::Path) -> Result<ExternalClient, String> {
        let (command, args, env) = fixture_spawn(script);
        ExternalClient::spawn(&command, &args, &env).await
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
        let client = spawn_fixture(&script).await.expect("spawn fixture");
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
        let client = spawn_fixture(&script).await.expect("spawn desync fixture");
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

    #[test]
    fn mcp_declaration_maps_to_stdio_server() {
        let manifest: ExternalManifest = serde_json::from_value(json!({
            "protocol_version": EXTERNAL_PLUGIN_PROTOCOL_VERSION,
            "name": "demo",
            "capabilities": ["commands"],
            "commands": [{"name": "run", "description": "run it"}],
            "mcp": {"command": "demo-server", "args": ["--stdio"], "env": {"A": "1"}}
        }))
        .expect("manifest with mcp declaration");
        assert!(!manifest.has_tools());
        assert!(manifest.has_commands());
        let mcp = manifest.mcp.expect("mcp declaration parsed");
        let server = mcp_server_from_declaration("demo", mcp);
        assert_eq!(server.name, "demo");
        assert_eq!(
            server.transport,
            xylitol_official_plugin_mcp::McpTransportKind::Stdio
        );
        assert_eq!(server.command.as_deref(), Some("demo-server"));
        assert_eq!(
            server.args.as_deref(),
            Some(["--stdio".to_string()].as_slice())
        );
        assert_eq!(
            server.env.as_ref().and_then(|e| e.get("A")),
            Some(&"1".to_string())
        );
        server.validate().expect("registered server must validate");
    }

    #[test]
    fn manifest_without_mcp_declaration_registers_no_server() {
        let manifest: ExternalManifest = serde_json::from_value(json!({
            "protocol_version": EXTERNAL_PLUGIN_PROTOCOL_VERSION,
            "name": "legacy",
            "capabilities": ["tools"],
            "tools": [{"name": "echo"}]
        }))
        .expect("manifest");
        assert!(
            manifest.mcp.is_none(),
            "a plugin without an mcp declaration must not register as an MCP server"
        );
    }

    #[test]
    fn external_mcp_registry_replaces_by_name() {
        let registry = ExternalMcpRegistry::default();
        registry.register(mcp_server_from_declaration(
            "demo",
            ExternalMcpSpec {
                command: "first".into(),
                args: Vec::new(),
                env: Default::default(),
            },
        ));
        registry.register(mcp_server_from_declaration(
            "demo",
            ExternalMcpSpec {
                command: "second".into(),
                args: Vec::new(),
                env: Default::default(),
            },
        ));
        let declared = registry.declared();
        assert_eq!(declared.len(), 1);
        assert_eq!(declared[0].command.as_deref(), Some("second"));
        assert!(
            registry.servers().is_empty(),
            "an unapproved declaration stays out of the served set"
        );
        registry.approve("demo");
        let servers = registry.servers();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].command.as_deref(), Some("second"));
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
        let client = spawn_fixture(&path).await.expect("spawn fixture");
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
        let client = spawn_fixture(&path).await.expect("spawn fixture");
        let error = client.initialize("fixture").await.expect_err("mismatch");
        assert!(error.contains("protocol mismatch"), "{error}");
        client.shutdown().await;
    }
}
