use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use rmcp::transport::auth::CredentialStore;
use tokio::sync::Mutex;
use xylitol_plugin_sdk::XyTool;
use xylitol_plugin_sdk::mcp::{McpCatalog, McpCatalogEntry, McpResourceContent};
use xylitol_plugin_sdk::plugin_events::SessionPrepareResult;
use xylitol_plugin_sdk::resource_lease::{
    LeaseState, ResourceKey, ResourceKind, ResourcePool, ResourceScope,
};
use xylitol_plugin_sdk::session_prepare::{SessionPrepareHub, SessionPrepareView};
use xylitol_plugin_sdk::tool_name::mcp_tool_armed_prefix;
use xylitol_plugin_sdk::tool_name::mcp_tool_public_name;

use crate::assemble::{
    adapters_from_discovered, connect_and_discover_at, prompt_adapters_from_discovered,
    resource_adapters_from_discovered,
};
use crate::client::{McpClientManager, McpConnectProgress};
use crate::config::{McpScope, McpServerConfig};

pub type ConnectionKey = ResourceKey;

struct PoolEntry {
    manager: Arc<McpClientManager>,
    name: String,
    transport: String,
    scope: String,
    credential_configured: bool,
    config: McpServerConfig,
    connect_timeout_ms: u64,
    call_timeout_ms: u64,
    claims: usize,
    reconnect_count: usize,
    idle_generation: u64,
    last_used: Instant,
    connected: Vec<(String, usize)>,
    diags: Vec<String>,
    rows: Vec<(String, String, String, serde_json::Value)>,
    rows_prompts: Vec<(String, String, String, serde_json::Value)>,
    rows_resources: Vec<(String, String, String, String)>,
    tools: Vec<Arc<dyn XyTool>>,
}

impl LeaseState for PoolEntry {
    fn ref_count(&self) -> usize {
        self.claims
    }

    fn set_ref_count(&mut self, count: usize) {
        self.claims = count;
    }

    fn idle_generation(&self) -> u64 {
        self.idle_generation
    }

    fn set_idle_generation(&mut self, generation: u64) {
        self.idle_generation = generation;
    }

    fn idle_delay(&self) -> Duration {
        idle_delay(&self.config)
    }

    fn touch(&mut self) {
        self.last_used = Instant::now();
    }
}

fn resource_scope(scope: McpScope) -> ResourceScope {
    match scope {
        McpScope::Shared => ResourceScope::Shared,
        McpScope::Workspace => ResourceScope::Workspace,
        McpScope::Session => ResourceScope::Session,
    }
}

fn idle_delay(config: &McpServerConfig) -> Duration {
    if config.scope == McpScope::Session {
        return Duration::ZERO;
    }
    Duration::from_millis(config.keep_alive_ms.saturating_add(config.idle_ttl_ms))
}

#[derive(Clone, Debug)]
pub struct McpToolDescriptor {
    pub key: ConnectionKey,
    pub server: String,
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
    pub kind: String,
}

struct HubState {
    servers: Mutex<Vec<McpServerConfig>>,
    pool: Mutex<ResourcePool<ConnectionKey, PoolEntry>>,
    claims: Mutex<HashMap<String, Vec<ConnectionKey>>>,
    current: Mutex<Option<(String, Vec<String>)>>,
    last_view: Mutex<Option<SessionPrepareView>>,
    last_result: Mutex<Option<SessionPrepareResult>>,
    reconnects: Mutex<HashMap<ConnectionKey, usize>>,
    last_tools: Mutex<Vec<Arc<dyn XyTool>>>,
    progress: Arc<Mutex<McpConnectProgress>>,
    notice: Mutex<Option<String>>,
}

#[derive(Clone)]
pub struct McpPrepareHub {
    state: Arc<HubState>,
}

fn workspace_fallback() -> Vec<String> {
    std::env::current_dir()
        .ok()
        .map(|path| vec![path.to_string_lossy().into_owned()])
        .unwrap_or_default()
}

