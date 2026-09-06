//! Shared application bootstrap — the single assembly path from CLI args to a
//! constructed [`AgentRuntime`], reused by the print / tui / server surfaces.
//!
//! This module is the seam that prevents assembly drift across surfaces: every
//! surface that needs a ready-to-run agent calls [`bootstrap`]; none inlines its
//! own config-load / registry-build / resource-discovery logic (spec la20).
//!
//! What lives here vs in [`crate::app::core::composition`]:
//! - `composition::build_agent` injects infra ports into an agent builder (the
//!   innermost wiring step).
//! - `bootstrap` runs everything *before* `build_agent`: config load →
//!   ModelRegistry build → trust resolution → resource discovery
//!   resource discovery (context_files/system_prompt/append/skills) → compaction settings →
//!   permission → `build_agent` → model select.
//!
//! Surfaces that need the *resolved ingredients* without constructing an agent
//! (e.g. `--list-models`, which only needs the registry) call
//! `resolve_assembly` directly; surfaces that need a ready-to-run agent call
//! [`bootstrap`].
//!
//! Layering note: this module emits diagnostics only via the returned
//! [`BootstrapOutput::warnings`] list (structured strings). It MUST NOT depend
//! on any surface's presentation module (e.g. `app::cli::provider_guidance`) —
//! surfaces decide how to render warnings. This keeps `app::core` free of
//! reverse dependencies on surfaces.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::agent::capabilities::ModelRegistry;
use crate::agent::model::resolver;
use crate::app::core::composition::{BuildAgentOptions, build_agent};
use crate::infra::config::loader::load_app_config_detailed_with;
use crate::infra::permission;
use crate::infra::session::SessionManager;
use crate::infra::timing;
use crate::protocol::model::XyModelMeta;

/// Process-env getter matching [`crate::infra::config::paths::ConfigPaths::discover`]:
/// real `std::env`, with `HOME` falling back to `dirs::home_dir` when unset.
fn process_get_env(k: &str) -> Option<String> {
    if let Ok(v) = std::env::var(k) {
        return Some(v);
    }
    if k == "HOME" {
        return dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
    }
    None
}

/// `~/.xylitol` from injectable `HOME`, with `dirs::home_dir` fallback (same as
/// `SessionManager::default_dir` / trust / agent dir defaults — without calling them).
fn xylitol_home_dir(get_env: &impl Fn(&str) -> Option<String>) -> PathBuf {
    get_env("HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
}

/// Sync bootstrap bridging into the async model/thinking mutators (their
/// session persistence is awaited; same pattern as the remote driver's
/// `block_on`).
fn block_on<T>(fut: impl std::future::Future<Output = T>) -> T {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build bootstrap current-thread runtime")
            .block_on(fut),
    }
}

/// Inputs to [`bootstrap`] / `resolve_assembly`, mirroring the CLI flags that
/// drive assembly.
///
/// Fields are the minimal set needed to reproduce the print-mode assembly path;
/// they do NOT include presentation concerns (prompt text, mode flags) which
/// stay in the surface.
pub struct BootstrapInput {
    /// `--config` path override.
    pub config_path: Option<std::path::PathBuf>,
    /// `--session` restore id.
    pub session: Option<String>,
    /// `--model` explicit selection (overrides resolved default profile).
    pub model: Option<String>,
    /// `--trust` / `--no-trust` override; `None` = use trust-store resolution.
    pub trust_override: Option<bool>,
    /// Whether to run interactively (affects trust UI callback). print/server
    /// pass `false`; product TUI passes `true` so Ask can prompt on stdio.
    pub interactive: bool,
    /// Diagnostic label identifying the calling surface (e.g. "print", "server",
    /// "tui", "list-models"). Appears in tracing spans so log readers can tell
    /// which surface triggered an assembly. Not used for control flow.
    pub caller: &'static str,
}

