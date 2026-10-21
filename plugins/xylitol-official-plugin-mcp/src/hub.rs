//! Shared MCP prepare hub: idempotent connect, fire-without-wait, wait+timeout.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::FutureExt;
use futures::future::BoxFuture;
use futures::future::Shared;
use tokio::sync::Mutex;
use xylitol_plugin_sdk::XyTool;
use xylitol_plugin_sdk::plugin_events::SessionPrepareResult;
use xylitol_plugin_sdk::session_prepare::{SessionPrepareHub, SessionPrepareView};
use xylitol_plugin_sdk::tool_name::mcp_tool_armed_prefix;

use crate::assemble::connect_and_discover_with_progress;
use crate::client::{McpClientManager, McpConnectProgress};
use crate::config::McpServerConfig;

type DiscoverOk = Option<(Arc<McpClientManager>, Vec<Arc<dyn XyTool>>)>;
type DiscoverShared = Shared<BoxFuture<'static, DiscoverOk>>;

struct HubReady {
    manager: Option<Arc<McpClientManager>>,
    tools: Vec<Arc<dyn XyTool>>,
    result: SessionPrepareResult,
    connected: Vec<(String, usize)>,
    diags: Vec<String>,
}

struct HubState {
    servers: Mutex<Vec<McpServerConfig>>,
    inner: Mutex<Inner>,
    progress: Arc<Mutex<McpConnectProgress>>,
    notice: Mutex<Option<String>>,
}

struct Inner {
    inflight: Option<DiscoverShared>,
    ready: Option<HubReady>,
}

#[derive(Clone)]
pub struct McpPrepareHub {
    state: Arc<HubState>,
}

