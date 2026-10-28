use rutis::host::{CtxHost, FiberState};
use rutis_loader::{Chain, Layer, Loader, LoaderOptions, LoaderPlugin, Patch};
use xylitol_plugin_sdk::rt::Ctx;

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
    let mut runtime = bootstrap(BootstrapInput::from(&opts))?.into_runtime();
    runtime.driver.attach_ctx(ctx.clone());
    ctx.provide(xylitol_plugin_sdk::RuntimeSessionStore(
        runtime.driver.store.clone(),
    ))
    .map_err(cordis_to_bootstrap)?;
    if let Some(hub) = ctx.get::<dyn xylitol_plugin_sdk::SessionPrepareHub>() {
        runtime.driver.set_prepare_hub(hub);
    }
    if let Some(registry) = ctx.get::<xylitol_plugin_sdk::LoopMiddlewareRegistry>() {
        runtime.driver.set_loop_middleware(Some(registry));
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let agent_dir = dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".xylitol");
    runtime
        .driver
        .enable_reload_state(cwd, agent_dir, opts.trust_override.unwrap_or(true));
    Ok(Kernel {
        ctx,
        loader,
        runtime,
        profile: opts.profile,
    })
}

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

    let rows = default_rows(opts, &ctx);
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

    let obs = ctx.plugin(xylitol_official_plugin_obs::plugin::ObsPlugin);
    (&obs).await.map_err(cordis_to_bootstrap)?;

    let trust = ctx.plugin(core::trust::TrustPlugin);
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

fn default_rows(opts: &BootOptions, ctx: &Ctx) -> Vec<Patch> {
    let session_row = if opts.ephemeral_sessions {
        serde_json::json!({
            "id": "session-store",
            "name": "session-store",
            "config": { "in_memory": true }
        })
    } else {
        serde_json::json!({ "id": "session-store", "name": "session-store" })
    };
    let mcp_servers = ctx
        .get::<xylitol_official_plugin_config::plugin::ConfigService>()
        .and_then(|cfg| cfg.app.mcp_servers.clone())
        .unwrap_or_default();
    let config = ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>();
    let loop_policies = config
        .as_ref()
        .map(|cfg| cfg.app.loop_policies.clone())
        .unwrap_or_default();
    let todo = config
        .as_ref()
        .map(|cfg| cfg.app.todo.clone())
        .unwrap_or_default();
    let workflow = config
        .as_ref()
        .map(|cfg| cfg.app.workflow.clone())
        .unwrap_or_default();
    let plan = config
        .as_ref()
        .map(|cfg| cfg.app.plan.clone())
        .unwrap_or_default();
    let mut insert = vec![
        session_row,
        serde_json::json!({ "id": "compaction", "name": "compaction" }),
        serde_json::json!({ "id": "tools-builtin", "name": "tools-builtin" }),
        serde_json::json!({ "id": "mcp", "name": "mcp", "config": { "servers": mcp_servers } }),
        serde_json::json!({ "id": "provider-openai", "name": "provider-openai" }),
        serde_json::json!({ "id": "provider-anthropic", "name": "provider-anthropic" }),
    ];
    if loop_policies.is_active() {
        insert.push(serde_json::json!({
            "id": "loop-policies",
            "name": "loop-policies",
            "config": loop_policies,
        }));
    }
    if todo.is_active() {
        insert.push(serde_json::json!({
            "id": "todo",
            "name": "todo",
            "config": todo,
        }));
    }
    if workflow.is_active() {
        insert.push(serde_json::json!({
            "id": "workflow",
            "name": "workflow",
            "config": workflow,
        }));
    }
    if plan.is_active() {
        insert.push(serde_json::json!({
            "id": "plan",
            "name": "plan",
            "config": plan,
        }));
    }
    match opts.profile {
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
