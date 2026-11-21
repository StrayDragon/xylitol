use std::sync::Arc;

use rutis::host::{CtxHost, ErrorKind, ErrorSink, FiberState};
pub use rutis_loader::report::{Wait, WaitReason};
use rutis_loader::{Builtins, Chain, Layer, Loader, LoaderOptions, LoaderPlugin, Patch};
use xylitol_plugin_sdk::config_section::SectionBinding;
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

#[derive(Clone)]
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
    crate::driver::attach_ctx_services(&mut runtime.driver, &ctx);
    if let Some(regs) = ctx.get::<xylitol_plugin_sdk::HostRegistries>() {
        runtime
            .driver
            .set_plugin_commands(regs.plugin_commands.snapshot());
    }
    ctx.provide(xylitol_plugin_sdk::RuntimeSessionStore(
        runtime.driver.store.clone(),
    ))
    .map_err(cordis_to_bootstrap)?;
    runtime
        .driver
        .enable_reload_state(cwd, agent_dir, opts.trust_override.unwrap_or(true));
    runtime
        .driver
        .set_reload_config_path(opts.config_path.clone());
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
    let handle = tokio::runtime::Handle::try_current()
        .map_err(|_| kernel_failed("rutis root requires a tokio runtime"))?;
    let ctx = Ctx::root_with_sink(handle, rutis_error_sink());
    mount_core(&ctx, opts).await?;
    crate::subagent::register_subagent_tool(&ctx, opts);

    let mut builtins = official_builtins();
    let external_specs = external_plugin_specs(&ctx);
    register_external_builtins(&mut builtins, &external_specs);
    let plugin = LoaderPlugin::new(
        Chain::new().with(Admit { inner: builtins }),
        LoaderOptions::default(),
    );
    let loader = plugin.handle();
    let loader_view = ctx.plugin(plugin);
    loader_view.await.map_err(cordis_to_bootstrap)?;

    let rows = default_rows(opts, &ctx, &external_specs);
    loader
        .reconcile(vec![Layer::new("base", rows)], None)
        .await
        .map_err(|e| kernel_failed(format!("loader reconcile: {e}")))?;

    wait_active(&ctx, &loader).await?;
    if let (Some(registry), Some(store)) = (
        ctx.get::<crate::external_plugin::ExternalMcpRegistry>(),
        crate::external_plugin::approval_store_path(),
    ) {
        for name in crate::external_plugin::load_approved_plugins(&store) {
            registry.approve(&name);
        }
    }
    let external_mcp = ctx
        .get::<crate::external_plugin::ExternalMcpRegistry>()
        .map(|registry| {
            let approved = registry.servers();
            let declared = registry.declared();
            let pending: Vec<String> = declared
                .iter()
                .filter(|server| approved.iter().all(|a| a.name != server.name))
                .map(|server| server.name.clone())
                .collect();
            if !pending.is_empty() {
                log::warn!(
                    target: "xylitol::plugin",
                    "external plugins declaring mcp are awaiting approval: {}",
                    pending.join(", ")
                );
            }
            approved
        })
        .unwrap_or_default();
    if !external_mcp.is_empty() {
        let servers = serde_json::to_value(&external_mcp).unwrap_or(serde_json::Value::Null);
        let patch = Patch {
            id: Some("mcp".into()),
            insert: None,
            name: None,
            overrides: serde_json::json!({ "config": { "servers": servers } })
                .as_object()
                .cloned()
                .unwrap_or_default(),
        };
        loader
            .reconcile(vec![Layer::new("external-mcp", vec![patch])], None)
            .await
            .map_err(|e| kernel_failed(format!("external mcp reconcile: {e}")))?;
        wait_active(&ctx, &loader).await?;
    }
    if let Some(config) = ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>() {
        let external_keys = config
            .external_config_keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        config
            .app
            .validate_unknown_keys(&external_keys, &crate::config_sections::all())
            .map_err(|error| kernel_failed(error.to_string()))?;
        if config.app.xylitol.unknown_keys
            == xylitol_official_plugin_config::types::UnknownKeyPolicy::Warn
            && let Some(message) = config
                .app
                .unknown_keys_warning(&external_keys, &crate::config_sections::all())
        {
            log::warn!(target: "xylitol::config", "{message}");
        }
    }
    Ok((ctx, loader))
}