fn desired_keys(
    servers: &[McpServerConfig],
    session_id: &str,
    workspaces: &[String],
) -> HashMap<ConnectionKey, Vec<McpServerConfig>> {
    let workspaces = if workspaces.is_empty() {
        workspace_fallback()
    } else {
        workspaces.to_vec()
    };
    let mut out: HashMap<ConnectionKey, Vec<McpServerConfig>> = HashMap::new();
    for server in servers {
        let fingerprint = server.fingerprint();
        let credential = if server.oauth.is_some() {
            Some(server.effective_credential_id())
        } else {
            server.credential.clone()
        };
        match server.scope {
            McpScope::Shared => {
                let key = ConnectionKey {
                    kind: ResourceKind::Mcp,
                    fingerprint,
                    scope: resource_scope(McpScope::Shared),
                    credential: credential.clone(),
                    workspace: None,
                    session: None,
                };
                out.entry(key).or_default().push(server.clone());
            }
            McpScope::Workspace => {
                for workspace in &workspaces {
                    let key = ConnectionKey {
                        kind: ResourceKind::Mcp,
                        fingerprint: fingerprint.clone(),
                        scope: resource_scope(McpScope::Workspace),
                        credential: credential.clone(),
                        workspace: Some(workspace.clone()),
                        session: None,
                    };
                    out.entry(key).or_default().push(server.clone());
                }
            }
            McpScope::Session => {
                let key = ConnectionKey {
                    kind: ResourceKind::Mcp,
                    fingerprint,
                    scope: resource_scope(McpScope::Session),
                    credential: credential.clone(),
                    workspace: None,
                    session: Some(session_id.to_string()),
                };
                out.entry(key).or_default().push(server.clone());
            }
        }
    }
    out
}

