//! Server runtime — second composition root for xylitol.
//!
//! Constructs the agent via the shared bootstrap path (config → registry → trust → resource discovery → build_agent), builds
//! the REST and WS routers, acquires the single-instance lock, and starts
//! the HTTP server with graceful shutdown.

use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

use crate::app::core::bootstrap::{BootstrapError, BootstrapInput, bootstrap};
use crate::app::server::lock::{LockInfo, ServerLock};
use crate::app::server::port_retry::{self, PORT_RETRY_LIMIT};
use crate::app::server::rest::{self, AppState};
use crate::app::server::ws::{EventJournal, ReverseRpcGateway};

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
///
/// Agent assembly fields (model registry, system prompt, compaction, ...) are
/// intentionally absent: the server shares the [`bootstrap`] assembly path with
/// print/tui (spec ce9), so it discovers config, resources, and trust from the
/// current environment just like print mode. Only transport-level knobs live here.
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub lock_path: Option<std::path::PathBuf>,
    pub sessions_dir: Option<std::path::PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            lock_path: None,
            sessions_dir: None,
        }
    }
}

/// Start the server. Returns a `RunningServer` handle and the bound port.
///
/// This is the server's composition root — it wires:
/// - XySessionStore (infra::session::SessionManager)
/// - XyEventSink (infra::event::EventBus)
/// - ModelRegistry / ToolSet
/// - REST router with shared state
/// - Single-instance lock
/// - Port retry
pub async fn start(
    config: ServerConfig,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    let cancel = CancellationToken::new();

    // ── Agent construction (shared bootstrap path — spec ce9) ──────
    // The server assembles the agent via the same bootstrap as print/tui, so
    // config load, resource discovery (AGENTS.md context_files,
    // append_system_prompt), trust resolution, and compaction settings all
    // apply identically. The previous headless shortcut (empty context_files,
    // caller-supplied registry) is removed: server no longer ships a
    // simplified copy of the assembly path.
    let bootstrapped = bootstrap(BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: None,
        interactive: false,
        caller: "server",
    })
    .map_err(|e| match e {
        BootstrapError::NoModelsAvailable => {
            "no models available: add explicit models.models entries in config.yaml".to_string()
        }
        BootstrapError::ConfigLoadFailed(msg) => format!("config load failed: {msg}"),
        BootstrapError::ConfigLoadedZeroModels => {
            BootstrapError::ConfigLoadedZeroModels.to_string()
        }
        BootstrapError::BuildFailed(msg) => msg,
    })?;
    let project_trusted = !bootstrapped.warnings.iter().any(|w| {
        matches!(
            w,
            crate::app::core::bootstrap::BootstrapWarning::ProjectNotTrusted { .. }
        )
    });
    let runtime = bootstrapped.into_runtime();
    let servers = runtime.mcp_servers.unwrap_or_default();
    let mut driver = runtime.driver;
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
    driver.enable_reload_state(cwd, agent_dir, project_trusted, servers);
    use crate::app::core::driver::XyDriver;
    driver.begin_mcp_bootstrap().await;
    driver.wait_mcp_bootstrap().await;
    if let Some(summary) = driver.mcp_status_summary().await {
        log::info!("{summary}");
    }

    // ── Server state (XyDriver seam — same as Print) ────────────────
    let session_id = format!("srv-{}", uuid::Uuid::new_v4());
    let journal = EventJournal::with_default_capacity(&session_id);
    let gateway = Arc::new(ReverseRpcGateway::new());
    let state = Arc::new(AppState {
        driver: Arc::new(Mutex::new(driver)),
        journal: Arc::new(Mutex::new(journal)),
        gateway,
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
