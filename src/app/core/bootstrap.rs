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
//!   (templates/context_files/system_prompt/append) → compaction settings →
//!   permission → `build_agent` → model select.
//!
//! Surfaces that need the *resolved ingredients* without constructing an agent
//! (e.g. `--list-models`, which only needs the registry) consume
//! [`resolve_assembly`] directly; surfaces that need a ready-to-run agent call
//! [`bootstrap`].
//!
//! Layering note: this module emits diagnostics only via the returned
//! [`BootstrapOutput::warnings`] list (structured strings). It MUST NOT depend
//! on any surface's presentation module (e.g. `app::cli::provider_guidance`) —
//! surfaces decide how to render warnings. This keeps `app::core` free of
//! reverse dependencies on surfaces.

use std::sync::Arc;

use crate::agent::model::resolver;
use crate::agent::session::ModelRegistry;
use crate::app::core::composition::{BuildAgentOptions, build_agent};
use crate::domain::resource_types::PromptTemplate;
use crate::domain::types::XyModelMeta;
use crate::infra::config::loader::load_app_config;
use crate::infra::config::value::InfraSecretResolver;
use crate::infra::permission;
use crate::infra::session::SessionManager;
use crate::infra::timing;

/// Inputs to [`bootstrap`] / [`resolve_assembly`], mirroring the CLI flags that
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
    /// Config file was found but failed to load (fell back to env vars).
    ConfigLoadFailed(String),
    /// Config loaded but contributed zero models (likely a `model:` typo).
    ConfigLoadedZeroModels,
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
    /// Surfaces construct an [`InProcessDriver`] from this + the agent so
    /// Driver session commands (SwitchSession/GetMessages) operate without
    /// reaching into agent internals.
    pub store: Arc<dyn crate::runtime_protocol::XySessionStore>,
    /// MCP servers from loaded config (`None` / empty = disabled, zero-cost).
    pub mcp_servers: Option<Vec<crate::app::core::mcp_spec::McpServerSpec>>,
}

/// Driver-ready result of [`BootstrappedAgent::into_runtime`].
///
/// This is the preferred embed / multi-client handoff: no need to name
/// `AgentRuntime` at the call site.
pub struct BootstrappedRuntime {
    pub driver: crate::app::core::driver::InProcessDriver,
    pub session_id: String,
    pub warnings: Vec<BootstrapWarning>,
    /// MCP servers for [`crate::app::core::composition::McpSession::reload`].
    pub mcp_servers: Option<Vec<crate::app::core::mcp_spec::McpServerSpec>>,
}

impl BootstrappedAgent {
    /// Consume into an [`InProcessDriver`] plus side-products (preferred path).
    pub fn into_runtime(self) -> BootstrappedRuntime {
        BootstrappedRuntime {
            driver: crate::app::core::driver::InProcessDriver::new(self.agent, self.store),
            session_id: self.session_id,
            warnings: self.warnings,
            mcp_servers: self.mcp_servers,
        }
    }

    /// Consume into an [`InProcessDriver`] only (drops warnings / session id /
    /// mcp config). Prefer [`Self::into_runtime`] when those are needed.
    pub fn into_driver(self) -> crate::app::core::driver::InProcessDriver {
        self.into_runtime().driver
    }
}

/// Resolved assembly inputs — the *ingredients* ready for `build_agent`, prior
/// to construction. Returned by [`resolve_assembly`] and consumed by callers that need ingredients without building (e.g. --list-models, which
/// rebuilds the agent per command) and by [`bootstrap`] (which builds once).
///
/// Exposing this lets ingredient-only callers share resolution without forcing a full build via
/// the single-built-agent model.
pub struct ResolvedAssembly {
    pub model_registry: ModelRegistry,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    pub max_iterations: u32,
    pub compaction_threshold: f64,
    pub cwd: String,
    pub compaction_settings: Option<crate::agent::compaction::CompactionSettings>,
    pub permission: Option<Arc<dyn crate::runtime_protocol::XyPermission>>,
    pub steering_mode: crate::agent::session::QueueMode,
    pub follow_up_mode: crate::agent::session::QueueMode,
    pub discovered_templates: Vec<PromptTemplate>,
    /// Resolved default profile's model id, if any (for startup model selection
    /// when `BootstrapInput::model` is absent).
    pub default_profile_model: Option<String>,
    /// Resolved session id (restored or freshly generated).
    pub session_id: String,
    /// Diagnostics produced during resolution.
    pub warnings: Vec<BootstrapWarning>,
    /// MCP servers from YAML (`None` / empty = not enabled).
    pub mcp_servers: Option<Vec<crate::app::core::mcp_spec::McpServerSpec>>,
}