impl McpPrepareHub {
    pub fn new(servers: Vec<McpServerConfig>) -> Self {
        let total = servers.len();
        Self {
            state: Arc::new(HubState {
                servers: Mutex::new(servers),
                pool: Mutex::new(ResourcePool::new()),
                claims: Mutex::new(HashMap::new()),
                current: Mutex::new(None),
                last_view: Mutex::new(None),
                last_result: Mutex::new(None),
                last_tools: Mutex::new(Vec::new()),
                progress: Arc::new(Mutex::new(McpConnectProgress {
                    connecting: false,
                    total,
                    finished: 0,
                    current: None,
                })),
                notice: Mutex::new(None),
                reconnects: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn schedule_idle_close(&self, key: ConnectionKey, generation: u64, delay: Duration) {
        if delay.is_zero() {
            return;
        }
        let hub = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            hub.expire_idle(key, generation).await;
        });
    }

    async fn expire_idle(&self, key: ConnectionKey, generation: u64) {
        let manager = {
            let mut pool = self.state.pool.lock().await;
            match pool.get(&key) {
                Some(entry) if entry.ref_count() == 0 && entry.idle_generation() == generation => {
                    pool.remove(&key).map(|entry| entry.manager)
                }
                _ => None,
            }
        };
        let Some(manager) = manager else {
            return;
        };
        *self.state.reconnects.lock().await.entry(key).or_insert(0) += 1;
        manager.shutdown().await;
        self.update_snapshot().await;
    }

    pub async fn ensure_for(
        &self,
        session_id: &str,
        workspaces: &[String],
    ) -> SessionPrepareResult {
        *self.state.current.lock().await = Some((session_id.to_string(), workspaces.to_vec()));
        self.claim_and_connect(session_id, workspaces).await;
        self.refresh_dirty_entries().await;
        self.update_snapshot().await;
        self.state
            .last_result
            .lock()
            .await
            .clone()
            .unwrap_or_default()
    }

    async fn claim_and_connect(&self, session_id: &str, workspaces: &[String]) {
        let servers = self.state.servers.lock().await.clone();
        let desired = desired_keys(&servers, session_id, workspaces);
        let mut shutdowns = Vec::new();
        let mut idle = Vec::new();
        let mut to_connect = Vec::new();
        {
            let mut pool = self.state.pool.lock().await;
            let old = self
                .state
                .claims
                .lock()
                .await
                .remove(session_id)
                .unwrap_or_default();
            for key in old {
                let Some(outcome) = pool.release(&key) else {
                    continue;
                };
                if outcome.delay.is_zero() {
                    let entry = pool.remove(&key).expect("entry exists");
                    *self
                        .state
                        .reconnects
                        .lock()
                        .await
                        .entry(key.clone())
                        .or_insert(0) += 1;
                    shutdowns.push(entry.manager);
                } else {
                    idle.push((key.clone(), outcome.generation, outcome.delay));
                }
            }
            let mut keys = Vec::new();
            for (key, key_servers) in desired {
                keys.push(key.clone());
                match pool.acquire(&key) {
                    Some(_) => {}
                    None => {
                        pool.insert(
                            key.clone(),
                            PoolEntry {
                                manager: Arc::new(McpClientManager::new()),
                                name: key_servers
                                    .first()
                                    .map(|server| server.name.clone())
                                    .unwrap_or_default(),
                                transport: key_servers
                                    .first()
                                    .map(|server| format!("{:?}", server.transport))
                                    .unwrap_or_default(),
                                scope: key_servers
                                    .first()
                                    .map(|server| format!("{:?}", server.scope))
                                    .unwrap_or_default(),
                                credential_configured: key_servers
                                    .first()
                                    .is_some_and(|server| server.credential.is_some()),
                                config: key_servers.first().cloned().unwrap_or_default(),
                                connect_timeout_ms: key_servers
                                    .first()
                                    .map(|server| server.connect_timeout_ms)
                                    .unwrap_or(8_000),
                                call_timeout_ms: key_servers
                                    .first()
                                    .map(|server| server.call_timeout_ms)
                                    .unwrap_or(120_000),
                                claims: 1,
                                reconnect_count: self
                                    .state
                                    .reconnects
                                    .lock()
                                    .await
                                    .get(&key)
                                    .copied()
                                    .unwrap_or(0),
                                idle_generation: 0,
                                last_used: Instant::now(),
                                connected: Vec::new(),
                                diags: Vec::new(),
                                rows: Vec::new(),
                                rows_prompts: Vec::new(),
                                rows_resources: Vec::new(),
                                tools: Vec::new(),
                            },
                        );
                        to_connect.push((key, key_servers));
                    }
                }
            }
            self.state
                .claims
                .lock()
                .await
                .insert(session_id.to_string(), keys);
        }
        for (key, generation, delay) in idle {
            self.schedule_idle_close(key, generation, delay);
        }
        for manager in shutdowns {
            manager.shutdown().await;
        }
        if to_connect.is_empty() {
            return;
        }
        for (key, servers) in to_connect {
            let workspace = key.workspace.clone().map(PathBuf::from);
            let discovered = connect_and_discover_at(
                &servers,
                workspace.as_deref(),
                Some(self.state.progress.clone()),
            )
            .await;
            let Some((manager, _tools)) = discovered else {
                continue;
            };
            let connected = manager
                .connected_servers()
                .await
                .into_iter()
                .map(|server| (server.id, server.tool_count))
                .collect();
            let diags = manager
                .diagnostics()
                .await
                .into_iter()
                .map(|diag| format!("{}: {}", diag.server, diag.message))
                .collect();
            let rows = manager.list_all_tools().await;
            let rows_prompts = manager.list_all_prompts().await;
            let rows_resources = manager.list_all_resources().await;
            let mut tools = adapters_from_discovered(manager.clone(), &rows);
            tools.extend(prompt_adapters_from_discovered(
                manager.clone(),
                &rows_prompts,
            ));
            tools.extend(resource_adapters_from_discovered(
                manager.clone(),
                &rows_resources,
            ));
            let mut pool = self.state.pool.lock().await;
            if let Some(entry) = pool.get_mut(&key) {
                entry.manager = manager;
                entry.connected = connected;
                entry.diags = diags;
                entry.rows = rows;
                entry.rows_prompts = rows_prompts;
                entry.rows_resources = rows_resources;
                entry.tools = tools;
            }
        }
    }

    async fn refresh_dirty_entries(&self) {
        let dirty_keys: Vec<ConnectionKey> = {
            let pool = self.state.pool.lock().await;
            pool.iter()
                .filter(|(_, entry)| entry.manager.tools_dirty())
                .map(|(key, _)| key.clone())
                .collect()
        };
        for key in dirty_keys {
            let manager = {
                let pool = self.state.pool.lock().await;
                pool.get(&key).map(|entry| entry.manager.clone())
            };
            let Some(manager) = manager else {
                continue;
            };
            let rows = manager.list_all_tools().await;
            let rows_prompts = manager.list_all_prompts().await;
            let rows_resources = manager.list_all_resources().await;
            let mut tools = adapters_from_discovered(manager.clone(), &rows);
            tools.extend(prompt_adapters_from_discovered(
                manager.clone(),
                &rows_prompts,
            ));
            tools.extend(resource_adapters_from_discovered(
                manager.clone(),
                &rows_resources,
            ));
            let mut pool = self.state.pool.lock().await;
            if let Some(entry) = pool.get_mut(&key) {
                entry.rows = rows;
                entry.rows_prompts = rows_prompts;
                entry.rows_resources = rows_resources;
                entry.tools = tools;
            }
        }
    }

    pub async fn catalog(&self) -> Vec<McpToolDescriptor> {
        let pool = self.state.pool.lock().await;
        let mut out = Vec::new();
        for (key, entry) in pool.iter() {
            for (server, name, description, schema) in &entry.rows {
                out.push(McpToolDescriptor {
                    key: key.clone(),
                    server: server.clone(),
                    name: mcp_tool_public_name(server, name),
                    description: description.clone(),
                    schema: schema.clone(),
                    kind: "tool".into(),
                });
            }
            for (server, name, description, schema) in &entry.rows_prompts {
                out.push(McpToolDescriptor {
                    key: key.clone(),
                    server: server.clone(),
                    name: mcp_tool_public_name(server, &format!("prompt__{name}")),
                    description: description.clone(),
                    schema: schema.clone(),
                    kind: "prompt".into(),
                });
            }
            for (server, _uri, name, description) in &entry.rows_resources {
                out.push(McpToolDescriptor {
                    key: key.clone(),
                    server: server.clone(),
                    name: mcp_tool_public_name(server, "resource__read"),
                    description: if description.is_empty() {
                        format!("Read MCP resource {name} from {server}")
                    } else {
                        description.clone()
                    },
                    schema: serde_json::json!({
                        "type": "object",
                        "properties": {"uri": {"type": "string"}},
                        "required": ["uri"]
                    }),
                    kind: "resource".into(),
                });
            }
        }
        out
    }

    pub async fn catalog_entries_for_session(&self, session_id: &str) -> McpCatalog {
        let keys: Vec<ConnectionKey> = self
            .state
            .claims
            .lock()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default();
        let pool = self.state.pool.lock().await;
        let mut catalog = McpCatalog::default();
        for (key, entry) in pool.iter() {
            if !keys.contains(key) {
                continue;
            }
            for (server, name, description, schema) in &entry.rows_prompts {
                catalog.prompts.push(McpCatalogEntry {
                    server: server.clone(),
                    kind: "prompt".into(),
                    name: name.clone(),
                    description: description.clone(),
                    uri: None,
                    schema: Some(schema.clone()),
                });
            }
            for (server, uri, name, description) in &entry.rows_resources {
                catalog.resources.push(McpCatalogEntry {
                    server: server.clone(),
                    kind: "resource".into(),
                    name: name.clone(),
                    description: description.clone(),
                    uri: Some(uri.clone()),
                    schema: None,
                });
            }
        }
        catalog
    }

    async fn manager_for_session(
        &self,
        session_id: &str,
        server: &str,
    ) -> Option<Arc<McpClientManager>> {
        let keys: Vec<ConnectionKey> = self
            .state
            .claims
            .lock()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default();
        let pool = self.state.pool.lock().await;
        pool.iter()
            .find(|(key, entry)| keys.contains(key) && entry.name == server)
            .map(|(_, entry)| entry.manager.clone())
    }

    pub async fn read_resource_for_session(
        &self,
        session_id: &str,
        server: &str,
        uri: &str,
    ) -> Result<McpResourceContent, String> {
        let manager = self
            .manager_for_session(session_id, server)
            .await
            .ok_or_else(|| format!("MCP server not found: {server}"))?;
        manager
            .read_resource_cached(server, uri)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn prompt_get_for_session(
        &self,
        session_id: &str,
        server: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let manager = self
            .manager_for_session(session_id, server)
            .await
            .ok_or_else(|| format!("MCP server not found: {server}"))?;
        let arguments = if arguments.is_null() {
            None
        } else {
            Some(arguments)
        };
        manager
            .get_prompt(server, name, arguments)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn catalog_for_session(&self, session_id: &str) -> Vec<McpToolDescriptor> {
        let keys: Vec<ConnectionKey> = self
            .state
            .claims
            .lock()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default();
        self.catalog()
            .await
            .into_iter()
            .filter(|descriptor| keys.contains(&descriptor.key))
            .collect()
    }

    async fn update_snapshot(&self) {
        let pool = self.state.pool.lock().await;
        let mut tools: Vec<Arc<dyn XyTool>> = Vec::new();
        let mut connected = Vec::new();
        let mut diags = Vec::new();
        for entry in pool.values() {
            for tool in &entry.tools {
                if !tools.iter().any(|existing| existing.name() == tool.name()) {
                    tools.push(tool.clone());
                }
            }
            connected.extend(entry.connected.iter().cloned());
            diags.extend(entry.diags.iter().cloned());
        }
        drop(pool);
        let mut names: Vec<String> = tools.iter().map(|tool| tool.name().to_string()).collect();
        names.sort();
        let fingerprint = names.join("\n");
        let env_body = if names.is_empty() {
            String::new()
        } else {
            format!("MCP armed: {}", names.join(", "))
        };
        *self.state.last_result.lock().await = Some(SessionPrepareResult {
            fingerprint,
            env_body,
        });
        *self.state.last_tools.lock().await = tools;
        let view = self.view_now().await;
        *self.state.last_view.lock().await = Some(view);
    }

    async fn view_now(&self) -> SessionPrepareView {
        let servers = self.state.servers.lock().await;
        let claimed_keys: Option<HashSet<ConnectionKey>> = {
            let current = self.state.current.lock().await.clone();
            match current {
                Some((session_id, _)) => Some(
                    self.state
                        .claims
                        .lock()
                        .await
                        .get(&session_id)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                ),
                None => None,
            }
        };
        let pool = self.state.pool.lock().await;
        let progress = self.state.progress.lock().await;
        let server_ids: Vec<String> = servers.iter().map(|server| server.name.clone()).collect();
        let mut connected = Vec::new();
        let mut diags = Vec::new();
        let mut tools: Vec<Arc<dyn XyTool>> = Vec::new();
        let mut server_details = Vec::new();
        let mut probe_lines = Vec::new();
        for (key, entry) in pool.iter() {
            if entry.ref_count() == 0 {
                continue;
            }
            if claimed_keys
                .as_ref()
                .is_some_and(|keys| !keys.contains(key))
            {
                continue;
            }
            connected.extend(entry.connected.iter().cloned());
            diags.extend(entry.diags.iter().cloned());
            tools.extend(entry.tools.iter().cloned());
            probe_lines.extend(entry.manager.probe_lines().await);
            let credential_set = if let Some(oauth) = entry.config.oauth.as_ref() {
                crate::credential_store::OAuthCredentialStore::new(
                    &crate::client::default_agent_dir(),
                    entry.config.effective_credential_id(),
                    oauth.keychain,
                )
                .load()
                .await
                .ok()
                .flatten()
                .is_some()
            } else {
                entry.credential_configured
            };
            server_details.push(format!(
                "server={} transport={} scope={} credential={} connected={} tools={} prompts={} resources={} connect_timeout_ms={} call_timeout_ms={} reconnect_count={} claims={} last_used_ms={} probe={} last_error={}",
                entry.name,
                entry.transport,
                entry.scope,
                if credential_set {
                    "set"
                } else {
                    "none"
                },
                !entry.connected.is_empty(),
                entry.rows.len(),
                entry.rows_prompts.len(),
                entry.rows_resources.len(),
                entry.connect_timeout_ms,
                entry.call_timeout_ms,
                entry.reconnect_count,
                entry.ref_count(),
                entry.last_used.elapsed().as_millis(),
                entry
                    .manager
                    .probe_summary(&entry.name)
                    .await
                    .unwrap_or_else(|| "none".into()),
                if entry.diags.is_empty() {
                    "none".to_string()
                } else {
                    entry.diags.join("; ")
                }
            ));
        }
        let armed = server_ids
            .iter()
            .map(|id| {
                let prefix = mcp_tool_armed_prefix(id);
                let count = tools
                    .iter()
                    .filter(|tool| tool.name().starts_with(&prefix))
                    .count();
                (id.clone(), count)
            })
            .collect();
        SessionPrepareView {
            configured: servers.len(),
            connected,
            diags,
            connecting_label: if progress.connecting {
                progress.connecting_label()
            } else {
                None
            },
            connecting: progress.connecting && !servers.is_empty(),
            complete: servers.is_empty() || (!progress.connecting && !pool.is_empty()),
            server_ids,
            armed,
            server_details,
            probe_lines,
        }
    }

    pub async fn reset_pool(&self) {
        let entries = {
            let mut pool = self.state.pool.lock().await;
            std::mem::take(&mut *pool)
        };
        *self.state.claims.lock().await = HashMap::new();
        *self.state.reconnects.lock().await = HashMap::new();
        *self.state.last_tools.lock().await = Vec::new();
        *self.state.last_result.lock().await = None;
        for entry in entries.into_values() {
            entry.manager.shutdown().await;
        }
    }

    pub fn release_session(&self, session_id: &str) {
        let this = self.clone();
        let session_id = session_id.to_string();
        tokio::spawn(async move {
            this.release_claims(&session_id).await;
        });
    }

    async fn release_claims(&self, session_id: &str) {
        let keys = self
            .state
            .claims
            .lock()
            .await
            .remove(session_id)
            .unwrap_or_default();
        let mut shutdowns = Vec::new();
        let mut idle = Vec::new();
        {
            let mut pool = self.state.pool.lock().await;
            for key in keys {
                let Some(outcome) = pool.release(&key) else {
                    continue;
                };
                if outcome.delay.is_zero() {
                    let entry = pool.remove(&key).expect("entry exists");
                    shutdowns.push(entry.manager);
                } else {
                    idle.push((key.clone(), outcome.generation, outcome.delay));
                }
            }
        }
        for (key, generation, delay) in idle {
            self.schedule_idle_close(key, generation, delay);
        }
        for manager in shutdowns {
            manager.shutdown().await;
        }
        self.update_snapshot().await;
    }

    pub async fn view_async(&self) -> SessionPrepareView {
        self.view_now().await
    }
}

impl McpPrepareHub {
    pub async fn start_oauth(
        &self,
        server: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpOAuthFlow, String> {
        let config = {
            let servers = self.state.servers.lock().await;
            servers
                .iter()
                .find(|config| config.name == server)
                .cloned()
                .ok_or_else(|| format!("MCP server not found: {server}"))?
        };
        let manager = {
            let pool = self.state.pool.lock().await;
            pool.values()
                .find(|entry| entry.name == server)
                .map(|entry| entry.manager.clone())
                .ok_or_else(|| format!("MCP server is not connected: {server}"))?
        };
        manager
            .start_oauth(&config)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn oauth_status(&self, flow_id: &str) -> xylitol_plugin_sdk::mcp::McpOAuthStatus {
        let managers: Vec<_> = {
            let pool = self.state.pool.lock().await;
            pool.values().map(|entry| entry.manager.clone()).collect()
        };
        for manager in managers {
            let status = manager.oauth_status(flow_id).await;
            if status != xylitol_plugin_sdk::mcp::McpOAuthStatus::Unknown {
                return status;
            }
        }
        xylitol_plugin_sdk::mcp::McpOAuthStatus::Unknown
    }

    pub async fn cancel_oauth(&self, flow_id: &str) {
        let managers: Vec<_> = {
            let pool = self.state.pool.lock().await;
            pool.values().map(|entry| entry.manager.clone()).collect()
        };
        for manager in managers {
            manager.cancel_oauth(flow_id).await;
        }
    }
}

#[async_trait]
impl SessionPrepareHub for McpPrepareHub {
    fn cue_connecting(&self) {
        if let Ok(mut progress) = self.state.progress.try_lock() {
            let total = self
                .state
                .servers
                .try_lock()
                .map(|servers| servers.len())
                .unwrap_or(progress.total);
            progress.connecting = total > 0;
            progress.total = total;
            progress.finished = 0;
            progress.current = None;
        }
    }

    fn fire(&self) {
        self.cue_connecting();
        let this = self.clone();
        tokio::spawn(async move {
            let _ = this.ensure().await;
        });
    }

    async fn reset_join(&self) {}

    async fn ensure(&self) -> SessionPrepareResult {
        let (session_id, workspaces) = self
            .state
            .current
            .lock()
            .await
            .clone()
            .unwrap_or_else(|| ("".to_string(), workspace_fallback()));
        self.ensure_for(&session_id, &workspaces).await
    }

    async fn wait(&self, timeout: Duration) -> SessionPrepareResult {
        match tokio::time::timeout(timeout, self.ensure()).await {
            Ok(result) => result,
            Err(_) => {
                *self.state.notice.lock().await =
                    Some(xylitol_plugin_sdk::SESSION_PREPARE_TIMEOUT_NOTICE.into());
                self.state
                    .last_result
                    .lock()
                    .await
                    .clone()
                    .unwrap_or_else(|| self.view().armed_result())
            }
        }
    }

    fn view(&self) -> SessionPrepareView {
        if let Ok(guard) = self.state.last_view.try_lock()
            && let Some(view) = guard.as_ref()
        {
            return view.clone();
        }
        let configured = self
            .state
            .servers
            .try_lock()
            .map(|servers| servers.len())
            .unwrap_or(0);
        SessionPrepareView {
            configured,
            connecting: configured > 0,
            complete: configured == 0,
            ..SessionPrepareView::default()
        }
    }

    fn take_notice(&self) -> Option<String> {
        self.state
            .notice
            .try_lock()
            .ok()
            .and_then(|mut guard| guard.take())
    }

    fn tools(&self) -> Vec<Arc<dyn XyTool>> {
        self.state
            .last_tools
            .try_lock()
            .map(|tools| tools.clone())
            .unwrap_or_default()
    }

    fn release_session(&self, session_id: &str) {
        McpPrepareHub::release_session(self, session_id);
    }

    async fn mcp_catalog_for_session(&self, session_id: &str) -> McpCatalog {
        self.catalog_entries_for_session(session_id).await
    }

    async fn mcp_resource_read(
        &self,
        session_id: &str,
        server: &str,
        uri: &str,
    ) -> Result<McpResourceContent, String> {
        self.read_resource_for_session(session_id, server, uri)
            .await
    }

    async fn mcp_prompt_get(
        &self,
        session_id: &str,
        server: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.prompt_get_for_session(session_id, server, name, arguments)
            .await
    }

    async fn mcp_oauth_start(
        &self,
        server: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpOAuthFlow, String> {
        self.start_oauth(server).await
    }

    async fn mcp_oauth_cancel(&self, flow_id: &str) -> Result<(), String> {
        self.cancel_oauth(flow_id).await;
        Ok(())
    }

    async fn mcp_oauth_status(&self, flow_id: &str) -> xylitol_plugin_sdk::mcp::McpOAuthStatus {
        self.oauth_status(flow_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_cache::parse_resource_content;

    fn server(name: &str, scope: McpScope) -> McpServerConfig {
        McpServerConfig {
            name: name.into(),
            scope,
            command: Some(format!("{name}-bin")),
            ..Default::default()
        }
    }

    #[test]
    fn resource_content_parses_text_and_blob() {
        let text = parse_resource_content(
            "docs",
            "file:///a.md",
            serde_json::json!({
                "contents": [{
                    "uri": "file:///a.md",
                    "mimeType": "text/markdown",
                    "text": "hello"
                }]
            }),
        );
        assert_eq!(text.text, "hello");
        assert_eq!(text.mime.as_deref(), Some("text/markdown"));
        assert!(!text.binary);

        let blob = parse_resource_content(
            "docs",
            "file:///a.bin",
            serde_json::json!({
                "contents": [{
                    "uri": "file:///a.bin",
                    "mimeType": "application/octet-stream",
                    "blob": "AA=="
                }]
            }),
        );
        assert!(blob.binary);
        assert!(blob.text.is_empty());
    }

    #[test]
    fn desired_keys_isolate_credentials() {
        let mut account_a = server("fs", McpScope::Workspace);
        account_a.credential = Some("acct-a".into());
        let mut account_b = server("fs", McpScope::Workspace);
        account_b.credential = Some("acct-b".into());
        let keys = desired_keys(&[account_a, account_b], "s1", &["/a".into()]);
        assert_eq!(keys.len(), 2);
        assert!(
            keys.keys()
                .any(|key| key.credential.as_deref() == Some("acct-a"))
        );
        assert!(
            keys.keys()
                .any(|key| key.credential.as_deref() == Some("acct-b"))
        );
    }

    #[test]
    fn desired_keys_default_oauth_credential_to_server_name() {
        let mut oauth = server("remote", McpScope::Shared);
        oauth.transport = crate::McpTransportKind::Http;
        oauth.url = Some("https://mcp.example.com/mcp".into());
        oauth.oauth = Some(crate::McpOAuthConfig::default());
        let keys = desired_keys(&[oauth], "s1", &["/a".into()]);
        assert_eq!(
            keys.keys().next().and_then(|key| key.credential.as_deref()),
            Some("remote")
        );
    }

    #[test]
    fn desired_keys_isolate_workspace_scope() {
        let servers = vec![server("fs", McpScope::Workspace)];
        let keys = desired_keys(&servers, "s1", &["/a".into(), "/b".into()]);
        assert_eq!(keys.len(), 2);
        assert!(keys.keys().all(|key| key.workspace.is_some()));
    }

    #[test]
    fn desired_keys_share_shared_scope_across_sessions() {
        let servers = vec![server("fs", McpScope::Shared)];
        let left = desired_keys(&servers, "s1", &["/a".into()]);
        let right = desired_keys(&servers, "s2", &["/b".into()]);
        assert_eq!(left.keys().next(), right.keys().next());
    }

    #[test]
    fn desired_keys_isolate_session_scope() {
        let servers = vec![server("fs", McpScope::Session)];
        let left = desired_keys(&servers, "s1", &["/a".into()]);
        let right = desired_keys(&servers, "s2", &["/a".into()]);
        assert_ne!(left.keys().next(), right.keys().next());
    }

    #[tokio::test]
    async fn catalog_empty_without_connections() {
        let hub = McpPrepareHub::new(Vec::new());
        assert!(hub.catalog().await.is_empty());
        assert!(hub.catalog_for_session("s1").await.is_empty());
    }

    #[tokio::test]
    async fn view_keeps_configured_count_when_not_ready() {
        let hub = McpPrepareHub::new(vec![server("fs", McpScope::Workspace)]);
        let view = hub.view();
        assert_eq!(view.configured, 1);
        assert!(view.connecting);
    }

    #[tokio::test]
    async fn failed_server_reconnects_on_next_ensure() {
        let hub = McpPrepareHub::new(vec![server("fs", McpScope::Shared)]);
        hub.ensure_for("s1", &[]).await;
        let first = hub.view_async().await;
        assert!(
            first
                .server_details
                .iter()
                .any(|detail| detail.contains("reconnect_count=0")),
            "{:?}",
            first.server_details
        );
        hub.ensure_for("s1", &[]).await;
        let second = hub.view_async().await;
        assert!(
            second
                .server_details
                .iter()
                .any(|detail| detail.contains("reconnect_count=1")),
            "{:?}",
            second.server_details
        );
    }

    #[tokio::test]
    async fn view_filters_to_current_session() {
        let hub = McpPrepareHub::new(vec![server("fs", McpScope::Workspace)]);
        hub.ensure_for("s1", &["/a".into()]).await;
        let first = hub.view_async().await;
        assert_eq!(first.server_details.len(), 1);
        hub.ensure_for("s2", &["/b".into()]).await;
        let second = hub.view_async().await;
        assert_eq!(
            second.server_details.len(),
            1,
            "old workspace claims must not leak into the current session"
        );
    }

    #[tokio::test]
    async fn idle_grace_keeps_connection_for_reuse_then_closes() {
        let mut config = server("fs", McpScope::Shared);
        config.keep_alive_ms = 50;
        let hub = McpPrepareHub::new(vec![config]);
        hub.ensure_for("s1", &[]).await;
        hub.release_session("s1");
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(
            hub.state.pool.lock().await.len(),
            1,
            "idle grace must keep the entry"
        );
        tokio::time::sleep(Duration::from_millis(90)).await;
        assert_eq!(
            hub.state.pool.lock().await.len(),
            0,
            "idle ttl must close the entry"
        );
    }

    #[tokio::test]
    async fn idle_entry_is_reused_by_next_session() {
        let mut config = server("fs", McpScope::Shared);
        config.keep_alive_ms = 100;
        let hub = McpPrepareHub::new(vec![config]);
        hub.ensure_for("s1", &[]).await;
        hub.release_session("s1");
        tokio::time::sleep(Duration::from_millis(10)).await;
        hub.ensure_for("s2", &[]).await;
        let pool = hub.state.pool.lock().await;
        assert_eq!(pool.len(), 1, "idle entry must be reused");
        assert_eq!(pool.values().next().map(|entry| entry.claims), Some(1));
    }

    #[tokio::test]
    async fn session_scope_release_closes_immediately() {
        let mut config = server("fs", McpScope::Session);
        config.keep_alive_ms = 100;
        let hub = McpPrepareHub::new(vec![config]);
        hub.ensure_for("s1", &[]).await;
        hub.release_session("s1");
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(
            hub.state.pool.lock().await.len(),
            0,
            "session scope must ignore idle grace"
        );
    }

    #[tokio::test]
    async fn idle_delay_sums_keep_alive_and_idle_ttl() {
        let mut config = server("fs", McpScope::Workspace);
        config.keep_alive_ms = 20;
        config.idle_ttl_ms = 40;
        assert_eq!(idle_delay(&config), Duration::from_millis(60));
    }

    #[tokio::test]
    async fn idle_entries_are_not_in_current_session_snapshot() {
        let mut config = server("fs", McpScope::Shared);
        config.keep_alive_ms = 100;
        let hub = McpPrepareHub::new(vec![config]);
        hub.ensure_for("s1", &[]).await;
        hub.release_session("s1");
        tokio::time::sleep(Duration::from_millis(10)).await;
        let view = hub.view_async().await;
        assert!(
            view.server_details.is_empty(),
            "idle entries must not leak into the current session snapshot"
        );
    }

    #[tokio::test]
    async fn release_session_drops_claims() {
        let hub = McpPrepareHub::new(Vec::new());
        hub.release_session("s1");
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(hub.state.claims.lock().await.is_empty());
    }
}