/// Structured diagnostics produced during assembly, surfaced to the caller for
/// rendering. Each variant maps to a user-visible message; surfaces decide the
/// exact wording (e.g. cli's `provider_guidance` enriches `NoModelsAvailable`).
#[derive(Debug)]
pub enum BootstrapWarning {
    /// A single model entry had a non-fatal config issue (skipped that entry).
    ModelEntrySkipped(String),
    /// A provider entry had no resolvable API key.
    NoApiKey { provider: String },
    /// The project CWD is not trusted; `.xylitol/` resources were skipped.
    ProjectNotTrusted { reason: String },
    /// Restoring an existing session.
    RestoringSession { session: String, path: String },
    /// Model id did not resolve cleanly (fuzzy/partial match with a warning).
    ModelResolutionWarning(String),
    /// Model id failed to resolve entirely; no model pre-selected.
    ModelResolutionFailed(String),
}

/// Product display when no model is selected (cli-entry ce18).
pub const UNSET_MODEL_DISPLAY: &str = "NOT-SET";

/// A fully-assembled agent plus the resolved side-products surfaces need.
///
/// Prefer [`Self::into_runtime`] (or [`Self::into_driver`]) over reading
/// [`Self::agent`] / [`Self::store`] directly — those fields remain for
/// transitional callers and still name `AgentRuntime` (not an embed stability
/// promise).
pub struct BootstrappedAgent {
    /// The constructed, ready-to-run agent.
    ///
    /// **Leak:** prefer [`Self::into_runtime`] so embedders need not name
    /// `AgentRuntime`.
    pub agent: crate::agent::AgentRuntime,
    /// Session id (restored or freshly generated).
    pub session_id: String,
    /// Diagnostics produced during assembly (surface renders these).
    pub warnings: Vec<BootstrapWarning>,
    /// Session store handle, the same instance the agent holds internally.
    /// Surfaces construct an [`crate::app::core::driver::XyInProcessDriver`] from this + the agent so
    /// XyDriver session commands (SwitchSession/GetMessages) operate without
    /// reaching into agent internals.
    pub store: Arc<dyn crate::protocol::ports::XySessionStore>,
    /// MCP servers from loaded config (`None` / empty = disabled, zero-cost).
    pub mcp_servers: Option<Vec<crate::app::core::mcp_spec::McpServerSpec>>,
}

/// XyDriver-ready result of [`BootstrappedAgent::into_runtime`].
///
/// This is the preferred embed / multi-client handoff: no need to name
/// `AgentRuntime` at the call site.
pub struct BootstrappedRuntime {
    pub driver: crate::app::core::driver::XyInProcessDriver,
    pub session_id: String,
    pub warnings: Vec<BootstrapWarning>,
    /// MCP servers for [`crate::app::core::composition::McpSession::reload`].
    pub mcp_servers: Option<Vec<crate::app::core::mcp_spec::McpServerSpec>>,
}

/// Run a short async bootstrap operation from synchronous assembly code.
///
/// Tokio permits `block_in_place` only on a multi-thread runtime. Current-thread
/// callers instead use a one-shot worker runtime so bootstrap remains usable in
/// current-thread tests and embedded callers.
fn block_on_bootstrap_task<T>(
    operation: &'static str,
    task: impl Future<Output = T> + Send + 'static,
) -> Option<T>
where
    T: Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current()
        && handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread
    {
        return Some(tokio::task::block_in_place(|| handle.block_on(task)));
    }

    let worker = std::thread::Builder::new()
        .name("xy-bootstrap-sync".into())
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map(|runtime| runtime.block_on(task))
        });

    match worker {
        Ok(worker) => match worker.join() {
            Ok(Ok(value)) => Some(value),
            Ok(Err(error)) => {
                log::warn!("bootstrap async operation failed operation={operation} error={error}");
                None
            }
            Err(_) => {
                log::warn!("bootstrap async operation panicked operation={operation}");
                None
            }
        },
        Err(error) => {
            log::warn!("bootstrap async worker failed operation={operation} error={error}");
            None
        }
    }
}

