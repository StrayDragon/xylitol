//! Server runtime — Host composition root (assemble, then bind).
//!
//! Bind / HTTP / WS live in `xylitol-serve`. Occupancy is [`HostState`].
//! Binds `ServerConfig.host`+`port` (default 127.0.0.1:18790). EADDRINUSE fails;
//! no port+1, no lock file. Session occupancy is lazy per slot.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use xylitol_serve::{Gateway, ListenConfig, OccupiedHost, RunningListener};

use crate::wiring::bootstrap::{BootstrapError, BootstrapInput, resolve_assembly};
use crate::wiring::composition::build_baseline;
use crate::wiring::occupancy::factory::ProductOccupancyFactory;
use crate::wiring::occupancy::host::{HostState, ReloadBaseline};

/// Handle to a running server. Dropping this triggers graceful shutdown.
pub type RunningServer = RunningListener;

/// Configuration for starting the server.
///
/// Agent assembly fields live in the shared `resolve_assembly` / `build_baseline`
/// path (spec ce9). Transport knobs are forwarded to [`ListenConfig`].
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub sessions_dir: Option<std::path::PathBuf>,
    /// Registration file override (tests / BDD). `None` = `~/.xylitol/serve.json`.
    pub registration_path: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 18790,
            sessions_dir: None,
            registration_path: None,
        }
    }
}

impl ServerConfig {
    fn listen(&self) -> ListenConfig {
        ListenConfig {
            host: self.host.clone(),
            port: self.port,
        }
    }
}

fn occupy(host: Arc<HostState>) -> Arc<dyn OccupiedHost> {
    host
}

/// Start the product Host listener (bind-first, then assemble: c2465 readiness
/// window — connections during assembly get a semantic 503 instead of refusal).
pub async fn start(
    config: ServerConfig,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    let gateway = Gateway::starting();
    let (mut running, port) = xylitol_serve::bind_listen(&config.listen(), gateway.clone()).await?;
    match assemble(&config).await {
        Ok(host) => {
            gateway.set_host(occupy(host));
            let registration_path = config
                .registration_path
                .clone()
                .unwrap_or_else(xylitol_serve::default_registration_path);
            if let Err(e) = running.arm_registration(
                format!("http://{}:{port}", config.host),
                registration_path,
                Duration::from_secs(5),
            ) {
                return Err(Box::new(e));
            }
            Ok((running, port))
        }
        Err(e) => {
            gateway.mark_failed();
            // Bounded hold so a probing client can read the failed reason
            // before the listener goes away (c2465 D2).
            tokio::time::sleep(Duration::from_secs(10)).await;
            drop(running);
            Err(e)
        }
    }
}

