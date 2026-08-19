//! Server runtime — Host composition root (salvo listener).
//!
//! Binds `ServerConfig.host`+`port` (default 127.0.0.1:18790). EADDRINUSE fails;
//! no port+1, no lock file. Product routes are four-quadrant POST unary + mux
//! downlink (see [`super::http`]). Session occupancy is lazy per slot
//! ([`super::host::HostState`]).

use std::sync::Arc;
use std::time::Duration;

use salvo::conn::tcp::TcpAcceptor;
use salvo::prelude::*;
use salvo::server::ServerHandle;

use crate::app::core::bootstrap::{BootstrapError, BootstrapInput, resolve_assembly};
use crate::app::core::composition::build_ports;
use crate::app::server::host::{HostState, ReloadBaseline};
use crate::app::server::http;

/// Handle to a running server. Dropping this triggers graceful shutdown.
pub struct RunningServer {
    handle: ServerHandle,
    host: Arc<HostState>,
}

impl RunningServer {
    /// Signal the server to shut down gracefully.
    pub fn shutdown(&self) {
        self.host
            .shutting_down
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.handle.stop_graceful(Duration::from_secs(30));
    }

    /// Wait for the server to finish shutting down.
    pub async fn join(self) {}
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.host
            .shutting_down
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.handle.stop_graceful(Duration::from_secs(30));
    }
}

/// Configuration for starting the server.
///
/// Agent assembly fields live in the shared `resolve_assembly` / `build_ports`
/// path (spec ce9). Only transport-level knobs live here.
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub sessions_dir: Option<std::path::PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 18790,
            sessions_dir: None,
        }
    }
}

/// Start the product Host listener (bootstrap → RuntimePorts → salvo).
pub async fn start(
    config: ServerConfig,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    let assembly = resolve_assembly(&BootstrapInput {
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

    let fallback_session = assembly.session_id.clone();
    let mcp_servers = assembly.mcp_servers.clone().unwrap_or_default();
    let project_trusted = !assembly.warnings.iter().any(|w| {
        matches!(
            w,
            crate::app::core::bootstrap::BootstrapWarning::ProjectNotTrusted { .. }
        )
    });
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
    let ports = if let Some(dir) = &config.sessions_dir {
        let store: std::sync::Arc<dyn crate::protocol::ports::XySessionStore> =
            std::sync::Arc::new(crate::infra::session::SessionManager::new(dir.clone()));
        crate::app::core::composition::build_ports_with_store(assembly.into_build_options(), store)?
    } else {
        build_ports(assembly.into_build_options())?
    };

    let host = HostState::from_bootstrap(
        ports,
        ReloadBaseline {
            cwd,
            agent_dir,
            project_trusted,
            mcp_servers,
        },
        fallback_session,
    );
    serve(config, host).await
}

/// Bind and serve an already-built [`HostState`] (tests / custom assembly).
pub async fn serve(
    config: ServerConfig,
    host: Arc<HostState>,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    // Fail on EADDRINUSE: bind with tokio first so we get a Result, then hand off.
    let std_listener = std::net::TcpListener::bind(&addr).map_err(|e| {
        Box::new(std::io::Error::new(e.kind(), format!("bind {addr}: {e}")))
            as Box<dyn std::error::Error>
    })?;
    std_listener.set_nonblocking(true)?;
    let local = std_listener.local_addr()?;
    let actual_port = local.port();

    let tokio_listener = tokio::net::TcpListener::from_std(std_listener)?;
    let acceptor = TcpAcceptor::try_from(tokio_listener)?;
    let router = http::router(host.clone());
    let server = Server::new(acceptor);
    let handle = server.handle();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal(shutdown_handle).await;
    });
    tokio::spawn(async move {
        server.serve(router).await;
    });

    Ok((RunningServer { handle, host }, actual_port))
}

async fn shutdown_signal(handle: ServerHandle) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM");
        let mut sigquit = signal(SignalKind::quit()).expect("install SIGQUIT");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigquit.recv() => {}
        }
    };

    #[cfg(not(unix))]
    let terminate = async {
        std::future::pending::<()>().await;
    };

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
    handle.stop_graceful(Duration::from_secs(30));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn eaddrinuse_fails_without_port_plus_one() {
        let host = HostState::for_test().expect("host");
        let (running, port) = serve(
            ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                sessions_dir: None,
            },
            host,
        )
        .await
        .expect("first bind");
        let occupied = port;
        let host2 = HostState::for_test().expect("host2");
        let second = serve(
            ServerConfig {
                host: "127.0.0.1".into(),
                port: occupied,
                sessions_dir: None,
            },
            host2,
        )
        .await;
        assert!(second.is_err(), "second bind must fail");
        if let Err(err) = second {
            let msg = err.to_string();
            assert!(
                msg.contains("Address already in use")
                    || msg.contains("addr")
                    || msg.contains("bind")
                    || msg.to_lowercase().contains("in use"),
                "expected EADDRINUSE-ish error, got {msg}"
            );
        }
        running.shutdown();
    }
}