#[derive(Default)]
struct RestoredSessionMetadata {
    thinking_level: Option<String>,
    session_name: Option<String>,
}

async fn load_restored_session_metadata(
    store: Arc<dyn crate::protocol::ports::XySessionStore>,
    session_id: String,
) -> RestoredSessionMetadata {
    let thinking_level = if !store.exists(&session_id).await {
        None
    } else {
        match store.build_session_context(&session_id).await {
            Ok(context) => Some(context.thinking_level),
            Err(error) => {
                log::warn!(
                    "session thinking restore skipped session_id={} error={error}",
                    session_id
                );
                None
            }
        }
    };
    let session_name = match store.get_session_name(&session_id).await {
        Ok(name) => name,
        Err(error) => {
            log::warn!(
                "observability session-name load skipped session_id={} error={error}",
                session_id
            );
            None
        }
    };
    RestoredSessionMetadata {
        thinking_level,
        session_name,
    }
}

impl BootstrappedAgent {
    /// Consume into an [`crate::app::core::driver::XyInProcessDriver`] plus side-products (preferred path).
    pub fn into_runtime(mut self) -> BootstrappedRuntime {
        let restored_session = block_on_bootstrap_task(
            "restore session metadata",
            load_restored_session_metadata(Arc::clone(&self.store), self.session_id.clone()),
        )
        .unwrap_or_default();
        self.agent
            .bind_session(self.session_id.clone())
            .expect("bootstrap bind_session");
        if let Some(level) = restored_session.thinking_level {
            // Session restoration is observational: retain the persisted literal
            // without validating it against today's support set or writing a
            // compensating history entry.
            self.agent.restore_thinking_level(level);
        }
        if let Some(name) = restored_session.session_name {
            xylitol_ai_bridge::provider::set_obs_session_name(Some(name.as_str()));
        }
        let driver = crate::app::core::driver::XyInProcessDriver::new(self.agent, self.store);
        let todo_gw = driver.todo_gateway();
        let sid = self.session_id.clone();
        let _ = block_on_bootstrap_task("bind todo session", async move {
            todo_gw.bind_session(Some(sid)).await;
            Ok::<(), String>(())
        });
        BootstrappedRuntime {
            driver,
            session_id: self.session_id,
            warnings: self.warnings,
            mcp_servers: self.mcp_servers,
        }
    }

    /// Consume into an [`crate::app::core::driver::XyInProcessDriver`] only (drops warnings / session id /
    /// mcp config). Prefer [`Self::into_runtime`] when those are needed.
    pub fn into_driver(self) -> crate::app::core::driver::XyInProcessDriver {
        self.into_runtime().driver
    }
}

/// Resolved assembly inputs — the *ingredients* ready for `build_agent`, prior
/// to construction. Returned by `resolve_assembly` and consumed by callers that need ingredients without building (e.g. --list-models, which
/// rebuilds the agent per command) and by [`bootstrap`] (which builds once).
///
/// Exposing this lets ingredient-only callers share resolution without forcing a full build via
/// the single-built-agent model.
pub struct ResolvedAssembly {
    pub model_registry: ModelRegistry,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    /// Skills discovered under Trust semantics (c1085).
    pub skills: Vec<crate::protocol::resource::SkillInfo>,
    pub cwd: String,
    pub compaction_settings: Option<crate::agent::compaction::CompactionSettings>,
    pub permission: Option<Arc<dyn crate::protocol::ports::XyPermission>>,
    pub steering_mode: crate::agent::capabilities::QueueMode,
    pub follow_up_mode: crate::agent::capabilities::QueueMode,
    /// Resolved default profile's model id, if any (for startup model selection
    /// when `BootstrapInput::model` is absent).
    pub default_profile_model: Option<String>,
    /// Resolved session id (restored or freshly generated).
    pub session_id: String,
    /// Whether the requested `--session` already exists on disk.
    pub session_file_exists: bool,
    /// Diagnostics produced during resolution.
    pub warnings: Vec<BootstrapWarning>,
    /// MCP servers from YAML (`None` / empty = not enabled).
    pub mcp_servers: Option<Vec<crate::app::core::mcp_spec::McpServerSpec>>,
    /// Three-tier script hook configuration.
    pub hooks_config: crate::infra::config::types::HooksConfig,
    /// Settings `defaultThinkingLevel` (camelCase JSON), if any.
    pub default_thinking_level: Option<String>,
    /// Settings `thinkingBudgets` (camelCase JSON), if any.
    pub thinking_budgets: Option<crate::protocol::model::ThinkingBudgets>,
    /// `AppConfig.tool_batch.mode` (c1545).
    pub batch_mode: crate::protocol::ports::XyBatchMode,
    /// `AppConfig.session.max_turns` when set (c1620).
    pub max_turns: Option<u32>,
}