fn rutis_error_sink() -> ErrorSink {
    Arc::new(|error: Arc<ErrorKind>| match &*error {
        ErrorKind::NotServedByName { service } => {
            log::debug!(
                target: "xylitol::rutis",
                "service {service} is provided by type but not registered by name \
                 (rutis-bridge is not linked; name serving is unused)"
            );
        }
        _ => log::error!(target: "xylitol::rutis", "{error}"),
    })
}

async fn mount_core(ctx: &Ctx, opts: &BootOptions) -> Result<(), BootstrapError> {
    let config = ctx.plugin(xylitol_official_plugin_config::plugin::ConfigPlugin {
        config_path: opts.config_path.clone(),
        sections: crate::config_sections::all(),
    });
    config.await.map_err(cordis_to_bootstrap)?;

    let registries = ctx.plugin(core::registries::RegistriesPlugin);
    registries.await.map_err(cordis_to_bootstrap)?;

    let obs = ctx.plugin(xylitol_official_plugin_obs::plugin::ObsPlugin);
    obs.await.map_err(cordis_to_bootstrap)?;

    let trust = ctx.plugin(core::trust::TrustPlugin);
    trust.await.map_err(cordis_to_bootstrap)?;

    let agent = ctx.plugin(core::agent_react::AgentReactPlugin);
    agent.await.map_err(cordis_to_bootstrap)?;
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
        .and_then(|cfg| {
            xylitol_official_plugin_mcp::McpServersSection::try_from_sections(&cfg.app.sections)
                .ok()
                .flatten()
                .map(|servers| servers.0)
        })
        .unwrap_or_default();
    let mut insert = base_rows(opts, mcp_servers);
    let context_inject = config
        .as_ref()
        .and_then(|cfg| {
            xylitol_official_plugin_context_inject::config::ContextInjectConfig::try_from_sections(
                &cfg.app.sections,
            )
            .ok()
            .flatten()
        })
        .unwrap_or_default();
    insert.push(serde_json::json!({
        "id": "context-inject",
        "name": "context-inject",
        "config": context_inject,
    }));
    #[cfg(feature = "fff")]
    insert.push(serde_json::json!({ "id": "tools-fff", "name": "tools-fff" }));
    insert.extend(optional_plugin_rows(config.as_deref()));
    insert.extend(external_rows(external_specs));
    serde_json::from_value(serde_json::json!([{ "insert": insert }])).unwrap_or_default()
}

struct ExternalPluginFactory {
    name: String,
    bin_dir: std::path::PathBuf,
}

impl rutis::host::PluginFactory<serde_json::Value> for ExternalPluginFactory {
    fn name(&self) -> &str {
        &self.name
    }

    fn build(
        &self,
        _config: &serde_json::Value,
    ) -> Result<Box<dyn rutis::host::DynPlugin>, rutis::Error> {
        Ok(Box::new(ExternalPlugin::discovered_at(
            &self.name,
            &self.bin_dir,
        )))
    }
}

fn register_external_builtins(builtins: &mut Builtins, specs: &[ExternalPluginConfig]) {
    let Some(bin_dir) = plugin_bin_dir() else {
        return;
    };
    for spec in specs {
        builtins.register_raw(
            spec.name.clone(),
            ExternalPluginFactory {
                name: spec.name.clone(),
                bin_dir: bin_dir.clone(),
            },
            None,
        );
    }
}

fn external_rows(specs: &[ExternalPluginConfig]) -> Vec<serde_json::Value> {
    specs
        .iter()
        .map(|spec| {
            serde_json::json!({
                "id": format!("external:{}", spec.name),
                "name": spec.name,
                "config": spec,
            })
        })
        .collect()
}

