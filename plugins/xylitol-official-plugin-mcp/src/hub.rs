use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;
use xylitol_plugin_sdk::XyTool;
use xylitol_plugin_sdk::plugin_events::SessionPrepareResult;
use xylitol_plugin_sdk::session_prepare::{SessionPrepareHub, SessionPrepareView};
use xylitol_plugin_sdk::tool_name::mcp_tool_armed_prefix;
use xylitol_plugin_sdk::tool_name::mcp_tool_public_name;

use crate::assemble::{
    adapters_from_discovered, connect_and_discover_at, prompt_adapters_from_discovered,
    resource_adapters_from_discovered,
};
use crate::client::{McpClientManager, McpConnectProgress};
use crate::config::{McpScope, McpServerConfig};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionKey {
    pub fingerprint: String,
    pub scope: McpScope,
    pub credential: Option<String>,
    pub workspace: Option<String>,
    pub session: Option<String>,
}

struct PoolEntry {
    manager: Arc<McpClientManager>,
    claims: usize,
    connected: Vec<(String, usize)>,
    diags: Vec<String>,
    rows: Vec<(String, String, String, serde_json::Value)>,
    rows_prompts: Vec<(String, String, String, serde_json::Value)>,
    rows_resources: Vec<(String, String, String, String)>,
    tools: Vec<Arc<dyn XyTool>>,
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
    pool: Mutex<HashMap<ConnectionKey, PoolEntry>>,
    claims: Mutex<HashMap<String, Vec<ConnectionKey>>>,
    current: Mutex<Option<(String, Vec<String>)>>,
    last_view: Mutex<Option<SessionPrepareView>>,
    last_result: Mutex<Option<SessionPrepareResult>>,
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
        match server.scope {
            McpScope::Shared => {
                let key = ConnectionKey {
                    fingerprint,
                    scope: McpScope::Shared,
                    credential: server.credential.clone(),
                    workspace: None,
                    session: None,
                };
                out.entry(key).or_default().push(server.clone());
            }
            McpScope::Workspace => {
                for workspace in &workspaces {
                    let key = ConnectionKey {
                        fingerprint: fingerprint.clone(),
                        scope: McpScope::Workspace,
                        credential: server.credential.clone(),
                        workspace: Some(workspace.clone()),
                        session: None,
                    };
                    out.entry(key).or_default().push(server.clone());
                }
            }
            McpScope::Session => {
                let key = ConnectionKey {
                    fingerprint,
                    scope: McpScope::Session,
                    credential: server.credential.clone(),
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
                pool: Mutex::new(HashMap::new()),
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
            }),
        }
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
                let remove = if let Some(entry) = pool.get_mut(&key) {
                    entry.claims = entry.claims.saturating_sub(1);
                    entry.claims == 0
                } else {
                    false
                };
                if remove && let Some(entry) = pool.remove(&key) {
                    shutdowns.push(entry.manager);
                }
            }
            let mut keys = Vec::new();
            for (key, key_servers) in desired {
                keys.push(key.clone());
                match pool.get_mut(&key) {
                    Some(entry) => entry.claims += 1,
                    None => {
                        pool.insert(
                            key.clone(),
                            PoolEntry {
                                manager: Arc::new(McpClientManager::new()),
                                claims: 1,
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
        let pool = self.state.pool.lock().await;
        let progress = self.state.progress.lock().await;
        let server_ids: Vec<String> = servers.iter().map(|server| server.name.clone()).collect();
        let mut connected = Vec::new();
        let mut diags = Vec::new();
        let mut tools: Vec<Arc<dyn XyTool>> = Vec::new();
        for entry in pool.values() {
            connected.extend(entry.connected.iter().cloned());
            diags.extend(entry.diags.iter().cloned());
            tools.extend(entry.tools.iter().cloned());
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
        }
    }

    pub async fn reset_pool(&self) {
        let entries = {
            let mut pool = self.state.pool.lock().await;
            std::mem::take(&mut *pool)
        };
        *self.state.claims.lock().await = HashMap::new();
        *self.state.last_tools.lock().await = Vec::new();
        *self.state.last_result.lock().await = None;
        for (_, entry) in entries {
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
        {
            let mut pool = self.state.pool.lock().await;
            for key in keys {
                let remove = if let Some(entry) = pool.get_mut(&key) {
                    entry.claims = entry.claims.saturating_sub(1);
                    entry.claims == 0
                } else {
                    false
                };
                if remove && let Some(entry) = pool.remove(&key) {
                    shutdowns.push(entry.manager);
                }
            }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str, scope: McpScope) -> McpServerConfig {
        McpServerConfig {
            name: name.into(),
            scope,
            command: Some(format!("{name}-bin")),
            ..Default::default()
        }
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
    fn desired_keys_isolate_credentials() {
        let mut left = server("fs", McpScope::Workspace);
        left.credential = Some("a".into());
        let mut right = server("fs", McpScope::Workspace);
        right.credential = Some("b".into());
        let keys = desired_keys(&[left, right], "s1", &["/a".into()]);
        assert_eq!(keys.len(), 2);
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
    async fn release_session_drops_claims() {
        let hub = McpPrepareHub::new(Vec::new());
        hub.release_session("s1");
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(hub.state.claims.lock().await.is_empty());
    }
}