impl ResolvedAssembly {
    /// Fold into [`BuildAgentOptions`] for `composition::build_agent`.
    /// Drops the side-products (warnings) the builder does not consume.
    pub fn into_build_options(self) -> BuildAgentOptions {
        BuildAgentOptions {
            model_registry: self.model_registry,
            system_prompt: self.system_prompt,
            context_files: self.context_files,
            append_system_prompt: self.append_system_prompt,
            skills: self.skills,
            cwd: self.cwd,
            compaction_settings: self.compaction_settings,
            permission: self.permission,
            steering_mode: self.steering_mode,
            follow_up_mode: self.follow_up_mode,
            event_sink: None,
            hooks_config: self.hooks_config,
            batch_mode: self.batch_mode,
        }
    }
}

/// Error when assembly cannot proceed (no models available, build failure).
#[derive(Debug)]
pub enum BootstrapError {
    /// YAML / template / IO config load failed (fail-closed; ce17).
    ConfigLoadFailed(String),
    /// Config YAML layers loaded but contributed zero registerable models (ce2).
    ConfigLoadedZeroModels,
    /// No models could be loaded from config or environment.
    NoModelsAvailable,
    /// `build_agent` returned an error.
    BuildFailed(String),
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapError::ConfigLoadFailed(e) => write!(f, "config load failed: {e}"),
            BootstrapError::ConfigLoadedZeroModels => write!(
                f,
                "config file present but loaded 0 models (check `models:` vs `model:` typo; \
                 see configs/example.yaml)"
            ),
            BootstrapError::NoModelsAvailable => write!(f, "no models available"),
            BootstrapError::BuildFailed(e) => write!(f, "agent build failed: {e}"),
        }
    }
}

impl std::error::Error for BootstrapError {}

/// Resolve all assembly inputs (config → registry → trust → resources →
/// compaction → permission) without constructing the agent.
///
/// Shared by [`bootstrap`] (build-once surfaces) and ingredient-only callers (e.g. --list-models).
/// Surfaces that need a single agent should call [`bootstrap`] directly.
pub fn resolve_assembly(input: &BootstrapInput) -> Result<ResolvedAssembly, BootstrapError> {
    resolve_assembly_with(
        input,
        process_get_env,
        std::env::current_dir().ok().as_deref(),
    )
}