fn plugins_setting(ctx: &Ctx) -> xylitol_official_plugin_config::types::PluginsConfig {
    ctx.get::<xylitol_official_plugin_config::plugin::ConfigService>()
        .map(|config| {
            xylitol_official_plugin_config::types::PluginsConfig::try_from_sections(
                &config.app.sections,
            )
            .ok()
            .flatten()
            .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn discovered_plugin_specs(
    settings: &xylitol_official_plugin_config::types::PluginsConfig,
) -> Vec<ExternalPluginConfig> {
    let Some(plugins_dir) = plugin_bin_dir() else {
        return Vec::new();
    };
    let mut specs = Vec::new();
    let Ok(entries) = std::fs::read_dir(&plugins_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<std::path::PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    for path in paths {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if is_reserved_plugin_name(name) {
            log::warn!("external plugin `{name}`: name collides with a built-in plugin; skipped");
            continue;
        }
        if !settings.is_enabled(name) {
            log::info!("external plugin `{name}`: disabled by config; skipped");
            continue;
        }
        specs.push(ExternalPluginConfig {
            name: name.to_string(),
            enabled: true,
            disabled: false,
        });
    }
    specs
}

fn plugin_bin_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("XYLITOL_PLUGINS_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                std::path::PathBuf::from(home)
                    .join(".xylitol")
                    .join("plugins")
                    .join("bin")
            })
        })
}

fn external_plugin_specs(ctx: &Ctx) -> Vec<ExternalPluginConfig> {
    let settings = plugins_setting(ctx);
    discovered_plugin_specs(&settings)
}

fn base_rows(
    opts: &BootOptions,
    mcp_servers: Vec<xylitol_official_plugin_mcp::McpServerConfig>,
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
    let loop_policies =
        xylitol_official_plugin_loop_policies::LoopPoliciesConfig::try_from_sections(
            &config.app.sections,
        )
        .unwrap_or_default()
        .unwrap_or_default();
    if loop_policies.is_active() {
        rows.push(serde_json::json!({
            "id": "loop-policies",
            "name": "loop-policies",
            "config": loop_policies,
        }));
    }
    let todo = xylitol_official_plugin_todo::TodoConfig::try_from_sections(&config.app.sections)
        .unwrap_or_default()
        .unwrap_or_default();
    if todo.is_active() {
        rows.push(serde_json::json!({
            "id": "todo",
            "name": "todo",
            "config": todo,
        }));
    }
    let workflow =
        xylitol_official_plugin_workflow::WorkflowConfig::try_from_sections(&config.app.sections)
            .unwrap_or_default()
            .unwrap_or_default();
    if workflow.is_active() {
        rows.push(serde_json::json!({
            "id": "workflow",
            "name": "workflow",
            "config": workflow,
        }));
    }
    let mut codemode = xylitol_plugin_sdk::CodemodeConfig::try_from_sections(&config.app.sections)
        .unwrap_or_default()
        .unwrap_or_default();
    if loop_policies.tool_exposure.enabled {
        codemode.tool_exposure = Some(xylitol_plugin_sdk::CodemodeExposurePolicy {
            allow: loop_policies.tool_exposure.allow.clone(),
            deny: loop_policies.tool_exposure.deny.clone(),
            read_only: loop_policies.tool_exposure.read_only,
            write_tools: loop_policies.tool_exposure.write_tools.clone(),
        });
    }
    if codemode.enabled {
        rows.push(serde_json::json!({
            "id": "codemode",
            "name": "codemode",
            "config": codemode,
        }));
    }
    rows.extend(lsp_dap_rows(config));
    rows
}