impl McpPrepareHub {
    pub fn new(servers: Vec<McpServerConfig>) -> Self {
        let total = servers.len();
        Self {
            state: Arc::new(HubState {
                servers: Mutex::new(servers),
                inner: Mutex::new(Inner {
                    inflight: None,
                    ready: None,
                }),
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

    async fn result_from_discover(discovered: DiscoverOk) -> HubReady {
        let Some((manager, tools)) = discovered else {
            return HubReady {
                manager: None,
                tools: Vec::new(),
                result: SessionPrepareResult::default(),
                connected: Vec::new(),
                diags: Vec::new(),
            };
        };
        let mut names: Vec<String> = tools.iter().map(|t| t.name().to_string()).collect();
        names.sort();
        let fingerprint = names.join("\n");
        let env_body = if names.is_empty() {
            String::new()
        } else {
            format!("MCP armed: {}", names.join(", "))
        };
        let connected = manager
            .connected_servers()
            .await
            .into_iter()
            .map(|s| (s.id, s.tool_count))
            .collect();
        let diags = manager
            .diagnostics()
            .await
            .into_iter()
            .map(|d| format!("{}: {}", d.server, d.message))
            .collect();
        HubReady {
            manager: Some(manager),
            tools,
            result: SessionPrepareResult {
                fingerprint,
                env_body,
            },
            connected,
            diags,
        }
    }

    async fn start_discover(&self) -> DiscoverShared {
        let servers = self.state.servers.lock().await.clone();
        let progress = self.state.progress.clone();
        {
            let mut p = progress.lock().await;
            p.connecting = !servers.is_empty();
            p.total = servers.len();
            p.finished = 0;
            p.current = None;
        }
        async move { connect_and_discover_with_progress(&servers, Some(progress)).await }
            .boxed()
            .shared()
    }

    pub async fn view_async(&self) -> SessionPrepareView {
        let inner = self.state.inner.lock().await;
        let progress = self.state.progress.lock().await;
        let servers = self.state.servers.lock().await;
        Self::view_from_parts(&inner, &progress, &servers)
    }

    fn view_from_parts(
        inner: &Inner,
        progress: &McpConnectProgress,
        servers: &[McpServerConfig],
    ) -> SessionPrepareView {
        let configured = servers.len();
        let ready = inner.ready.as_ref();
        let connecting = progress.connecting && ready.is_none();
        let server_ids: Vec<String> = servers.iter().map(|c| c.name.clone()).collect();
        let mut connected = Vec::new();
        let mut diags = Vec::new();
        let mut armed = Vec::new();
        if let Some(ready) = ready {
            connected = ready.connected.clone();
            diags = ready.diags.clone();
            for id in &server_ids {
                let prefix = mcp_tool_armed_prefix(id);
                let n = ready
                    .tools
                    .iter()
                    .filter(|t| t.name().starts_with(&prefix))
                    .count();
                armed.push((id.clone(), n));
            }
        }
        SessionPrepareView {
            configured,
            connected,
            diags,
            connecting_label: if connecting {
                progress.connecting_label()
            } else {
                None
            },
            connecting,
            complete: ready.is_some() || configured == 0,
            server_ids,
            armed,
        }
    }
}

#[async_trait]
impl SessionPrepareHub for McpPrepareHub {
    fn cue_connecting(&self) {
        if let Ok(mut p) = self.state.progress.try_lock() {
            let n = self
                .state
                .servers
                .try_lock()
                .map(|s| s.len())
                .unwrap_or(p.total);
            p.connecting = n > 0;
            p.total = n;
            p.finished = 0;
            p.current = None;
        }
    }

    fn fire(&self) {
        self.cue_connecting();
        let this = self.clone();
        tokio::spawn(async move {
            this.reset_join().await;
            let _ = this.ensure().await;
        });
    }

    async fn reset_join(&self) {
        let mut inner = self.state.inner.lock().await;
        inner.inflight = None;
        let old = inner.ready.take();
        drop(inner);
        if let Some(ready) = old
            && let Some(manager) = ready.manager
        {
            manager.shutdown().await;
        }
    }

    async fn ensure(&self) -> SessionPrepareResult {
        let shared = {
            let inner = self.state.inner.lock().await;
            if let Some(ready) = inner.ready.as_ref() {
                return ready.result.clone();
            }
            if let Some(inflight) = inner.inflight.clone() {
                inflight
            } else {
                drop(inner);
                let shared = self.start_discover().await;
                let mut inner = self.state.inner.lock().await;
                if let Some(ready) = inner.ready.as_ref() {
                    return ready.result.clone();
                }
                inner.inflight = Some(shared.clone());
                shared
            }
        };
        let discovered = shared.await;
        let ready = Self::result_from_discover(discovered).await;
        let result = ready.result.clone();
        let mut inner = self.state.inner.lock().await;
        inner.inflight = None;
        inner.ready = Some(ready);
        {
            let mut p = self.state.progress.lock().await;
            p.connecting = false;
            p.current = None;
        }
        result
    }

    async fn wait(&self, timeout: Duration) -> SessionPrepareResult {
        match tokio::time::timeout(timeout, self.ensure()).await {
            Ok(r) => r,
            Err(_) => {
                *self.state.notice.lock().await =
                    Some(xylitol_plugin_sdk::SESSION_PREPARE_TIMEOUT_NOTICE.into());
                self.view_async().await.armed_result()
            }
        }
    }

    fn view(&self) -> SessionPrepareView {
        let Ok(inner) = self.state.inner.try_lock() else {
            return SessionPrepareView {
                connecting: true,
                complete: false,
                ..SessionPrepareView::default()
            };
        };
        let Ok(progress) = self.state.progress.try_lock() else {
            return SessionPrepareView {
                connecting: true,
                complete: false,
                ..SessionPrepareView::default()
            };
        };
        let Ok(servers) = self.state.servers.try_lock() else {
            return SessionPrepareView {
                connecting: true,
                complete: false,
                ..SessionPrepareView::default()
            };
        };
        Self::view_from_parts(&inner, &progress, &servers)
    }

    fn take_notice(&self) -> Option<String> {
        self.state.notice.try_lock().ok().and_then(|mut g| g.take())
    }

    fn tools(&self) -> Vec<Arc<dyn XyTool>> {
        self.state
            .inner
            .try_lock()
            .ok()
            .and_then(|g| g.ready.as_ref().map(|r| r.tools.clone()))
            .unwrap_or_default()
    }
}
