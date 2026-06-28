//! Server runtime — second composition root for xylitol.
//!
//! Constructs the agent with port injections (SessionStore/EventSink), builds
//! the REST and WS routers, acquires the single-instance lock, and starts
//! the HTTP server with graceful shutdown.

use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

use crate::agent::compaction::CompactionSettings;
use crate::agent::facade::Agent;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::tools::ToolRegistry;
use crate::core::ports::{BashExecutor, EventSink, SessionStore};
use crate::infra::bash_exec::InfraBashExecutor;
use crate::infra::config::value::InfraSecretResolver;
use crate::infra::event::EventBus;
use crate::infra::session::SessionManager;
use crate::server::lock::{LockInfo, ServerLock};
use crate::server::port_retry::{self, PORT_RETRY_LIMIT};
use crate::server::rest::{self, AppState};
use crate::server::ws::{EventJournal, ReverseRpcGateway};

/// Handle to a running server. Dropping this triggers graceful shutdown.
pub struct RunningServer {
    cancel: CancellationToken,
    /// Single-instance lock; held for the struct's lifetime so the OS lock
    /// stays acquired, and released when `RunningServer` is dropped. Never
    /// read directly.
    #[allow(dead_code)]
    lock: Option<ServerLock>,
}

impl RunningServer {
    /// Signal the server to shut down gracefully.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }

    /// Wait for the server to finish shutting down.
    pub async fn join(self) {
        // Lock drops on destruction which removes the lock file.
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Configuration for starting the server.
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub lock_path: Option<std::path::PathBuf>,
    pub sessions_dir: Option<std::path::PathBuf>,
    pub model_registry: ModelRegistry,
    pub system_prompt: Option<String>,
    pub max_iterations: u32,
    pub compaction_threshold: f64,
    pub compaction_settings: Option<CompactionSettings>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            lock_path: None,
            sessions_dir: None,
            model_registry: ModelRegistry::new(Arc::new(InfraSecretResolver::new())),
            system_prompt: None,
            max_iterations: 50,
            compaction_threshold: 0.8,
            compaction_settings: None,
        }
    }
}

/// Start the server. Returns a `RunningServer` handle and the bound port.
///
/// This is the server's composition root — it wires:
/// - SessionStore (infra::session::SessionManager)
/// - EventSink (infra::event::EventBus)
/// - ModelRegistry / ToolRegistry
/// - REST router with shared state
/// - Single-instance lock
/// - Port retry
pub async fn start(
    config: ServerConfig,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    let cancel = CancellationToken::new();

    // ── Port construction ──────────────────────────────────────────
    let sessions_dir = config
        .sessions_dir
        .unwrap_or_else(SessionManager::default_dir);
    std::fs::create_dir_all(&sessions_dir)?;
    let session_mgr = SessionManager::new(sessions_dir);
    let store: Arc<dyn SessionStore> = Arc::new(session_mgr.clone());
    let sink: Arc<dyn EventSink> = Arc::new(EventBus::new());

    // ── Agent construction ─────────────────────────────────────────
    let tool_registry = ToolRegistry::from_tools(crate::infra::tools::default_tools());
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let bash_executor: Arc<dyn BashExecutor> = Arc::new(InfraBashExecutor::new());
    let agent = Agent::with_ports(
        config.model_registry.clone(),
        tool_registry,
        store,
        sink,
        config.system_prompt,
        // Server mode: no AGENTS.md context_files / append_system_prompt
        // wired (server is headless; resource discovery is the caller's job).
        Vec::new(),
        Vec::new(),
        config.max_iterations,
        config.compaction_threshold,
        cwd,
        config.compaction_settings,
        // HC-1: model builder + sandbox supplied by the composition root.
        Arc::new(crate::infra::provider::factory::build_provider),
        crate::infra::sandbox::noop_engine(),
        bash_executor,
    );

    // ── Server state ──────────────────────────────────────────────
    let session_id = format!("srv-{}", uuid::Uuid::new_v4());
    let journal = EventJournal::with_default_capacity(&session_id);
    let gateway = Arc::new(ReverseRpcGateway::new());
    let state = Arc::new(AppState {
        agent: Arc::new(Mutex::new(agent)),
        journal: Arc::new(Mutex::new(journal)),
        gateway,
        model_registry: config.model_registry,
    });

    // ── Lock acquisition ──────────────────────────────────────────
    let lock_path = config
        .lock_path
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/xylitol-server.lock"));

    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "localhost".into());

    let (listener, actual_port, lock) = acquire_lock_and_bind(&lock_path, &hostname, config.port)?;

    // ── Router ────────────────────────────────────────────────────
    let app = rest::router(state).layer(CorsLayer::permissive());

    // ── Start server ──────────────────────────────────────────────
    let server_cancel = cancel.clone();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                server_cancel.cancelled().await;
            })
            .await
            .ok();
    });

    Ok((
        RunningServer {
            cancel,
            lock: Some(lock),
        },
        actual_port,
    ))
}

/// Acquire the server lock and bind to a port, with retry if the port is busy.
fn acquire_lock_and_bind(
    lock_path: &std::path::Path,
    hostname: &str,
    start_port: u16,
) -> Result<(tokio::net::TcpListener, u16, ServerLock), Box<dyn std::error::Error>> {
    let pid = std::process::id();

    let info = LockInfo {
        port: start_port,
        pid,
        hostname: hostname.to_string(),
    };
    let lock = ServerLock::try_acquire(lock_path, &info)?;

    for offset in 0..PORT_RETRY_LIMIT {
        let try_port = start_port + offset;
        // Use std TcpListener first to probe, then convert to tokio
        match std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, try_port)) {
            Ok(std_listener) => {
                let actual = std_listener.local_addr().unwrap().port();
                if actual != start_port {
                    lock.update_port(actual)?;
                }
                // Convert to tokio listener
                std_listener.set_nonblocking(true)?;
                let tokio_listener = TcpListener::from_std(std_listener)?;
                return Ok((tokio_listener, actual, lock));
            }
            Err(_) if offset < PORT_RETRY_LIMIT - 1 => continue,
            Err(e) => {
                return Err(Box::new(port_retry::PortRetryExhausted {
                    start_port,
                    last_error: e,
                }));
            }
        }
    }

    unreachable!("loop always returns or errors")
}