/// Injectable [`resolve_assembly`] for tests: config discovery, `~/.xylitol`
/// paths, and trust/resource cwd come from `get_env` / `cwd` instead of the
/// process environment.
pub fn resolve_assembly_with(
    input: &BootstrapInput,
    get_env: impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> Result<ResolvedAssembly, BootstrapError> {
    let cli_config_path = input.config_path.as_deref();
    let mut warnings: Vec<BootstrapWarning> = Vec::new();

    // ── Step 1: load YAML config ──────────────────────────────────
    let loaded = match load_app_config_detailed_with(cli_config_path, &get_env, cwd) {
        Ok(loaded) => loaded,
        Err(e) => return Err(BootstrapError::ConfigLoadFailed(e.to_string())),
    };
    let from_yaml_layers = loaded.from_yaml_layers;
    let app_config = Some(loaded.config);
    timing::time("config.load");

    // ── Step 2: build ModelRegistry ───────────────────────────────
    let mut model_registry = ModelRegistry::new();

    if let Some(ref cfg) = app_config {
        let mut missing_key_providers = std::collections::BTreeSet::new();
        for (alias, entry) in &cfg.model.models {
            // m17: explicit YAML aliases always register; missing key → empty (no kind-env fallback).
            let api_key = resolve_entry_api_key(entry);
            if api_key.is_empty() {
                missing_key_providers.insert(entry.provider.provider_name().to_string());
            }

            match build_model_meta(alias, entry, api_key) {
                Ok(meta) => model_registry.register(meta),
                Err(e) => {
                    warnings.push(BootstrapWarning::ModelEntrySkipped(format!(
                        "models.{alias}: {e}"
                    )));
                }
            }
        }
        for provider in missing_key_providers {
            warnings.push(BootstrapWarning::NoApiKey { provider });
        }
    }

    // ce2: YAML present but zero explicit model aliases → hard fail (no env gpt-4o).
    if from_yaml_layers && model_registry.is_empty() {
        return Err(BootstrapError::ConfigLoadedZeroModels);
    }

    // m12: never invent registry entries from OPENAI_/ANTHROPIC_ env alone.
    timing::time("model_registry.load");

    if model_registry.is_empty() {
        return Err(BootstrapError::NoModelsAvailable);
    }

    // ── Session dir + restore info ────────────────────────────────
    let agent_home = xylitol_home_dir(&get_env);
    let sessions_dir = agent_home.join("sessions");
    std::fs::create_dir_all(&sessions_dir).ok();
    let session_mgr = SessionManager::new(sessions_dir.clone());
    let session_file_exists = input
        .session
        .as_deref()
        .is_some_and(|session_id| session_mgr.exists(session_id));
    if let Some(ref session_arg) = input.session
        && session_file_exists
    {
        warnings.push(BootstrapWarning::RestoringSession {
            session: session_arg.clone(),
            path: sessions_dir.display().to_string(),
        });
    }
    timing::time("session.restore");

    // ── Step 3: resolve default profile (drives system_prompt/model) ──
    let resolved_profile = app_config
        .as_ref()
        .and_then(|cfg| cfg.resolve_default_profile().ok());
    let config_system_prompt = resolved_profile
        .as_ref()
        .and_then(|p| p.system_prompt.clone());
    let default_profile_model = resolved_profile
        .as_ref()
        // Registry alias (not the raw vendor id): consumers select by registry
        // identity (`registry.find(alias)`); raw names fail when alias ≠ model.
        .map(|p| p.model_id.clone());

    let cwd_path = cwd
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    let cwd = cwd_path.to_string_lossy().to_string();

    // ── Step 3b: resolve trust + discover resources ───────────────
    let trust_manager = crate::infra::trust::TrustManager::new(&agent_home);
    let trust_cwd = cwd.clone();
    let trust_resolution = crate::infra::trust::resolve_project_trusted(
        &trust_manager,
        &cwd,
        input.trust_override,
        crate::infra::trust::DefaultProjectTrust::default(),
        input.interactive,
        |options| {
            if input.interactive {
                crate::infra::trust::prompt_trust_options_stdio(&trust_cwd, options)
            } else {
                None
            }
        },
    );
    let project_trusted = trust_resolution.trusted;
    if !project_trusted {
        warnings.push(BootstrapWarning::ProjectNotTrusted {
            reason: match trust_resolution.reason {
                crate::infra::trust::TrustReason::Override => "overridden via --no-trust".into(),
                crate::infra::trust::TrustReason::Store => "denied in trust store".into(),
                crate::infra::trust::TrustReason::FallbackNoUi => {
                    "no interactive prompt available".into()
                }
                _ => "policy".into(),
            },
        });
    }

    let (context_files, loader_system_prompt, append_system_prompt, skills) = {
        let agent_dir = agent_home.clone();
        let loader_cwd = if project_trusted {
            cwd_path.clone()
        } else {
            std::env::temp_dir()
        };
        let loader = crate::infra::resource::DefaultResourceLoader::new(loader_cwd, agent_dir);
        let ctx: Vec<(String, String)> = loader
            .get_agents_files()
            .iter()
            .map(|f| (f.path.to_string_lossy().into_owned(), f.content.clone()))
            .collect();
        let sys = loader.get_system_prompt().map(String::from);
        let append = loader.get_append_system_prompt().to_vec();
        let skills = loader.get_skills().0.to_vec();
        (ctx, sys, append, skills)
    };
    log::debug!(
        "resource discovery resolved caller={} trusted={} cwd={} context_files={} append_system_prompt={} loader_system_prompt={} skills={}",
        input.caller,
        project_trusted,
        cwd,
        context_files.len(),
        append_system_prompt.len(),
        loader_system_prompt.is_some(),
        skills.len()
    );
    for (path, content) in &context_files {
        log::debug!(
            "context file discovered context_file={} bytes={}",
            path,
            content.len()
        );
    }

    // ── Step 3b2: compaction + queue settings ─────────────────────
    let (
        compaction_settings,
        steering_mode,
        follow_up_mode,
        default_thinking_level,
        thinking_budgets,
    ) = {
        let agent_dir = agent_home;
        let settings_cwd = if project_trusted {
            cwd_path
        } else {
            std::env::temp_dir()
        };
        let settings_mgr = crate::infra::settings::SettingsManager::from_files(
            &settings_cwd,
            &agent_dir,
            project_trusted,
        );
        let compaction = settings_mgr
            .get_settings()
            .compaction
            .as_ref()
            .map(|c| crate::agent::compaction::CompactionSettings::from(c.clone()));
        let steering_mode = queue_mode_from_settings(settings_mgr.get_steering_mode());
        let follow_up_mode = queue_mode_from_settings(settings_mgr.get_follow_up_mode());
        let default_thinking_level = settings_mgr.get_settings().default_thinking_level.clone();
        let thinking_budgets: Option<crate::protocol::model::ThinkingBudgets> =
            settings_mgr.get_thinking_budgets().map(Into::into);
        (
            compaction,
            steering_mode,
            follow_up_mode,
            default_thinking_level,
            thinking_budgets,
        )
    };

    // ── Step 3c: permission engine ────────────────────────────────
    let permission_engine = app_config.as_ref().and_then(|cfg| {
        cfg.security
            .permission
            .as_ref()
            .filter(|sc| sc.enabled)
            .map(|sc| permission::build_permission(sc))
    });

    // ── Step 3c1: merge system-prompt sources ─────────────────────
    let system_prompt = loader_system_prompt.or(config_system_prompt);

    let session_id = input
        .session
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mcp_servers = crate::app::core::mcp_spec::McpServerSpec::from_infra_list(
        app_config.as_ref().and_then(|c| c.mcp_servers.clone()),
    );

    let hooks_config = app_config
        .as_ref()
        .map(|c| c.hooks.clone())
        .unwrap_or_default();

    let batch_mode = app_config
        .as_ref()
        .map(|c| crate::protocol::ports::XyBatchMode::from(c.tool_batch.mode))
        .unwrap_or_default();

    let max_turns = app_config
        .as_ref()
        .and_then(|c| c.session.as_ref())
        .and_then(|s| s.max_turns);

    Ok(ResolvedAssembly {
        model_registry,
        system_prompt,
        context_files,
        append_system_prompt,
        skills,
        cwd,
        compaction_settings,
        permission: permission_engine,
        steering_mode,
        follow_up_mode,
        default_profile_model,
        session_id,
        session_file_exists,
        warnings,
        mcp_servers,
        hooks_config,
        default_thinking_level,
        thinking_budgets,
        batch_mode,
        max_turns,
    })
}

/// Bootstrap a fully-assembled agent: resolve inputs, build once, select model.
///
/// Used by print / tui / server — surfaces that hold a single agent for their
/// lifetime. Ingredient-only paths (e.g. --list-models) use `resolve_assembly` directly.
pub fn bootstrap(input: BootstrapInput) -> Result<BootstrappedAgent, BootstrapError> {
    bootstrap_with(
        input,
        process_get_env,
        std::env::current_dir().ok().as_deref(),
    )
}

/// Injectable [`bootstrap`] for tests (same `get_env` / `cwd` semantics as
/// [`resolve_assembly_with`]).
pub fn bootstrap_with(
    input: BootstrapInput,
    get_env: impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> Result<BootstrappedAgent, BootstrapError> {
    let model = input.model.clone();

    let mut assembly = resolve_assembly_with(&input, get_env, cwd)?;
    let session_id = assembly.session_id.clone();
    let mcp_servers = assembly.mcp_servers.clone();
    let default_thinking_level = assembly.default_thinking_level.clone();
    let thinking_budgets = assembly.thinking_budgets.clone();
    let max_turns = assembly.max_turns;
    let session_file_exists = assembly.session_file_exists;
    let target_model = model.or_else(|| assembly.default_profile_model.clone());
    let mut warnings = std::mem::take(&mut assembly.warnings);

    let mut agent = build_agent(assembly.into_build_options())
        .map_err(|e| BootstrapError::BuildFailed(e.to_string()))?;

    if let Some(n) = max_turns.filter(|&n| n >= 1) {
        agent.set_should_stop_after_turn(Some(crate::agent::max_turns_stop_hook(n)));
    }

    timing::time("session.create");

    let mut requested_thinking_level = None;
    if let Some(mid) = target_model {
        let available_owned = agent.model_registry();
        let available: Vec<&XyModelMeta> = available_owned.list().iter().collect();
        match resolver::resolve_model(&mid, &available, None) {
            Ok(resolved) => {
                if let Some(ref warning) = resolved.warning {
                    warnings.push(BootstrapWarning::ModelResolutionWarning(warning.clone()));
                }
                let _ = block_on(agent.select_model(&resolved.model.id));
                requested_thinking_level = resolved.thinking_level;
            }
            Err(err) => {
                warnings.push(BootstrapWarning::ModelResolutionFailed(err.to_string()));
            }
        }
    }

    // Settings are only a first-session preference. A resumed session restores
    // its exact persisted level later, and model switching always uses the
    // declared list's final item.
    if !session_file_exists {
        agent.apply_default_thinking_level(default_thinking_level.as_deref());
    }
    // A `model:thinkingLevel` request is more specific than the Settings
    // first-session preference. Unsupported values leave the selected model
    // default unchanged.
    if let Some(level) = requested_thinking_level
        && let Err(error) = block_on(agent.set_thinking_level(level.clone()))
    {
        log::warn!("bootstrap model:thinkingLevel rejected level={level} error={error}");
    }
    agent.set_thinking_budgets(thinking_budgets);

    // Reuse the same session store injected into the agent at composition time.
    let store = agent.session_store();

    Ok(BootstrappedAgent {
        agent,
        session_id,
        warnings,
        store,
        mcp_servers,
    })
}

/// Report from [`reload_skills`] (c1085).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsReloadReport {
    pub names: Vec<String>,
    pub count: usize,
}

/// Re-discover skills from disk and apply to `driver` (c1085).
///
/// Trust semantics match bootstrap. Does **not** mutate session history.
pub fn reload_skills(
    driver: &mut crate::app::core::driver::XyInProcessDriver,
    cwd: &std::path::Path,
    agent_dir: &std::path::Path,
    project_trusted: bool,
) -> SkillsReloadReport {
    let loader_cwd = if project_trusted {
        cwd.to_path_buf()
    } else {
        std::env::temp_dir()
    };
    let loader =
        crate::infra::resource::DefaultResourceLoader::new(loader_cwd, agent_dir.to_path_buf());
    let skills = loader.get_skills().0.to_vec();
    let mut names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
    names.sort();
    names.dedup();
    let report = SkillsReloadReport {
        count: names.len(),
        names,
    };
    driver.apply_skills(skills);
    report
}

/// Report from [`reload_prompt_context`] (c1100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptContextReloadReport {
    pub context_file_count: usize,
    pub has_system_prompt: bool,
    pub append_count: usize,
}

