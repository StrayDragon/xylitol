//! `Kernel::boot`: one assembly path for Print / Host / embed.

use rutis_loader::{Chain, Layer, Loader, LoaderOptions, LoaderPlugin, Patch};
use xylitol_plugin_sdk::rt::{Ctx, FiberState, Typed};

use crate::admit::Admit;
use crate::bootstrap::{BootstrapError, BootstrapInput, BootstrappedRuntime, bootstrap};
use crate::builtins::official_builtins;
use crate::core;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Print,
    Host,
    Embed,
}

pub struct BootOptions {
    pub profile: Profile,
    pub config_path: Option<std::path::PathBuf>,
    pub session: Option<String>,
    pub model: Option<String>,
    pub trust_override: Option<bool>,
    pub interactive: bool,
    pub caller: &'static str,
    pub ephemeral_sessions: bool,
}

impl From<&BootOptions> for BootstrapInput {
    fn from(opts: &BootOptions) -> Self {
        BootstrapInput {
            config_path: opts.config_path.clone(),
            session: opts.session.clone(),
            model: opts.model.clone(),
            trust_override: opts.trust_override,
            interactive: opts.interactive,
            caller: opts.caller,
            ephemeral_sessions: opts.ephemeral_sessions,
        }
    }
}

pub struct Kernel {
    pub ctx: Ctx,
    pub loader: Loader,
    pub runtime: BootstrappedRuntime,
    pub profile: Profile,
}

pub async fn boot(opts: BootOptions) -> Result<Kernel, BootstrapError> {
    let (ctx, loader) = boot_ctx(&opts).await?;
    let runtime = bootstrap(BootstrapInput::from(&opts))?.into_runtime();
    Ok(Kernel {
        ctx,
        loader,
        runtime,
        profile: opts.profile,
    })
}

/// Mount core plugins and loader rows without assembling an agent.
pub async fn boot_ctx(opts: &BootOptions) -> Result<(Ctx, Loader), BootstrapError> {
    let ctx = Ctx::root().map_err(cordis_to_bootstrap)?;
    mount_core(&ctx, opts).await?;

    let builtins = official_builtins();
    let plugin = LoaderPlugin::new(
        Chain::new().with(Admit { inner: builtins }),
        LoaderOptions::default(),
    );
    let loader = plugin.handle();
    let loader_view = ctx.plugin(plugin);
    (&loader_view).await.map_err(cordis_to_bootstrap)?;

    let rows = default_rows(opts.profile);
    loader
        .reconcile(vec![Layer::new("base", rows)], None)
        .await
        .map_err(|e| kernel_failed(format!("loader reconcile: {e}")))?;

    wait_active(&ctx).await?;
    Ok((ctx, loader))
}

async fn mount_core(ctx: &Ctx, opts: &BootOptions) -> Result<(), BootstrapError> {
    let config = ctx.plugin(xylitol_official_plugin_config::plugin::ConfigPlugin {
        config_path: opts.config_path.clone(),
    });
    (&config).await.map_err(cordis_to_bootstrap)?;

    let registries = ctx.plugin(core::registries::RegistriesPlugin);
    (&registries).await.map_err(cordis_to_bootstrap)?;

    let obs = ctx.plugin(Typed::new(xylitol_official_plugin_obs::plugin::ObsPlugin));
    (&obs).await.map_err(cordis_to_bootstrap)?;

    let trust = ctx.plugin(Typed::new(core::trust::TrustPlugin));
    (&trust).await.map_err(cordis_to_bootstrap)?;

    let session_host = ctx.plugin(core::session_host::SessionHostPlugin);
    (&session_host).await.map_err(cordis_to_bootstrap)?;

    let agent = ctx.plugin(core::agent_react::AgentReactPlugin);
    (&agent).await.map_err(cordis_to_bootstrap)?;

    let tools = ctx.plugin(core::tool_table::ToolTablePlugin);
    (&tools).await.map_err(cordis_to_bootstrap)?;

    let driver = ctx.plugin(core::driver::DriverPlugin);
    (&driver).await.map_err(cordis_to_bootstrap)?;
    Ok(())
}

fn default_rows(profile: Profile) -> Vec<Patch> {
    let mut insert = vec![
        serde_json::json!({ "id": "session-store", "name": "session-store" }),
        serde_json::json!({ "id": "compaction", "name": "compaction" }),
        serde_json::json!({ "id": "tools-builtin", "name": "tools-builtin" }),
        serde_json::json!({ "id": "provider-openai", "name": "provider-openai" }),
        serde_json::json!({ "id": "provider-anthropic", "name": "provider-anthropic" }),
    ];
    match profile {
        Profile::Print | Profile::Embed => {
            insert.push(serde_json::json!({ "id": "app-print", "name": "app-print" }));
        }
        Profile::Host => {
            insert.push(serde_json::json!({ "id": "serve", "name": "serve" }));
        }
    }
    serde_json::from_value(serde_json::json!([{ "insert": insert }])).unwrap_or_default()
}

async fn wait_active(ctx: &Ctx) -> Result<(), BootstrapError> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let diag = ctx.diagnostics();
        let failed: Vec<String> = diag
            .plugins
            .iter()
            .filter(|p| matches!(p.state, FiberState::Failed | FiberState::Disposed))
            .map(|p| {
                let err = p
                    .error
                    .as_ref()
                    .map(|e| format!(":{e}"))
                    .unwrap_or_default();
                format!("{}:{:?}{err}", p.name, p.state)
            })
            .collect();
        if !failed.is_empty() {
            return Err(kernel_failed(format!(
                "kernel self-check failed: {}",
                failed.join(", ")
            )));
        }
        let unstable = diag
            .plugins
            .iter()
            .any(|p| matches!(p.state, FiberState::Pending | FiberState::Loading));
        if !unstable {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            let names: Vec<String> = diag
                .plugins
                .iter()
                .filter(|p| !matches!(p.state, FiberState::Active))
                .map(|p| format!("{}:{:?}", p.name, p.state))
                .collect();
            return Err(kernel_failed(format!(
                "kernel self-check timed out: {}",
                names.join(", ")
            )));
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

fn cordis_to_bootstrap(err: impl std::fmt::Display) -> BootstrapError {
    kernel_failed(err.to_string())
}

fn kernel_failed(msg: impl Into<String>) -> BootstrapError {
    BootstrapError::KernelFailed(msg.into())
}
