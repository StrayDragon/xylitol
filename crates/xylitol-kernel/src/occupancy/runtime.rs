use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use xylitol_serve::{Gateway, ListenConfig, OccupiedHost, RunningListener};

use crate::boot::{BootOptions, Profile, boot_ctx};
use crate::bootstrap::{BootstrapError, BootstrapInput, resolve_assembly};
use crate::composition::build_baseline;
use crate::occupancy::factory::ProductOccupancyFactory;
use crate::occupancy::host::{HostState, ReloadBaseline};

pub type RunningServer = RunningListener;

pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub sessions_dir: Option<std::path::PathBuf>,
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
            tokio::time::sleep(Duration::from_secs(10)).await;
            drop(running);
            Err(e)
        }
    }
}

async fn assemble(config: &ServerConfig) -> Result<Arc<HostState>, Box<dyn std::error::Error>> {
    let boot_opts = BootOptions {
        profile: Profile::Host,
        config_path: None,
        session: None,
        model: None,
        trust_override: None,
        interactive: false,
        caller: "server",
        ephemeral_sessions: false,
    };
    let (ctx, loader) = boot_ctx(&boot_opts).await?;

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
        BootstrapError::KernelFailed(msg) => format!("kernel boot failed: {msg}"),
    })?;

    let fallback_session = assembly.session_id.clone();
    let mcp_servers = assembly.mcp_servers.clone().unwrap_or_default();
    let default_model_id = assembly.default_profile_model.clone();
    let project_trusted = !assembly.warnings.iter().any(|w| {
        matches!(
            w,
            crate::bootstrap::BootstrapWarning::ProjectNotTrusted { .. }
        )
    });
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let agent_dir = xylitol_runtime::resource::DefaultResourceLoader::default_agent_dir();
    let ports = if let Some(dir) = &config.sessions_dir {
        let store: std::sync::Arc<dyn xylitol_plugin_sdk::XySessionStore> = std::sync::Arc::new(
            xylitol_official_plugin_session::SessionManager::new(dir.clone()),
        );
        crate::composition::build_baseline_with_store(assembly.into_build_options(), store)?
    } else {
        build_baseline(assembly.into_build_options())?
    };

    let factory = Arc::new(
        ProductOccupancyFactory::new(
            ports.clone(),
            cwd.clone(),
            agent_dir.clone(),
            project_trusted,
            mcp_servers,
            default_model_id.clone(),
        )
        .with_prepare_ctx(ctx, loader),
    );
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

#[allow(dead_code)]
pub async fn serve(
    config: ServerConfig,
    host: Arc<HostState>,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    xylitol_serve::serve(config.listen(), occupy(host)).await
}

#[allow(dead_code)]
pub async fn serve_registered(
    config: ServerConfig,
    host: Arc<HostState>,
    registration_path: PathBuf,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    xylitol_serve::serve_registered(config.listen(), occupy(host), registration_path).await
}

pub async fn bind_serve(
    config: &ServerConfig,
    gateway: Arc<Gateway>,
) -> Result<(RunningServer, u16), Box<dyn std::error::Error>> {
    xylitol_serve::bind_listen(&config.listen(), gateway).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn eaddrinuse_fails_without_port_plus_one() {
        let host = crate::occupancy::host_state_for_test().expect("host");
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
        let host2 = crate::occupancy::host_state_for_test().expect("host2");
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