/// Re-discover AGENTS/SYSTEM/APPEND from disk and apply to `driver` (c1100).
///
/// Trust semantics match bootstrap: when `project_trusted` is false, project CWD
/// is replaced with a temp path so project-local resources are skipped.
/// `config_system_prompt` is the profile fallback when no SYSTEM.md is found.
///
/// Does **not** mutate session history / transcript.
pub fn reload_prompt_context(
    driver: &mut crate::app::core::driver::XyInProcessDriver,
    cwd: &std::path::Path,
    agent_dir: &std::path::Path,
    project_trusted: bool,
    config_system_prompt: Option<String>,
) -> PromptContextReloadReport {
    let loader_cwd = if project_trusted {
        cwd.to_path_buf()
    } else {
        std::env::temp_dir()
    };
    let loader =
        crate::infra::resource::DefaultResourceLoader::new(loader_cwd, agent_dir.to_path_buf());

    let context_files: Vec<(String, String)> = loader
        .get_agents_files()
        .iter()
        .map(|f| (f.path.to_string_lossy().into_owned(), f.content.clone()))
        .collect();
    let system_prompt = loader
        .get_system_prompt()
        .map(String::from)
        .or(config_system_prompt);
    let append_system_prompt = loader.get_append_system_prompt().to_vec();

    let report = PromptContextReloadReport {
        context_file_count: context_files.len(),
        has_system_prompt: system_prompt.is_some(),
        append_count: append_system_prompt.len(),
    };
    driver.apply_prompt_resources(context_files, system_prompt, append_system_prompt);
    report
}

