use rutis::host::{CtxHost, FiberState};
use rutis_loader::{Chain, Layer, Loader, LoaderOptions, LoaderPlugin, Patch};
use xylitol_plugin_sdk::rt::Ctx;

use crate::admit::{Admit, is_reserved_plugin_name};
use crate::bootstrap::{BootstrapError, BootstrapInput, BootstrappedRuntime, bootstrap};
use crate::builtins::official_builtins;
use crate::core;
use crate::external_plugin::ExternalPlugin;
use xylitol_official_plugin_config::types::ExternalPluginConfig;

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
            static_tools: Vec::new(),
            agent_profile: None,
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
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let agent_dir = dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".xylitol");
    boot_with(opts, cwd, agent_dir).await
}

async fn boot_with(
    opts: BootOptions,
    cwd: std::path::PathBuf,
    agent_dir: std::path::PathBuf,
) -> Result<Kernel, BootstrapError> {
    let (ctx, loader) = boot_ctx(&opts).await?;
    let mut bootstrap_input = BootstrapInput::from(&opts);
    bootstrap_input.static_tools = static_tools_from_ctx(&ctx);
    let mut runtime = bootstrap(bootstrap_input)?.into_runtime();
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

fn static_tools_from_ctx(ctx: &Ctx) -> Vec<std::sync::Arc<dyn xylitol_plugin_sdk::XyTool>> {
    let Some(regs) = ctx.get::<xylitol_plugin_sdk::HostRegistries>() else {
        return Vec::new();
    };
    regs.tools
        .snapshot()
        .into_iter()
        .filter(|(name, _)| !xylitol_plugin_sdk::is_mcp_tool_name(name))
        .map(|(_, tool)| tool)
        .collect()
}

pub async fn boot_ctx(opts: &BootOptions) -> Result<(Ctx, Loader), BootstrapError> {
    let ctx = Ctx::root().map_err(cordis_to_bootstrap)?;
    mount_core(&ctx, opts).await?;

    let mut builtins = official_builtins();
    let external_specs = external_plugin_specs(&ctx);
    for spec in &external_specs {
        builtins.register(spec.name.clone(), ExternalPlugin::new);
    }
    let plugin = LoaderPlugin::new(
        Chain::new().with(Admit { inner: builtins }),
        LoaderOptions::default(),
    );
    let loader = plugin.handle();
    let loader_view = ctx.plugin(plugin);
    (&loader_view).await.map_err(cordis_to_bootstrap)?;

    let rows = default_rows(opts, &ctx, &external_specs);
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

fn default_rows(
    opts: &BootOptions,
    ctx: &Ctx,
    external_specs: &[ExternalPluginConfig],
) -> Vec<Patch> {
    let config = ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>();
    let mcp_servers = config
        .as_ref()
        .and_then(|cfg| cfg.app.mcp_servers.clone())
        .unwrap_or_default();
    let mut insert = base_rows(opts, mcp_servers);
    #[cfg(feature = "fff")]
    insert.push(serde_json::json!({ "id": "tools-fff", "name": "tools-fff" }));
    insert.extend(optional_plugin_rows(config.as_deref()));
    for spec in external_specs {
        insert.push(serde_json::json!({
            "id": format!("external:{}", spec.name),
            "name": spec.name,
            "config": spec,
        }));
    }
    insert.push(profile_row(opts.profile));
    serde_json::from_value(serde_json::json!([{ "insert": insert }])).unwrap_or_default()
}

fn external_plugin_specs(ctx: &Ctx) -> Vec<ExternalPluginConfig> {
    let Some(config) = ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>() else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    let mut specs = Vec::new();
    for spec in &config.app.external_plugins {
        if !spec.trusted {
            log::warn!(
                "external plugin `{}`: not trusted (`trusted: true` required); skipped",
                spec.name
            );
            continue;
        }
        if spec.name.trim().is_empty() || spec.command.trim().is_empty() {
            log::warn!("external plugin: name and command are required; skipped");
            continue;
        }
        if is_reserved_plugin_name(&spec.name) {
            log::warn!(
                "external plugin `{}`: name collides with a built-in plugin; skipped",
                spec.name
            );
            continue;
        }
        if !seen.insert(spec.name.clone()) {
            log::warn!("external plugin `{}`: duplicate name; skipped", spec.name);
            continue;
        }
        specs.push(spec.clone());
    }
    specs
}

fn base_rows(
    opts: &BootOptions,
    mcp_servers: Vec<xylitol_official_plugin_config::types::McpServerConfig>,
) -> Vec<serde_json::Value> {
    vec![
        session_row(opts),
        serde_json::json!({ "id": "compaction", "name": "compaction" }),
        serde_json::json!({ "id": "tools-builtin", "name": "tools-builtin" }),
        serde_json::json!({ "id": "mcp", "name": "mcp", "config": { "servers": mcp_servers } }),
        serde_json::json!({ "id": "provider-openai", "name": "provider-openai" }),
        serde_json::json!({ "id": "provider-anthropic", "name": "provider-anthropic" }),
    ]
}

fn session_row(opts: &BootOptions) -> serde_json::Value {
    if opts.ephemeral_sessions {
        serde_json::json!({
            "id": "session-store",
            "name": "session-store",
            "config": { "in_memory": true }
        })
    } else {
        serde_json::json!({ "id": "session-store", "name": "session-store" })
    }
}

fn optional_plugin_rows(
    config: Option<&xylitol_official_plugin_config::plugin::ConfigService>,
) -> Vec<serde_json::Value> {
    let Some(config) = config else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    if config.app.loop_policies.is_active() {
        rows.push(serde_json::json!({
            "id": "loop-policies",
            "name": "loop-policies",
            "config": config.app.loop_policies,
        }));
    }
    if config.app.todo.is_active() {
        rows.push(serde_json::json!({
            "id": "todo",
            "name": "todo",
            "config": config.app.todo,
        }));
    }
    if config.app.workflow.is_active() {
        rows.push(serde_json::json!({
            "id": "workflow",
            "name": "workflow",
            "config": config.app.workflow,
        }));
    }
    if config.app.plan.is_active() {
        rows.push(serde_json::json!({
            "id": "plan",
            "name": "plan",
            "config": config.app.plan,
        }));
    }
    rows
}

fn profile_row(profile: Profile) -> serde_json::Value {
    match profile {
        Profile::Print | Profile::Embed => {
            serde_json::json!({ "id": "app-print", "name": "app-print" })
        }
        Profile::Host => serde_json::json!({ "id": "serve", "name": "serve" }),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn external_fixture() -> &'static str {
        r#"
import json, sys
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    request = json.loads(line)
    if request.get("method") == "initialize":
        result = {
            "protocol_version": 1,
            "name": "fixture",
            "version": "0.1.0",
            "capabilities": ["tools"],
            "tools": [{"name": "echo", "description": "Echo", "input_schema": {"type": "object"}}],
        }
    elif request.get("method") == "tools/call":
        result = {"content": str(request.get("params", {}).get("arguments", {}).get("text", ""))}
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32601, "message": "unknown"}}), flush=True)
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#
    }

    fn boot_opts(config_path: std::path::PathBuf) -> BootOptions {
        BootOptions {
            profile: Profile::Print,
            config_path: Some(config_path),
            session: None,
            model: None,
            trust_override: Some(false),
            interactive: false,
            caller: "contract",
            ephemeral_sessions: true,
        }
    }

    fn write_external_config(
        dir: &tempfile::TempDir,
        trusted: bool,
        name: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let script = dir.path().join("external_fixture.py");
        std::fs::write(&script, external_fixture()).expect("fixture");
        let config_path = dir.path().join("config.yaml");
        let config = format!(
            "models:\n  default_model: q\n  models:\n    q:\n      provider: openai\n      model: m\nexternal_plugins:\n  - name: \"{name}\"\n    command: \"python3\"\n    args: [\"-u\", \"{}\"]\n    trusted: {trusted}\n    timeout_ms: 5000\n",
            script.to_string_lossy()
        );
        std::fs::write(&config_path, config).expect("config");
        (config_path, script)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn boot_ctx_registers_trusted_external_tools() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config_path, _script) = write_external_config(&dir, true, "fixture");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(
            registries.tools.get("fixture__echo").is_some(),
            "trusted external plugin tool must be registered"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn boot_ctx_skips_untrusted_external_plugins() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config_path, _script) = write_external_config(&dir, false, "fixture");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(
            registries.tools.get("fixture__echo").is_none(),
            "untrusted external plugin must not be assembled"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn boot_ctx_skips_external_plugin_with_reserved_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config_path, _script) = write_external_config(&dir, true, "tools-builtin");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(
            registries.tools.get("tools-builtin__echo").is_none(),
            "reserved external plugin name must not be registered"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn boot_with_self_check_print_profile() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        std::fs::write(
            &config_path,
            "models:\n  default_model: q\n  models:\n    q:\n      provider: openai\n      model: m\n",
        )
        .unwrap();
        let opts = BootOptions {
            profile: Profile::Print,
            config_path: Some(config_path),
            session: None,
            model: None,
            trust_override: Some(false),
            interactive: false,
            caller: "contract",
            ephemeral_sessions: true,
        };
        let kernel = boot_with(opts, dir.path().to_path_buf(), dir.path().join(".xylitol"))
            .await
            .expect("boot_with");
        assert_eq!(kernel.profile, Profile::Print);
        let _ = kernel.ctx.shutdown().await;
    }
}