fn lsp_dap_rows(
    config: &xylitol_official_plugin_config::plugin::ConfigService,
) -> Vec<serde_json::Value> {
    let mut rows = Vec::new();
    let lsp = xylitol_official_plugin_lsp::LspPluginConfig::try_from_sections(&config.app.sections)
        .ok()
        .flatten();
    if let Some(lsp) = lsp.as_ref() {
        rows.push(serde_json::json!({
            "id": "lsp",
            "name": "lsp",
            "config": lsp,
        }));
    }
    let dap = xylitol_official_plugin_dap::DapPluginConfig::try_from_sections(&config.app.sections)
        .ok()
        .flatten();
    if let Some(dap) = dap.as_ref() {
        rows.push(serde_json::json!({
            "id": "dap",
            "name": "dap",
            "config": dap,
        }));
    }
    rows
}

async fn wait_active(ctx: &Ctx, loader: &Loader) -> Result<(), BootstrapError> {
    wait_active_within(ctx, loader, std::time::Duration::from_secs(2)).await
}

async fn wait_active_within(
    ctx: &Ctx,
    loader: &Loader,
    timeout: std::time::Duration,
) -> Result<(), BootstrapError> {
    let deadline = tokio::time::Instant::now() + timeout;
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
            let names = describe_unready(ctx, loader);
            return Err(kernel_failed(format!(
                "kernel self-check timed out: {}",
                names.join(", ")
            )));
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

fn describe_unready(ctx: &Ctx, loader: &Loader) -> Vec<String> {
    let waiting = describe_waits(loader);
    if !waiting.is_empty() {
        return waiting;
    }
    ctx.diagnostics()
        .plugins
        .iter()
        .filter(|p| !matches!(p.state, FiberState::Active))
        .map(|p| format!("{}:{:?}", p.name, p.state))
        .collect()
}

fn describe_waits(loader: &Loader) -> Vec<String> {
    loader
        .entries()
        .iter()
        .filter(|entry| entry.plugin.is_some())
        .filter_map(|entry| {
            let waits = loader.waits(entry);
            if waits.is_empty() {
                return None;
            }
            let described: Vec<String> = waits.iter().map(describe_wait).collect();
            Some(format!("{}: {}", entry.id, described.join(", ")))
        })
        .collect()
}

pub fn describe_wait(wait: &Wait) -> String {
    let reason = match &wait.reason {
        WaitReason::OwnRuntime => "its own runtime has not released its rows".to_string(),
        WaitReason::NoProvider { like: Some(like) } => {
            format!("no row provides it; did you mean `{like}`?")
        }
        WaitReason::NoProvider { like: None } => "no row provides it".to_string(),
        WaitReason::NoEnabledProvider { like: Some(like) } => {
            format!("no enabled row provides it; disabled rows may (did you mean `{like}`?)")
        }
        WaitReason::NoEnabledProvider { like: None } => {
            "no enabled row provides it; disabled rows may".to_string()
        }
        WaitReason::Explained(text) => text.clone(),
        WaitReason::ProviderDisabled { row } => {
            format!("the row `{row}` providing it is disabled")
        }
        WaitReason::ProviderFailed { row, error } => {
            format!("the row `{row}` providing it failed: {error}")
        }
        WaitReason::ProviderInactive { row } => {
            format!("the row `{row}` providing it is not running")
        }
        WaitReason::ProviderRuntime { row, runtime } => {
            format!("the row `{row}` providing it waits for runtime `{runtime}`")
        }
        WaitReason::Cycle { row, service } => {
            format!("the row `{row}` providing it waits for `{service}`, which this row provides")
        }
        WaitReason::ProviderWaiting { row, services } => {
            format!(
                "the row `{row}` providing it also waits for {}",
                services.join(", ")
            )
        }
        WaitReason::ProviderStarting { row } => {
            format!("the row `{row}` providing it is starting")
        }
        WaitReason::ProviderRunning { row } => {
            format!("the row `{row}` providing it runs but has not provided it")
        }
        WaitReason::Other(text) => text.clone(),
        _ => "no reason given".to_string(),
    };
    format!("{} ({reason})", wait.service)
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
        r#"#!/usr/bin/env python3
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
            "capabilities": ["tools", "commands"],
            "tools": [{"name": "echo", "description": "Echo", "input_schema": {"type": "object"}}],
            "commands": [{"name": "status", "description": "Fixture status", "argument_hint": "refresh"}],
        }
    elif request.get("method") == "tools/call":
        result = {"content": str(request.get("params", {}).get("arguments", {}).get("text", ""))}
    elif request.get("method") == "commands/call":
        result = {"text": "fixture:" + str(request.get("params", {}).get("args", ""))}
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32601, "message": "unknown"}}), flush=True)
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#
    }

    fn boot_opts(config_path: std::path::PathBuf) -> BootOptions {
        if let Some(parent) = config_path.parent() {
            unsafe {
                std::env::set_var("XYLITOL_CONFIG_DIR", parent.join(".config").join("xylitol"));
                std::env::set_var("HOME", parent);
                std::env::set_var("XYLITOL_PLUGINS_DIR", parent.join("plugins").join("bin"));
            }
        }
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
        enabled: bool,
        name: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let script = dir.path().join("external_fixture.py");
        std::fs::write(&script, external_fixture()).expect("fixture");
        let bin_dir = dir.path().join("plugins").join("bin");
        std::fs::create_dir_all(&bin_dir).expect("plugins/bin");
        let installed = bin_dir.join(name);
        std::fs::copy(&script, &installed).expect("install fixture");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&installed).expect("stat").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&installed, perms).expect("chmod");
        }
        let plugins_yaml = if enabled {
            format!("plugins:\n  enabled: [\"{name}\"]\n")
        } else {
            "plugins:\n  enabled: [\"other\"]\n".to_string()
        };
        let config_path = dir.path().join("config.yaml");
        let config = format!(
            "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\n{plugins_yaml}"
        );
        std::fs::write(&config_path, config).expect("config");
        (config_path, installed)
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(env_global)]
    async fn boot_ctx_registers_enabled_external_plugin_commands() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config_path, _installed) = write_external_config(&dir, true, "fixture");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(
            registries.plugin_commands.get("plugin:fixture").is_some(),
            "an enabled discovered plugin must contribute its commands"
        );
        assert!(
            registries.tools.get("fixture__echo").is_none(),
            "a plugin declaring only commands and mcp must not bridge private tools"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn boot_ctx_registers_codemode_when_enabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.yaml");
        std::fs::write(
            &config_path,
            "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\ncodemode:\n  enabled: true\n",
        )
        .expect("config");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(
            registries.tools.get("run_code").is_some(),
            "enabled codemode must register run_code"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(env_global)]
    async fn boot_ctx_registers_external_plugin_commands() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config_path, _installed) = write_external_config(&dir, true, "fixture");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        let command = registries
            .plugin_commands
            .get("plugin:fixture")
            .expect("plugin command must be registered");
        assert_eq!(command.description(), "Fixture status");
        let text = command.invoke("refresh").await.expect("invoke");
        assert_eq!(text, "fixture:refresh");
        let servers = ctx
            .get::<crate::external_plugin::ExternalMcpRegistry>()
            .expect("external mcp registry")
            .servers();
        assert!(
            servers.iter().all(|server| server.name != "fixture"),
            "the fixture declares no mcp, so no server may be registered: {servers:?}"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(env_global)]
    async fn reload_rebuilds_model_registry_from_config() {
        use xylitol_runtime::driver::XyDriver;

        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.yaml");
        std::fs::write(
            &config_path,
            "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\n",
        )
        .expect("config");
        unsafe {
            std::env::set_var("XYLITOL_CONFIG_DIR", dir.path());
            std::env::set_var("HOME", dir.path());
        }
        let opts = BootOptions {
            profile: Profile::Print,
            config_path: Some(config_path.clone()),
            session: None,
            model: None,
            trust_override: Some(false),
            interactive: false,
            caller: "contract",
            ephemeral_sessions: true,
        };
        let mut kernel = boot_with(opts, dir.path().to_path_buf(), dir.path().join(".xylitol"))
            .await
            .expect("boot_with");
        assert_eq!(kernel.runtime.driver.available_models().len(), 1);
        std::fs::write(
            &config_path,
            "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\n    extra:\n      provider: openai\n      model: extra\n",
        )
        .expect("config");
        let cancel = tokio_util::sync::CancellationToken::new();
        let report = kernel
            .runtime
            .driver
            .reload_runtime(&cancel)
            .await
            .expect("reload");
        let models = report
            .steps
            .iter()
            .find(|step| step.step == "models")
            .expect("models step");
        assert!(models.ok, "models step failed: {}", models.message);
        assert_eq!(kernel.runtime.driver.available_models().len(), 2);
        let _ = kernel.ctx.shutdown().await;
        unsafe {
            std::env::remove_var("XYLITOL_CONFIG_DIR");
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(env_global)]
    async fn reload_reports_failed_participant_and_continues() {
        use xylitol_runtime::driver::{ReloadContext, ReloadParticipant, ReloadRegistry, XyDriver};

        struct Failing;

        #[async_trait::async_trait]
        impl ReloadParticipant for Failing {
            fn name(&self) -> &'static str {
                "failing"
            }

            async fn reload(
                &self,
                _driver: &mut xylitol_runtime::XyInProcessDriver,
                _ctx: &ReloadContext,
            ) -> Result<String, String> {
                Err("boom".into())
            }
        }

        struct Succeeding;

        #[async_trait::async_trait]
        impl ReloadParticipant for Succeeding {
            fn name(&self) -> &'static str {
                "succeeding"
            }

            async fn reload(
                &self,
                _driver: &mut xylitol_runtime::XyInProcessDriver,
                _ctx: &ReloadContext,
            ) -> Result<String, String> {
                Ok("done".into())
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.yaml");
        std::fs::write(
            &config_path,
            "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\n",
        )
        .expect("config");
        unsafe {
            std::env::set_var("XYLITOL_CONFIG_DIR", dir.path());
            std::env::set_var("HOME", dir.path());
        }
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
        let mut kernel = boot_with(opts, dir.path().to_path_buf(), dir.path().join(".xylitol"))
            .await
            .expect("boot_with");
        let registry = std::sync::Arc::new(ReloadRegistry::new());
        registry.register(std::sync::Arc::new(Failing));
        registry.register(std::sync::Arc::new(Succeeding));
        kernel.runtime.driver.set_reload_registry(registry);
        let cancel = tokio_util::sync::CancellationToken::new();
        let report = kernel
            .runtime
            .driver
            .reload_runtime(&cancel)
            .await
            .expect("reload");
        let failing = report
            .steps
            .iter()
            .find(|step| step.step == "failing")
            .expect("failing step");
        assert!(!failing.ok);
        assert_eq!(failing.message, "boom");
        let succeeding = report
            .steps
            .iter()
            .find(|step| step.step == "succeeding")
            .expect("succeeding step");
        assert!(succeeding.ok);
        assert_eq!(succeeding.message, "done");
        let _ = kernel.ctx.shutdown().await;
        unsafe {
            std::env::remove_var("XYLITOL_CONFIG_DIR");
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(env_global)]
    async fn reload_runs_plugin_hub_participants() {
        use xylitol_runtime::driver::XyDriver;

        struct PluginStep;

        #[async_trait::async_trait]
        impl xylitol_plugin_sdk::ReloadParticipant for PluginStep {
            fn name(&self) -> &'static str {
                "plugin"
            }

            async fn reload(
                &self,
                _event: &xylitol_plugin_sdk::ReloadEvent,
            ) -> Result<String, String> {
                Ok("plugin done".into())
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.yaml");
        std::fs::write(
            &config_path,
            "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\n",
        )
        .expect("config");
        unsafe {
            std::env::set_var("XYLITOL_CONFIG_DIR", dir.path());
            std::env::set_var("HOME", dir.path());
        }
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
        let mut kernel = boot_with(opts, dir.path().to_path_buf(), dir.path().join(".xylitol"))
            .await
            .expect("boot_with");
        let hub = kernel
            .ctx
            .get::<xylitol_plugin_sdk::ReloadHub>()
            .expect("reload hub");
        hub.register(std::sync::Arc::new(PluginStep));
        let cancel = tokio_util::sync::CancellationToken::new();
        let report = kernel
            .runtime
            .driver
            .reload_runtime(&cancel)
            .await
            .expect("reload");
        let plugin = report
            .steps
            .iter()
            .find(|step| step.step == "plugin")
            .expect("plugin step");
        assert!(plugin.ok, "plugin step failed: {}", plugin.message);
        assert_eq!(plugin.message, "plugin done");
        assert_eq!(report.steps.last().map(|step| step.step), Some("plugin"));
        let _ = kernel.ctx.shutdown().await;
        unsafe {
            std::env::remove_var("XYLITOL_CONFIG_DIR");
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(env_global)]
    async fn boot_ctx_skips_plugins_outside_the_enabled_set() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config_path, _installed) = write_external_config(&dir, false, "fixture");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(
            registries.plugin_commands.get("plugin:fixture").is_none(),
            "a plugin outside the enabled set must not be assembled"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial(env_global)]
    async fn boot_ctx_skips_external_plugin_with_reserved_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config_path, _installed) = write_external_config(&dir, true, "tools-builtin");
        let (ctx, _loader) = boot_ctx(&boot_opts(config_path)).await.expect("boot_ctx");
        let registries = ctx
            .get::<xylitol_plugin_sdk::HostRegistries>()
            .expect("registries");
        assert!(
            registries
                .plugin_commands
                .get("plugin:tools-builtin")
                .is_none(),
            "reserved external plugin name must not be registered"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn boot_with_self_check_print_profile() {
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var(
                "XYLITOL_CONFIG_DIR",
                dir.path().join(".config").join("xylitol"),
            );
            std::env::set_var("HOME", dir.path());
        }
        let config_path = dir.path().join("config.yaml");
        std::fs::write(
            &config_path,
            "models:\n  default_model: q\n  entries:\n    q:\n      provider: openai\n      model: m\n",
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

    #[test]
    fn describe_wait_names_the_service_and_reason() {
        let cases = [
            (
                WaitReason::OwnRuntime,
                "SessionStore (its own runtime has not released its rows)",
            ),
            (
                WaitReason::NoProvider { like: None },
                "SessionStore (no row provides it)",
            ),
            (
                WaitReason::NoProvider {
                    like: Some("Tools".into()),
                },
                "SessionStore (no row provides it; did you mean `Tools`?)",
            ),
            (
                WaitReason::NoEnabledProvider { like: None },
                "SessionStore (no enabled row provides it; disabled rows may)",
            ),
            (
                WaitReason::NoEnabledProvider {
                    like: Some("Tools".into()),
                },
                "SessionStore (no enabled row provides it; disabled rows may (did you mean `Tools`?))",
            ),
            (
                WaitReason::Explained("not a service type".into()),
                "SessionStore (not a service type)",
            ),
            (
                WaitReason::ProviderDisabled {
                    row: "session-store".into(),
                },
                "SessionStore (the row `session-store` providing it is disabled)",
            ),
            (
                WaitReason::ProviderFailed {
                    row: "session-store".into(),
                    error: "no API key".into(),
                },
                "SessionStore (the row `session-store` providing it failed: no API key)",
            ),
            (
                WaitReason::ProviderInactive {
                    row: "session-store".into(),
                },
                "SessionStore (the row `session-store` providing it is not running)",
            ),
            (
                WaitReason::ProviderRuntime {
                    row: "session-store".into(),
                    runtime: "py".into(),
                },
                "SessionStore (the row `session-store` providing it waits for runtime `py`)",
            ),
            (
                WaitReason::Cycle {
                    row: "session-store".into(),
                    service: "Tools".into(),
                },
                "SessionStore (the row `session-store` providing it waits for `Tools`, which this row provides)",
            ),
            (
                WaitReason::ProviderWaiting {
                    row: "session-store".into(),
                    services: vec!["A".into(), "B".into()],
                },
                "SessionStore (the row `session-store` providing it also waits for A, B)",
            ),
            (
                WaitReason::ProviderStarting {
                    row: "session-store".into(),
                },
                "SessionStore (the row `session-store` providing it is starting)",
            ),
            (
                WaitReason::ProviderRunning {
                    row: "session-store".into(),
                },
                "SessionStore (the row `session-store` providing it runs but has not provided it)",
            ),
            (
                WaitReason::Other("removing".into()),
                "SessionStore (removing)",
            ),
        ];
        for (reason, expected) in cases {
            assert_eq!(describe_wait(&Wait::new("SessionStore", reason)), expected);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn wait_active_times_out_and_falls_back_to_states() {
        use xylitol_plugin_sdk::rt::Plugin;

        struct NeverReady;

        impl Plugin for NeverReady {
            type Deps = ();

            fn name(&self) -> &str {
                "never-ready"
            }

            async fn apply(&self, _ctx: &Ctx, _: ()) -> xylitol_plugin_sdk::rt::Result {
                std::future::pending::<()>().await;
                Ok(())
            }
        }

        let ctx = Ctx::root().expect("root");
        let _view = ctx.plugin(NeverReady);
        let loader = LoaderPlugin::new(
            Chain::new().with(Admit {
                inner: Builtins::new(),
            }),
            LoaderOptions::default(),
        )
        .handle();

        let err = wait_active_within(&ctx, &loader, std::time::Duration::from_millis(20))
            .await
            .expect_err("a loading plugin must time out");
        let message = err.to_string();
        assert!(message.contains("kernel self-check timed out"), "{message}");
        assert!(message.contains("never-ready"), "{message}");
    }

    static LOG_RECORDS: std::sync::Mutex<Vec<(log::Level, String, String)>> =
        std::sync::Mutex::new(Vec::new());

    struct LogCapture;

    impl log::Log for LogCapture {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            LOG_RECORDS.lock().expect("records").push((
                record.level(),
                record.target().to_string(),
                record.args().to_string(),
            ));
        }

        fn flush(&self) {}
    }

    static LOG_CAPTURE: LogCapture = LogCapture;

    fn install_capture_logger() -> bool {
        static INSTALLED: std::sync::Once = std::sync::Once::new();
        static OURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        INSTALLED.call_once(|| {
            if log::set_logger(&LOG_CAPTURE).is_ok() {
                OURS.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        log::set_max_level(log::LevelFilter::Trace);
        OURS.load(std::sync::atomic::Ordering::SeqCst)
    }

    #[test]
    #[serial_test::serial(log_global)]
    fn rutis_error_sink_routes_to_the_log_facade() {
        if !install_capture_logger() {
            eprintln!(
                "skipping: another logger claimed the process before this test; \
                 the log-facade contract is covered by the standalone run"
            );
            return;
        }
        LOG_RECORDS.lock().expect("records").clear();

        let sink = rutis_error_sink();
        sink(Arc::new(ErrorKind::NotServedByName {
            service: "loader".into(),
        }));
        {
            let records = LOG_RECORDS.lock().expect("records");
            assert!(
                !records
                    .iter()
                    .any(|(level, _, _)| *level == log::Level::Error),
                "name-serving notice must not be an error: {records:?}"
            );
            assert!(
                records.iter().any(|(level, target, message)| {
                    *level == log::Level::Debug
                        && target == "xylitol::rutis"
                        && message.contains("loader")
                }),
                "name-serving notice must be logged at debug: {records:?}"
            );
        }

        LOG_RECORDS.lock().expect("records").clear();
        sink(Arc::new(ErrorKind::PluginFailed("boom".into())));

        let records = LOG_RECORDS.lock().expect("records");
        assert!(
            records.iter().any(|(level, target, message)| {
                *level == log::Level::Error
                    && target == "xylitol::rutis"
                    && message.contains("boom")
            }),
            "the rutis sink must write errors through log: {records:?}"
        );
    }
}