/// Per-model `api_key` after config/secret render.
/// Omit or empty → empty string. MUST NOT fall back to kind-level env (m17).
fn resolve_entry_api_key(entry: &crate::infra::config::types::ModelEntry) -> String {
    match &entry.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => String::new(),
    }
}

/// One YAML `models.<alias>` entry → its [`XyModelMeta`]. Err = skip with a
/// `ModelEntrySkipped` warning (thinking-config resolution failure).
fn build_model_meta(
    alias: &str,
    entry: &crate::infra::config::types::ModelEntry,
    api_key: String,
) -> Result<XyModelMeta, String> {
    let context_window = if entry.context_window > 0 {
        entry.context_window
    } else {
        crate::agent::model::registry::default_context_window_for(entry.provider)
    };

    let (thinking_levels, thinking_level_map) =
        entry.resolve_thinking_config().map_err(|e| e.to_string())?;

    Ok(XyModelMeta {
        id: alias.to_string(),
        config: crate::protocol::model::XyModelConfig {
            kind: entry.provider,
            api_key,
            model: entry.model.clone(),
            base_url: entry.base_url.clone(),
            // c1598: honor YAML `models.*.api`; None → infra default_for
            api: entry.api.clone(),
            compat: entry.compat.clone(),
        },
        display_name: alias.to_string(),
        thinking: entry.thinking,
        context_window,
        api: entry.api.clone().unwrap_or_default(),
        provider: entry.provider.provider_name().to_string(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels,
        thinking_level_map,
    })
}

fn queue_mode_from_settings(
    mode: crate::infra::settings::types::SteeringMode,
) -> crate::agent::capabilities::QueueMode {
    match mode {
        crate::infra::settings::types::SteeringMode::All => {
            crate::agent::capabilities::QueueMode::All
        }
        crate::infra::settings::types::SteeringMode::OneAtATime => {
            crate::agent::capabilities::QueueMode::OneAtATime
        }
    }
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod tests;