impl ResolvedAssembly {
    /// Fold into [`BuildAgentOptions`] for `composition::build_agent`.
    /// Drops the side-products (templates, warnings) the builder
    /// does not consume.
    pub fn into_build_options(self) -> BuildAgentOptions {
        BuildAgentOptions {
            model_registry: self.model_registry,
            system_prompt: self.system_prompt,
            context_files: self.context_files,
            append_system_prompt: self.append_system_prompt,
            max_iterations: self.max_iterations,
            compaction_threshold: self.compaction_threshold,
            cwd: self.cwd,
            compaction_settings: self.compaction_settings,
            permission: self.permission,
            steering_mode: self.steering_mode,
            follow_up_mode: self.follow_up_mode,
            event_sink: None,
        }
    }
}

/// Error when assembly cannot proceed (no models available, build failure).
#[derive(Debug)]
pub enum BootstrapError {
    /// No models could be loaded from config or environment.
    NoModelsAvailable,
    /// `build_agent` returned an error.
    BuildFailed(String),
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
    let cli_config_path = input.config_path.as_deref();
    let mut warnings: Vec<BootstrapWarning> = Vec::new();

    // ── Step 1: load YAML config ──────────────────────────────────
    let app_config = match load_app_config(cli_config_path) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            warnings.push(BootstrapWarning::ConfigLoadFailed(e.to_string()));
            None
        }
    };
    timing::time("config.load");

    let config_loaded = app_config.is_some();

    // ── Step 2: build ModelRegistry ───────────────────────────────
    let secret_resolver: Arc<dyn crate::runtime_protocol::XySecretResolver> =
        Arc::new(InfraSecretResolver::new());
    let mut model_registry = ModelRegistry::new(secret_resolver);

    if let Some(ref cfg) = app_config {
        for (alias, entry) in &cfg.model.models {
            let api_key = resolve_api_key(entry.provider);
            if api_key.is_none() {
                warnings.push(BootstrapWarning::NoApiKey {
                    provider: entry.provider.provider_name().to_string(),
                });
                continue;
            }

            let context_window = if entry.context_window > 0 {
                entry.context_window
            } else {
                crate::agent::model::registry::default_context_window_for(entry.provider)
            };

            model_registry.register(XyModelMeta {
                id: alias.clone(),
                config: crate::domain::model::XyModelConfig {
                    kind: entry.provider,
                    api_key: api_key.expect("checked above"),
                    model: entry.model.clone(),
                    base_url: entry.base_url.clone(),
                    api: None,
                },
                display_name: alias.clone(),
                thinking: entry.thinking,
                context_window,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            });
        }
    }

    if config_loaded && model_registry.is_empty() {
        warnings.push(BootstrapWarning::ConfigLoadedZeroModels);
    }

    if model_registry.is_empty() {
        for (provider_name, env_var, kind) in [
            (
                "openai",
                "OPENAI_API_KEY",
                crate::domain::model::XyModelKind::OpenAi,
            ),
            (
                "anthropic",
                "ANTHROPIC_API_KEY",
                crate::domain::model::XyModelKind::Anthropic,
            ),
        ] {
            if let Ok(key) = std::env::var(env_var)
                && let Some(model_id) =
                    crate::agent::model::registry::default_model_id_for_provider(provider_name)
            {
                model_registry.register(XyModelMeta {
                    id: model_id.to_string(),
                    config: crate::domain::model::XyModelConfig {
                        kind,
                        api_key: key,
                        model: model_id.to_string(),
                        base_url: None,
                        api: None,
                    },
                    display_name: model_id.to_string(),
                    thinking: true,
                    context_window: crate::agent::model::registry::default_context_window_for(kind),
                    api: String::new(),
                    provider: String::new(),
                    cost_input: 0.0,
                    cost_output: 0.0,
                    cost_cache_read: 0.0,
                    cost_cache_write: 0.0,
                    max_tokens: 0,
                    thinking_levels: Vec::new(),
                });
            }
        }
    }
    timing::time("model_registry.load");

    if model_registry.is_empty() {
        return Err(BootstrapError::NoModelsAvailable);
    }

    // ── Session dir + restore info ────────────────────────────────
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).ok();
    let session_mgr = SessionManager::new(sessions_dir.clone());
    if let Some(ref session_arg) = input.session
        && session_mgr.exists(session_arg)
    {
        warnings.push(BootstrapWarning::RestoringSession {
            session: session_arg.clone(),
            path: sessions_dir.display().to_string(),
        });
    }
    timing::time("session.restore");

    // ── Step 3: resolve default profile (drives system_prompt/max_iter/model) ──
    let resolved_profile = app_config
        .as_ref()
        .and_then(|cfg| cfg.resolve_default_profile().ok());
    let config_system_prompt = resolved_profile
        .as_ref()
        .and_then(|p| p.system_prompt.clone());
    let max_iterations = resolved_profile
        .as_ref()
        .map(|p| p.max_iterations)
        .unwrap_or(50);
    let default_profile_model = resolved_profile
        .as_ref()
        .map(|p| p.model_config.model.clone());

    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // ── Step 3b: resolve trust + discover resources ───────────────
    let trust_manager =
        crate::infra::trust::TrustManager::new(crate::infra::trust::TrustManager::default_dir());
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

    let (discovered_templates, context_files, loader_system_prompt, append_system_prompt) = {
        let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
        let loader_cwd = if project_trusted {
            std::path::PathBuf::from(&cwd)
        } else {
            std::env::temp_dir()
        };
        let loader = crate::infra::resource::DefaultResourceLoader::new(loader_cwd, agent_dir);
        let templates = loader.get_prompts().0.to_vec();
        let ctx: Vec<(String, String)> = loader
            .get_agents_files()
            .iter()
            .map(|f| (f.path.to_string_lossy().into_owned(), f.content.clone()))
            .collect();
        let sys = loader.get_system_prompt().map(String::from);
        let append = loader.get_append_system_prompt().to_vec();
        (templates, ctx, sys, append)
    };
    tracing::debug!(
        caller = %input.caller,
        trusted = project_trusted,
        cwd = %cwd,
        context_files = context_files.len(),
        templates = discovered_templates.len(),
        append_system_prompt = append_system_prompt.len(),
        loader_system_prompt = loader_system_prompt.is_some(),
        "resource discovery resolved"
    );
    for (path, content) in &context_files {
        tracing::debug!(
            context_file = %path,
            bytes = content.len(),
            "context file discovered"
        );
    }

    // ── Step 3b2: compaction + queue settings ─────────────────────
    let (compaction_settings, steering_mode, follow_up_mode) = {
        let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
        let settings_cwd = if project_trusted {
            std::path::PathBuf::from(&cwd)
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
        (compaction, steering_mode, follow_up_mode)
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

    Ok(ResolvedAssembly {
        model_registry,
        system_prompt,
        context_files,
        append_system_prompt,
        max_iterations,
        compaction_threshold: 0.8,
        cwd,
        compaction_settings,
        permission: permission_engine,
        steering_mode,
        follow_up_mode,
        discovered_templates,
        default_profile_model,
        session_id,
        warnings,
        mcp_servers,
    })
}

/// Bootstrap a fully-assembled agent: resolve inputs, build once, select model.
///
/// Used by print / tui / server — surfaces that hold a single agent for their
/// lifetime. Ingredient-only paths (e.g. --list-models) use [`resolve_assembly`] directly.
pub fn bootstrap(input: BootstrapInput) -> Result<BootstrappedAgent, BootstrapError> {
    let model = input.model.clone();

    let mut assembly = resolve_assembly(&input)?;
    let discovered_templates = assembly.discovered_templates.clone();
    let session_id = assembly.session_id.clone();
    let mcp_servers = assembly.mcp_servers.clone();
    let target_model = model.or_else(|| assembly.default_profile_model.clone());
    let mut warnings = std::mem::take(&mut assembly.warnings);

    let mut agent =
        build_agent(assembly.into_build_options()).map_err(BootstrapError::BuildFailed)?;
    agent
        .inner_mut()
        .register_prompt_commands(&discovered_templates);

    timing::time("session.create");

    if let Some(mid) = target_model {
        let available: Vec<&XyModelMeta> = agent.inner().model_registry().list().iter().collect();
        match resolver::resolve_model(&mid, &available, None) {
            Ok(resolved) => {
                if let Some(ref warning) = resolved.warning {
                    warnings.push(BootstrapWarning::ModelResolutionWarning(warning.clone()));
                }
                let _ = agent.inner_mut().select_model(&resolved.model.id);
            }
            Err(msg) => {
                warnings.push(BootstrapWarning::ModelResolutionFailed(msg));
            }
        }
    }

    // Reconstruct the same session store injected into the agent by
    // composition::build_agent. It is dir-backed, so a fresh instance points at
    // the same backing data as the agent's internal copy — this lets an
    // InProcessDriver serve session commands without composition::build_agent
    // having to return its injected ports.
    let store: Arc<dyn crate::runtime_protocol::XySessionStore> =
        Arc::new(SessionManager::new(SessionManager::default_dir()));

    Ok(BootstrappedAgent {
        agent,
        session_id,
        warnings,
        store,
        mcp_servers,
    })
}

/// Read the API key for a provider from environment variables.
fn resolve_api_key(kind: crate::domain::model::XyModelKind) -> Option<String> {
    match kind {
        crate::domain::model::XyModelKind::OpenAi => std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_KEY"))
            .ok(),
        crate::domain::model::XyModelKind::Anthropic => std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_KEY"))
            .ok(),
        crate::domain::model::XyModelKind::Fake => Some(String::new()),
    }
}

fn queue_mode_from_settings(
    mode: crate::infra::settings::types::SteeringMode,
) -> crate::agent::session::QueueMode {
    match mode {
        crate::infra::settings::types::SteeringMode::All => crate::agent::session::QueueMode::All,
        crate::infra::settings::types::SteeringMode::OneAtATime => {
            crate::agent::session::QueueMode::OneAtATime
        }
    }
}