/// Config → assembly → RuntimeBaseline → HostState (the pre-bind startup work).
async fn assemble(config: &ServerConfig) -> Result<Arc<HostState>, Box<dyn std::error::Error>> {
    let assembly = resolve_assembly(&BootstrapInput {
        config_path: None,
        session: None,
        model: None,
        trust_override: None,
        interactive: false,
        caller: "server",
        ephemeral_sessions: false,
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
    let default_model_id = assembly.default_profile_model.clone();
    let project_trusted = !assembly.warnings.iter().any(|w| {
        matches!(
            w,
            crate::wiring::bootstrap::BootstrapWarning::ProjectNotTrusted { .. }
        )
    });
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let agent_dir = xylitol_runtime::resource::DefaultResourceLoader::default_agent_dir();
    let ports = if let Some(dir) = &config.sessions_dir {
        let store: std::sync::Arc<dyn xylitol_plugin_sdk::XySessionStore> = std::sync::Arc::new(
            xylitol_official_plugin_session::SessionManager::new(dir.clone()),
        );
        crate::wiring::composition::build_baseline_with_store(assembly.into_build_options(), store)?
    } else {
        build_baseline(assembly.into_build_options())?
    };

    let factory = Arc::new(ProductOccupancyFactory::new(
        ports.clone(),
        cwd.clone(),
        agent_dir.clone(),
        project_trusted,
        mcp_servers,
        default_model_id.clone(),
    ));
    Ok(HostState::from_bootstrap(
        ports,
        ReloadBaseline {
            cwd,
            agent_dir,
            project_trusted,
            default_model_id,
        },
        factory,
        fallback_session,
    ))
}

/// Bind and serve an already-built [`HostState`] (tests / custom assembly).
/// Ready as soon as the socket is up (no assembly window).
///
/// BDD @executable readiness scenarios + driver unit tests; no product caller.
#[allow(dead_code)] // BDD @executable contract / unit-test seam, not product-called
pub async fn serve(
    config: ServerConfig,
    host: Arc<HostState>,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    xylitol_serve::serve(config.listen(), occupy(host)).await
}

/// Pre-assembled variant that also arms the registration contract (BDD /
/// tests): ready at bind, registration written, self-eviction armed.
#[allow(dead_code)] // BDD @executable contract / unit-test seam, not product-called
pub async fn serve_registered(
    config: ServerConfig,
    host: Arc<HostState>,
    registration_path: PathBuf,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    xylitol_serve::serve_registered(config.listen(), occupy(host), registration_path).await
}

/// Bind and serve with an explicit gateway — the assembly-window seam used by
/// [`start`] and the readiness BDD scenarios (c2465).
pub async fn bind_serve(
    config: &ServerConfig,
    gateway: Arc<Gateway>,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    xylitol_serve::bind_listen(&config.listen(), gateway).await
}

/// Domain-object event parity: inject journal events, collect v3 mux frames.
pub async fn dual_rail_event_parity()
-> Result<Vec<(xylitol_protocol::Event, xylitol_protocol::Event)>, String> {
    use futures::StreamExt;
    use xylitol_app_tui::host_client::{HostClient, HttpWsClient};

    let host = crate::wiring::occupancy::host_state_for_test().map_err(|e| format!("host: {e}"))?;
    let (_running, port) = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host.clone(),
    )
    .await
    .map_err(|e| format!("serve: {e}"))?;
    let url = format!("http://127.0.0.1:{port}");

    let client_v3 = HttpWsClient::new(url).with_envelope(true);
    let mut mux_v3 = client_v3.mux().await.map_err(|e| format!("v3 mux: {e}"))?;

    let d = client_v3
        .unary("host.describe", serde_json::json!({}))
        .await
        .map_err(|e| format!("v3 describe: {e}"))?;
    if !d.ok {
        return Err(format!("v3 describe failed: {:?}", d.error));
    }

    let session = "s-parity";
    let r = client_v3
        .unary(
            "subscribe",
            serde_json::json!({"session_id": session, "last_seq": 0}),
        )
        .await
        .map_err(|e| format!("subscribe: {e}"))?;
    if !r.ok {
        return Err(format!("subscribe failed: {:?}", r.error));
    }

    let injected = vec![
        xylitol_protocol::Event::TextDelta {
            text: "parity chunk".into(),
        },
        xylitol_protocol::Event::ToolStart {
            id: "t-parity".into(),
            name: "grep".into(),
            args: serde_json::json!({"pattern": "TODO"}),
        },
        xylitol_protocol::Event::TurnEnd { turn_index: 1 },
    ];
    for ev in &injected {
        host.slot(session).await.append_and_push(ev.clone()).await;
    }

    let mut got_v3 = Vec::new();
    while let Ok(Some(Ok(frame))) =
        tokio::time::timeout(std::time::Duration::from_secs(3), mux_v3.next()).await
    {
        if let xylitol_protocol::RpcMessage::ServerRequest {
            method, payload, ..
        } = &frame
            && method == "session/event"
            && let Some(ev) = payload
                .get("event")
                .cloned()
                .and_then(|v| serde_json::from_value::<xylitol_protocol::Event>(v).ok())
        {
            got_v3.push(ev);
            if got_v3.len() >= injected.len() {
                break;
            }
        }
    }
    if got_v3.len() != injected.len() {
        return Err(format!(
            "v3 path collected {} events: {got_v3:?}",
            got_v3.len()
        ));
    }
    Ok(injected.into_iter().zip(got_v3).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn eaddrinuse_fails_without_port_plus_one() {
        let host = crate::wiring::occupancy::host_state_for_test().expect("host");
        let (running, port) = serve(
            ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                sessions_dir: None,
                registration_path: None,
            },
            host,
        )
        .await
        .expect("first bind");
        let occupied = port;
        let host2 = crate::wiring::occupancy::host_state_for_test().expect("host2");
        let second = serve(
            ServerConfig {
                host: "127.0.0.1".into(),
                port: occupied,
                sessions_dir: None,
                registration_path: None,
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
