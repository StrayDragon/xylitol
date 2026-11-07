use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use futures::stream::{FuturesUnordered, StreamExt};
use serde_json::Value;
use tokio::sync::Mutex;

use super::types::{McpServerConfig, McpTransportKind};
use xylitol_plugin_sdk::tool_name::{is_provider_safe_tool_name, mcp_tool_public_name};

#[cfg(feature = "mcp")]
type McpService = rmcp::service::RunningService<rmcp::RoleClient, McpClientHandler>;
#[cfg(not(feature = "mcp"))]
type McpService = ();

#[cfg(feature = "mcp")]
#[derive(Clone)]
pub struct McpClientHandler {
    dirty: Arc<AtomicBool>,
}

#[cfg(feature = "mcp")]
impl rmcp::ClientHandler for McpClientHandler {
    fn on_tool_list_changed(
        &self,
        _context: rmcp::service::NotificationContext<rmcp::RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        self.dirty.store(true, Ordering::SeqCst);
        std::future::ready(())
    }

    fn create_elicitation(
        &self,
        request: rmcp::model::ElicitRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleClient>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ElicitResult, rmcp::model::ErrorData>>
    + Send
    + '_ {
        use rmcp::model::{ElicitRequestParams, ElicitResult, ElicitationAction};
        if let ElicitRequestParams::FormElicitationParams {
            requested_schema, ..
        } = &request
            && schema_requests_secrets(requested_schema)
        {
            log::warn!(
                "MCP elicitation declined: schema requests secret-like fields {:?}",
                requested_schema.properties.keys().collect::<Vec<_>>()
            );
        }
        std::future::ready(Ok(ElicitResult::new(ElicitationAction::Decline)))
    }
}

#[cfg(feature = "mcp")]
fn schema_requests_secrets(schema: &rmcp::model::ElicitationSchema) -> bool {
    const MARKERS: &[&str] = &[
        "password",
        "secret",
        "token",
        "api_key",
        "apikey",
        "credential",
        "private_key",
    ];
    schema.properties.keys().any(|name| {
        let lower = name.to_ascii_lowercase();
        MARKERS.iter().any(|marker| lower.contains(marker))
    })
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("spawn failed: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("init failed: {0}")]
    Init(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("invalid header name {key:?}: {source}")]
    InvalidHeaderName {
        key: String,
        #[source]
        source: http::header::InvalidHeaderName,
    },
    #[error("invalid header value for {key}: {source}")]
    InvalidHeaderValue {
        key: String,
        #[source]
        source: http::header::InvalidHeaderValue,
    },
    #[error("call {server_id}/{tool_name} failed: {source}")]
    Call {
        server_id: String,
        tool_name: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("serialize result: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("MCP server not found: {server_id}")]
    ServerNotFound { server_id: String },
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Timeout(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectDiagnostic {
    pub server: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedMcpServer {
    pub id: String,
    pub transport: McpTransportKind,
    pub tool_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpConnectProgress {
    pub connecting: bool,
    pub total: usize,
    pub finished: usize,
    pub current: Option<String>,
}

impl McpConnectProgress {
    pub fn connecting_label(&self) -> Option<String> {
        if !self.connecting || self.total == 0 {
            return None;
        }
        let n = self.finished.min(self.total);
        Some(format!("connecting {n}/{}", self.total))
    }
}

#[cfg_attr(not(feature = "mcp"), allow(dead_code))]
pub const MCP_CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
pub const MCP_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProbe {
    pub status: String,
    pub summary: String,
    pub ttl_ms: u64,
}

pub struct McpClientManager {
    services: Mutex<HashMap<String, McpService>>,
    tools_dirty: Arc<AtomicBool>,
    call_timeouts: Mutex<HashMap<String, std::time::Duration>>,
    transports: Mutex<HashMap<String, McpTransportKind>>,
    tool_counts: Mutex<HashMap<String, usize>>,
    diagnostics: Mutex<Vec<McpConnectDiagnostic>>,
    progress: Mutex<McpConnectProgress>,
    probes: Mutex<HashMap<String, CapabilityProbe>>,
    connect_attempts: AtomicUsize,
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

fn public_name_is_safe(server: &str, name: &str) -> bool {
    is_provider_safe_tool_name(&mcp_tool_public_name(server, name))
}

fn classify_probe_error(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("method not found")
        || lower.contains("unsupported")
        || lower.contains("not supported")
    {
        "unsupported"
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "timeout"
    } else if lower.contains("auth") || lower.contains("401") || lower.contains("unauthorized") {
        "auth"
    } else if lower.contains("decode") || lower.contains("invalid") {
        "invalid_response"
    } else {
        "transport"
    }
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            services: Mutex::new(HashMap::new()),
            tools_dirty: Arc::new(AtomicBool::new(false)),
            call_timeouts: Mutex::new(HashMap::new()),
            transports: Mutex::new(HashMap::new()),
            tool_counts: Mutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
            progress: Mutex::new(McpConnectProgress::default()),
            probes: Mutex::new(HashMap::new()),
            connect_attempts: AtomicUsize::new(0),
        }
    }

    pub async fn connect(self: &Arc<Self>, servers: Option<&[McpServerConfig]>) {
        match servers {
            Some(servers) if !servers.is_empty() => self.connect_servers(servers).await,
            _ => {}
        }
    }

    pub async fn connect_servers(self: &Arc<Self>, servers: &[McpServerConfig]) {
        self.connect_servers_with_progress(servers, None, None)
            .await
    }

    pub async fn connect_servers_with_progress(
        self: &Arc<Self>,
        servers: &[McpServerConfig],
        workspace: Option<&std::path::Path>,
        progress_out: Option<Arc<Mutex<McpConnectProgress>>>,
    ) {
        self.connect_attempts.fetch_add(1, Ordering::SeqCst);
        {
            let mut diags = self.diagnostics.lock().await;
            diags.clear();
        }

        let mut validated = Vec::new();
        for server_config in servers {
            let name = server_config.name.clone();
            if let Err(e) = server_config.validate() {
                log::warn!("MCP server {name}: invalid config: {e}");
                self.push_diagnostic(name, e.to_string()).await;
                continue;
            }
            validated.push(server_config.clone());
        }

        let total = validated.len();
        self.write_progress(&progress_out, true, total, 0, None)
            .await;

        let mut futs = FuturesUnordered::new();
        for server_config in validated {
            let this = Arc::clone(self);
            futs.push(async move {
                let name = server_config.name.clone();
                let connect = async {
                    match server_config.transport {
                        McpTransportKind::Stdio => {
                            this.connect_stdio(&name, &server_config, workspace).await
                        }
                        McpTransportKind::Http => this.connect_http(&name, &server_config).await,
                        McpTransportKind::Custom => Err(McpError::Config(format!(
                            "custom transport `{}` has no connector; a plugin must register one",
                            server_config
                                .custom_transport
                                .as_deref()
                                .unwrap_or("unknown")
                        ))),
                    }
                };
                let connect_timeout =
                    std::time::Duration::from_millis(server_config.connect_timeout_ms);
                let result = match tokio::time::timeout(connect_timeout, connect).await {
                    Ok(inner) => inner,
                    Err(_) => Err(McpError::Timeout(format!(
                        "connect timed out after {:?}",
                        connect_timeout
                    ))),
                };
                (name, result, server_config.call_timeout_ms)
            });
        }

        let mut finished = 0usize;
        while let Some((name, result, call_timeout_ms)) = futs.next().await {
            self.call_timeouts.lock().await.insert(
                name.clone(),
                std::time::Duration::from_millis(call_timeout_ms),
            );
            if let Err(e) = result {
                log::warn!("MCP server {name}: connection failed: {e}");
                self.push_diagnostic(name.clone(), e.to_string()).await;
            }
            finished += 1;
            self.write_progress(&progress_out, true, total, finished, Some(name))
                .await;
        }

        self.write_progress(&progress_out, false, total, finished, None)
            .await;
        self.spawn_probes().await;
    }

    async fn write_progress(
        &self,
        progress_out: &Option<Arc<Mutex<McpConnectProgress>>>,
        connecting: bool,
        total: usize,
        finished: usize,
        current: Option<String>,
    ) {
        let snap = McpConnectProgress {
            connecting,
            total,
            finished,
            current,
        };
        *self.progress.lock().await = snap.clone();
        if let Some(out) = progress_out {
            *out.lock().await = snap;
        }
    }

    async fn push_diagnostic(&self, server: String, message: String) {
        self.diagnostics
            .lock()
            .await
            .push(McpConnectDiagnostic { server, message });
    }

    async fn push_diagnostic_once(&self, server: &str, message: String) {
        let mut diags = self.diagnostics.lock().await;
        if !diags
            .iter()
            .any(|diag| diag.server == server && diag.message == message)
        {
            diags.push(McpConnectDiagnostic {
                server: server.to_string(),
                message,
            });
        }
    }

    pub fn tools_dirty(&self) -> bool {
        self.tools_dirty.load(Ordering::SeqCst)
    }

    pub fn mark_tools_dirty(&self) {
        self.tools_dirty.store(true, Ordering::SeqCst);
    }

    pub async fn diagnostics(&self) -> Vec<McpConnectDiagnostic> {
        self.diagnostics.lock().await.clone()
    }

    pub fn reconnect_count(&self) -> usize {
        self.connect_attempts
            .load(Ordering::SeqCst)
            .saturating_sub(1)
    }

    pub async fn probe_summary(&self, server: &str) -> Option<String> {
        self.probes.lock().await.get(server).map(|probe| {
            format!(
                "status={} ttl_ms={} {}",
                probe.status, probe.ttl_ms, probe.summary
            )
        })
    }

    pub async fn probe_lines(&self) -> Vec<String> {
        let probes = self.probes.lock().await;
        let mut lines: Vec<String> = probes
            .iter()
            .map(|(server, probe)| {
                format!(
                    "probe server={server} status={} ttl_ms={} {}",
                    probe.status, probe.ttl_ms, probe.summary
                )
            })
            .collect();
        lines.sort();
        lines
    }

    async fn spawn_probes(self: &Arc<Self>) {
        let peers: Vec<_> = {
            let services = self.services.lock().await;
            services
                .iter()
                .map(|(server, service)| (server.clone(), service.peer().clone()))
                .collect()
        };
        for (server, peer) in peers {
            let manager = Arc::clone(self);
            tokio::spawn(async move {
                let result =
                    tokio::time::timeout(MCP_PROBE_TIMEOUT, peer.discover(Default::default()))
                        .await;
                let probe = match result {
                    Ok(Ok(discovered)) => {
                        let mut summary = format!(
                            "versions={:?} capabilities={:?}",
                            discovered.supported_versions, discovered.capabilities
                        );
                        summary.truncate(240);
                        CapabilityProbe {
                            status: "ok".into(),
                            summary,
                            ttl_ms: discovered.ttl_ms,
                        }
                    }
                    Ok(Err(error)) => CapabilityProbe {
                        status: classify_probe_error(&error.to_string()).into(),
                        summary: error.to_string(),
                        ttl_ms: 0,
                    },
                    Err(_) => CapabilityProbe {
                        status: "timeout".into(),
                        summary: format!("probe timed out after {MCP_PROBE_TIMEOUT:?}"),
                        ttl_ms: 0,
                    },
                };
                manager.probes.lock().await.insert(server, probe);
            });
        }
    }

    pub async fn connected_servers(&self) -> Vec<ConnectedMcpServer> {
        let services = self.services.lock().await;
        let transports = self.transports.lock().await;
        let counts = self.tool_counts.lock().await;
        let mut out = Vec::new();
        for id in services.keys() {
            let tool_count = counts.get(id).copied().unwrap_or(0);
            let transport = transports
                .get(id)
                .cloned()
                .unwrap_or(McpTransportKind::Stdio);
            out.push(ConnectedMcpServer {
                id: id.clone(),
                transport,
                tool_count,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    async fn connect_stdio(
        &self,
        name: &str,
        config: &McpServerConfig,
        workspace: Option<&std::path::Path>,
    ) -> Result<(), McpError> {
        #[cfg(not(feature = "mcp"))]
        {
            let _ = (name, config);
            return Err(McpError::Config(
                "binary built without feature `mcp`".into(),
            ));
        }
        #[cfg(feature = "mcp")]
        {
            use rmcp::serve_client;
            use rmcp::transport::child_process::TokioChildProcess;
            use tokio::process::Command;
            use xylitol_plugin_sdk::host_env;

            let command = config.command.as_ref().ok_or_else(|| {
                McpError::Config("command is required for stdio transport".into())
            })?;
            let args = config.args.as_deref().unwrap_or_default();

            let mut cmd = Command::new(command);
            cmd.args(args);
            if let Some(workspace) = workspace {
                cmd.current_dir(workspace);
            }
            if let Some(ref env) = config.env {
                for (k, v) in env {
                    cmd.env(k, v);
                }
            }
            host_env::scrub_tokio_command(&mut cmd);

            let transport = TokioChildProcess::new(cmd).map_err(McpError::Spawn)?;

            let handler = McpClientHandler {
                dirty: self.tools_dirty.clone(),
            };
            let service = serve_client(handler, transport)
                .await
                .map_err(|e| McpError::Init(Box::new(e)))?;

            let mut services = self.services.lock().await;
            services.insert(name.to_string(), service);
            self.transports
                .lock()
                .await
                .insert(name.to_string(), McpTransportKind::Stdio);
            log::info!(
                "MCP server connected name={} transport={}",
                { name },
                "stdio"
            );
            Ok(())
        }
    }

    async fn connect_http(&self, name: &str, config: &McpServerConfig) -> Result<(), McpError> {
        if let Some(url) = config.url.as_deref()
            && url.trim_end_matches('/').ends_with("/sse")
        {
            return Err(McpError::Config(format!(
                "legacy HTTP+SSE is not supported: {url}; use Streamable HTTP"
            )));
        }
        #[cfg(not(feature = "mcp"))]
        {
            let _ = (name, config);
            return Err(McpError::Config(
                "binary built without feature `mcp`".into(),
            ));
        }
        #[cfg(feature = "mcp")]
        {
            use http::{HeaderName, HeaderValue};
            use rmcp::serve_client;
            use rmcp::transport::StreamableHttpClientTransport;
            use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

            let url = config
                .url
                .as_ref()
                .ok_or_else(|| McpError::Config("url is required for sse transport".into()))?;
            let mut transport_cfg = StreamableHttpClientTransportConfig::with_uri(url.clone());
            if let Some(ref headers) = config.headers {
                let mut custom = HashMap::new();
                for (k, v) in headers {
                    let name = HeaderName::from_bytes(k.as_bytes()).map_err(|source| {
                        McpError::InvalidHeaderName {
                            key: k.clone(),
                            source,
                        }
                    })?;
                    let value = HeaderValue::from_str(v).map_err(|source| {
                        McpError::InvalidHeaderValue {
                            key: k.clone(),
                            source,
                        }
                    })?;
                    custom.insert(name, value);
                }
                transport_cfg = transport_cfg.custom_headers(custom);
            }
            let transport = StreamableHttpClientTransport::from_config(transport_cfg);
            let handler = McpClientHandler {
                dirty: self.tools_dirty.clone(),
            };
            let service = serve_client(handler, transport)
                .await
                .map_err(|e| McpError::Init(Box::new(e)))?;

            let mut services = self.services.lock().await;
            services.insert(name.to_string(), service);
            self.transports
                .lock()
                .await
                .insert(name.to_string(), McpTransportKind::Http);
            log::info!(
                "MCP server connected name={} transport={}",
                { name },
                "http"
            );
            Ok(())
        }
    }

    pub async fn list_all_tools(&self) -> Vec<(String, String, String, Value)> {
        #[cfg(not(feature = "mcp"))]
        {
            Vec::new()
        }
        #[cfg(feature = "mcp")]
        {
            let mut result = Vec::new();
            let mut counts: HashMap<String, usize> = HashMap::new();
            let services = self.services.lock().await;
            for (server_id, service) in services.iter() {
                let tools =
                    match tokio::time::timeout(MCP_CALL_TIMEOUT, service.list_all_tools()).await {
                        Err(_) => {
                            log::warn!(
                                "list_all_tools timed out server_id={} timeout_secs={}",
                                { server_id },
                                MCP_CALL_TIMEOUT.as_secs()
                            );
                            counts.insert(server_id.clone(), 0);
                            continue;
                        }
                        Ok(r) => r,
                    };
                let tools = match tools {
                    Ok(t) => t,
                    Err(e) => {
                        log::warn!(
                            "list_all_tools failed server_id={} error={}",
                            { server_id },
                            e
                        );
                        counts.insert(server_id.clone(), 0);
                        continue;
                    }
                };
                let mut safe_count = 0;
                for tool in tools {
                    let tool_name = tool.name.to_string();
                    if !public_name_is_safe(server_id, &tool_name) {
                        let public = mcp_tool_public_name(server_id, &tool_name);
                        log::warn!(
                            "MCP tool name not provider-safe; skipped server={server_id} tool={tool_name} public={public}"
                        );
                        self.push_diagnostic_once(
                            server_id,
                            format!(
                                "tool '{tool_name}' skipped: public name '{public}' is not provider-safe"
                            ),
                        )
                        .await;
                        continue;
                    }
                    let description = tool.description.as_deref().unwrap_or("").to_string();
                    let schema = Value::Object(tool.input_schema.as_ref().clone());
                    result.push((server_id.clone(), tool_name, description, schema));
                    safe_count += 1;
                }
                counts.insert(server_id.clone(), safe_count);
            }
            drop(services);
            *self.tool_counts.lock().await = counts;
            self.tools_dirty.store(false, Ordering::SeqCst);
            result
        }
    }

    pub async fn list_all_prompts(&self) -> Vec<(String, String, String, Value)> {
        #[cfg(not(feature = "mcp"))]
        {
            Vec::new()
        }
        #[cfg(feature = "mcp")]
        {
            let mut result = Vec::new();
            let services = self.services.lock().await;
            for (server_id, service) in services.iter() {
                let prompts = match tokio::time::timeout(
                    MCP_CALL_TIMEOUT,
                    service.list_all_prompts(),
                )
                .await
                {
                    Ok(Ok(prompts)) => prompts,
                    Ok(Err(e)) => {
                        log::warn!("list_all_prompts failed server_id={server_id} error={e}");
                        continue;
                    }
                    Err(_) => {
                        log::warn!("list_all_prompts timed out server_id={server_id}");
                        continue;
                    }
                };
                for prompt in prompts {
                    let description = prompt.description.unwrap_or_default();
                    let mut properties = serde_json::Map::new();
                    let mut required = Vec::new();
                    for argument in prompt.arguments.unwrap_or_default() {
                        properties.insert(
                            argument.name.clone(),
                            serde_json::json!({
                                "type": "string",
                                "description": argument.description.unwrap_or_default(),
                            }),
                        );
                        if argument.required.unwrap_or(false) {
                            required.push(argument.name);
                        }
                    }
                    let schema = serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": required,
                    });
                    let prompt_name = prompt.name;
                    if !public_name_is_safe(server_id, &format!("prompt__{prompt_name}")) {
                        let public =
                            mcp_tool_public_name(server_id, &format!("prompt__{prompt_name}"));
                        log::warn!(
                            "MCP prompt name not provider-safe; skipped server={server_id} prompt={prompt_name} public={public}"
                        );
                        self.push_diagnostic_once(
                            server_id,
                            format!(
                                "prompt '{prompt_name}' skipped: public name '{public}' is not provider-safe"
                            ),
                        )
                        .await;
                        continue;
                    }
                    result.push((server_id.clone(), prompt_name, description, schema));
                }
            }
            result
        }
    }

    pub async fn get_prompt(
        &self,
        server_id: &str,
        prompt_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, McpError> {
        #[cfg(not(feature = "mcp"))]
        {
            let _ = (server_id, prompt_name, arguments);
            Err(McpError::Config("binary built without feature mcp".into()))
        }
        #[cfg(feature = "mcp")]
        {
            use rmcp::model::GetPromptRequestParams;

            let services = self.services.lock().await;
            let service = services
                .get(server_id)
                .ok_or_else(|| McpError::ServerNotFound {
                    server_id: server_id.to_string(),
                })?;
            let mut params = GetPromptRequestParams::new(prompt_name.to_string());
            params.arguments = arguments.and_then(|value| value.as_object().cloned());
            let call_timeout = self
                .call_timeouts
                .lock()
                .await
                .get(server_id)
                .copied()
                .unwrap_or(MCP_CALL_TIMEOUT);
            let result = tokio::time::timeout(call_timeout, service.get_prompt(params))
                .await
                .map_err(|_| {
                    McpError::Timeout(format!(
                        "mcp prompt get timed out after {call_timeout:?} (server={server_id}, prompt={prompt_name})"
                    ))
                })?
                .map_err(|source| McpError::Call {
                    server_id: server_id.to_string(),
                    tool_name: prompt_name.to_string(),
                    source: Box::new(source),
                })?;
            serde_json::to_value(&result).map_err(McpError::Serialize)
        }
    }

    pub async fn list_all_resources(&self) -> Vec<(String, String, String, String)> {
        #[cfg(not(feature = "mcp"))]
        {
            Vec::new()
        }
        #[cfg(feature = "mcp")]
        {
            let mut result = Vec::new();
            let services = self.services.lock().await;
            for (server_id, service) in services.iter() {
                let resources = match tokio::time::timeout(
                    MCP_CALL_TIMEOUT,
                    service.list_all_resources(),
                )
                .await
                {
                    Ok(Ok(resources)) => resources,
                    Ok(Err(e)) => {
                        log::warn!("list_all_resources failed server_id={server_id} error={e}");
                        continue;
                    }
                    Err(_) => {
                        log::warn!("list_all_resources timed out server_id={server_id}");
                        continue;
                    }
                };
                for resource in resources {
                    result.push((
                        server_id.clone(),
                        resource.uri,
                        resource.name,
                        resource.description.unwrap_or_default(),
                    ));
                }
            }
            result
        }
    }

    pub async fn read_resource(&self, server_id: &str, uri: &str) -> Result<Value, McpError> {
        #[cfg(not(feature = "mcp"))]
        {
            let _ = (server_id, uri);
            Err(McpError::Config("binary built without feature mcp".into()))
        }
        #[cfg(feature = "mcp")]
        {
            use rmcp::model::ReadResourceRequestParams;

            let services = self.services.lock().await;
            let service = services
                .get(server_id)
                .ok_or_else(|| McpError::ServerNotFound {
                    server_id: server_id.to_string(),
                })?;
            let call_timeout = self
                .call_timeouts
                .lock()
                .await
                .get(server_id)
                .copied()
                .unwrap_or(MCP_CALL_TIMEOUT);
            let result = tokio::time::timeout(
                call_timeout,
                service.read_resource(ReadResourceRequestParams::new(uri.to_string())),
            )
            .await
            .map_err(|_| {
                McpError::Timeout(format!(
                    "mcp resource read timed out after {call_timeout:?} (server={server_id}, uri={uri})"
                ))
            })?
            .map_err(|source| McpError::Call {
                server_id: server_id.to_string(),
                tool_name: uri.to_string(),
                source: Box::new(source),
            })?;
            serde_json::to_value(&result).map_err(McpError::Serialize)
        }
    }

    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        args: Value,
    ) -> Result<Value, McpError> {
        #[cfg(not(feature = "mcp"))]
        {
            let _ = (server_id, tool_name, args);
            return Err(McpError::Config(
                "binary built without feature `mcp`".into(),
            ));
        }
        #[cfg(feature = "mcp")]
        {
            use rmcp::model::{CallToolRequestParams, CallToolResult};

            let services = self.services.lock().await;
            let service = services
                .get(server_id)
                .ok_or_else(|| McpError::ServerNotFound {
                    server_id: server_id.to_string(),
                })?;

            let args_map = args.as_object().cloned().unwrap_or_default();
            let params = CallToolRequestParams::new(tool_name.to_string()).with_arguments(args_map);

            let call_timeout = self
                .call_timeouts
                .lock()
                .await
                .get(server_id)
                .copied()
                .unwrap_or(MCP_CALL_TIMEOUT);
            let result: CallToolResult = tokio::time::timeout(
                call_timeout,
                service.call_tool(params),
            )
            .await
            .map_err(|_| {
                McpError::Timeout(format!(
                    "mcp tool call timed out after {:?} (server={server_id}, tool={tool_name})",
                    call_timeout
                ))
            })?
            .map_err(|source| McpError::Call {
                server_id: server_id.to_string(),
                tool_name: tool_name.to_string(),
                source: Box::new(source),
            })?;

            let json = serde_json::to_value(&result).map_err(McpError::Serialize)?;
            Ok(json)
        }
    }

    pub async fn shutdown(&self) {
        let mut services = self.services.lock().await;
        #[cfg(feature = "mcp")]
        for (name, mut service) in services.drain() {
            if let Err(e) = service.close().await {
                log::warn!("MCP server shutdown error server={} error={}", name, e);
            }
        }
        #[cfg(not(feature = "mcp"))]
        services.clear();
        self.transports.lock().await.clear();
        self.tool_counts.lock().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn elicitation_secret_schema_detected() {
        let schema: rmcp::model::ElicitationSchema = serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {"password": {"type": "string"}}
        }))
        .expect("schema");
        assert!(schema_requests_secrets(&schema));

        let schema: rmcp::model::ElicitationSchema = serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {"name": {"type": "string"}}
        }))
        .expect("schema");
        assert!(!schema_requests_secrets(&schema));
    }

    #[tokio::test]
    async fn tools_dirty_flag_clears_after_listing() {
        let manager = McpClientManager::new();
        manager.mark_tools_dirty();
        assert!(manager.tools_dirty());
        let _ = manager.list_all_tools().await;
        assert!(!manager.tools_dirty());
    }

    #[test]
    fn test_mcp_client_manager_new() {
        let manager = McpClientManager::new();
        let services = manager.services.blocking_lock();
        assert!(services.is_empty());
    }

    #[test]
    fn test_connect_with_no_servers() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = Arc::new(McpClientManager::new());
        rt.block_on(manager.connect(None));
        let services = rt.block_on(async { manager.services.lock().await });
        assert!(services.is_empty());
    }

    #[test]
    fn test_connect_with_empty_servers_list() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = Arc::new(McpClientManager::new());
        rt.block_on(manager.connect(Some(&[])));
        let services = rt.block_on(async { manager.services.lock().await });
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn invalid_stdio_records_diagnostic_no_service() {
        let manager = Arc::new(McpClientManager::new());
        let servers = vec![McpServerConfig {
            name: "bad".into(),
            transport: McpTransportKind::Stdio,
            command: None,
            ..Default::default()
        }];
        manager.connect_servers(&servers).await;
        let diags = manager.diagnostics().await;
        assert!(
            diags
                .iter()
                .any(|d| d.server == "bad" && d.message.contains("command")),
            "expected command diagnostic; got {diags:?}"
        );
        assert!(manager.connected_servers().await.is_empty());
    }

    #[tokio::test]
    async fn invalid_sse_url_records_diagnostic() {
        let manager = Arc::new(McpClientManager::new());
        let servers = vec![McpServerConfig {
            name: "bad-url".into(),
            transport: McpTransportKind::Http,
            url: Some("not-a-url".into()),
            ..Default::default()
        }];
        manager.connect_servers(&servers).await;
        let diags = manager.diagnostics().await;
        assert!(
            diags
                .iter()
                .any(|d| d.server == "bad-url" && d.message.contains("url")),
            "expected url diagnostic; got {diags:?}"
        );
    }

    #[tokio::test]
    async fn connect_fail_continues_and_ok() {
        let manager = Arc::new(McpClientManager::new());
        let servers = vec![
            McpServerConfig {
                name: "missing-bin".into(),
                transport: McpTransportKind::Stdio,
                command: Some("/nonexistent/xylitol-mcp-fake-bin".into()),
                ..Default::default()
            },
            McpServerConfig {
                name: "also-bad".into(),
                transport: McpTransportKind::Stdio,
                command: None,
                ..Default::default()
            },
        ];
        manager.connect_servers(&servers).await;
        let diags = manager.diagnostics().await;
        assert!(diags.len() >= 2, "both failures observable; got {diags:?}");
        assert!(manager.connected_servers().await.is_empty());
    }

    #[tokio::test]
    async fn connect_timeout_records_diagnostic() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    drop(stream);
                });
            }
        });
        let manager = Arc::new(McpClientManager::new());
        let servers = vec![McpServerConfig {
            name: "hanging".into(),
            transport: McpTransportKind::Http,
            url: Some(format!("http://{addr}/mcp")),
            connect_timeout_ms: 100,
            ..Default::default()
        }];
        let started = std::time::Instant::now();
        manager.connect_servers(&servers).await;
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        let diags = manager.diagnostics().await;
        assert!(
            diags
                .iter()
                .any(|d| d.server == "hanging" && d.message.contains("timed out")),
            "expected timeout diagnostic; got {diags:?}"
        );
    }

    #[tokio::test]
    async fn connected_servers_empty_without_connect() {
        let manager = McpClientManager::new();
        assert!(manager.connected_servers().await.is_empty());
        assert!(manager.diagnostics().await.is_empty());
    }

    #[tokio::test]
    async fn connected_servers_reads_cached_counts_without_relisting() {
        let manager = Arc::new(McpClientManager::new());
        let _ = manager.list_all_tools().await;
        let t0 = std::time::Instant::now();
        let rows = manager.connected_servers().await;
        assert!(t0.elapsed().as_millis() < 50, "cache path must stay local");
        assert!(rows.is_empty());
    }

    #[test]
    fn connecting_label_is_ready_over_total() {
        let start = McpConnectProgress {
            connecting: true,
            total: 2,
            finished: 0,
            current: None,
        };
        assert_eq!(start.connecting_label().as_deref(), Some("connecting 0/2"));

        let mid = McpConnectProgress {
            connecting: true,
            total: 2,
            finished: 1,
            current: Some("lspz".into()),
        };
        assert_eq!(mid.connecting_label().as_deref(), Some("connecting 1/2"));

        let done_but_boot = McpConnectProgress {
            connecting: true,
            total: 2,
            finished: 2,
            current: None,
        };
        assert_eq!(
            done_but_boot.connecting_label().as_deref(),
            Some("connecting 2/2")
        );

        let settled = McpConnectProgress {
            connecting: false,
            total: 2,
            finished: 2,
            current: None,
        };
        assert!(settled.connecting_label().is_none());
    }

    #[test]
    fn classify_probe_error_categories() {
        assert_eq!(
            classify_probe_error("method not found: server/discover"),
            "unsupported"
        );
        assert_eq!(classify_probe_error("request timed out"), "timeout");
        assert_eq!(classify_probe_error("401 unauthorized"), "auth");
        assert_eq!(classify_probe_error("invalid response"), "invalid_response");
        assert_eq!(classify_probe_error("connection reset"), "transport");
    }

    #[test]
    fn public_name_safety_rejects_unsafe_segments() {
        assert!(public_name_is_safe("aaa", "read"));
        assert!(public_name_is_safe("team-docs", "read_file"));
        assert!(!public_name_is_safe("team/docs", "read"));
        assert!(!public_name_is_safe("aaa", "read/write"));
    }

    #[test]
    fn reconnect_count_starts_at_zero() {
        let manager = McpClientManager::new();
        assert_eq!(manager.reconnect_count(), 0);
    }

    #[tokio::test]
    async fn probe_lines_are_sorted_and_include_status() {
        let manager = McpClientManager::new();
        {
            let mut probes = manager.probes.lock().await;
            probes.insert(
                "b".into(),
                CapabilityProbe {
                    status: "ok".into(),
                    summary: "versions=[]".into(),
                    ttl_ms: 1000,
                },
            );
            probes.insert(
                "a".into(),
                CapabilityProbe {
                    status: "unsupported".into(),
                    summary: "method not found".into(),
                    ttl_ms: 0,
                },
            );
        }
        let lines = manager.probe_lines().await;
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("probe server=a status=unsupported"),
            "{lines:?}"
        );
        assert!(
            lines[1].starts_with("probe server=b status=ok"),
            "{lines:?}"
        );
    }
}
