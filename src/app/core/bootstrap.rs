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
//! (e.g. `--list-models`, which only needs the registry) call
//! `resolve_assembly` directly; surfaces that need a ready-to-run agent call
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
use crate::infra::config::loader::load_app_config_detailed;
use crate::infra::config::value::InfraSecretResolver;
use crate::infra::permission;
use crate::infra::session::SessionManager;
use crate::infra::timing;
use crate::protocol::resource::PromptTemplate;
use crate::protocol::types::XyModelMeta;

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

impl BootstrappedAgent {
    /// Consume into an [`crate::app::core::driver::XyInProcessDriver`] plus side-products (preferred path).
    pub fn into_runtime(mut self) -> BootstrappedRuntime {
        self.agent.inner_mut().set_session(self.session_id.clone());
        BootstrappedRuntime {
            driver: crate::app::core::driver::XyInProcessDriver::new(self.agent, self.store),
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
    pub compaction_threshold: f64,
    pub cwd: String,
    pub compaction_settings: Option<crate::agent::compaction::CompactionSettings>,
    pub permission: Option<Arc<dyn crate::protocol::ports::XyPermission>>,
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
    /// Three-tier script hook configuration.
    pub hooks_config: crate::infra::config::types::HooksConfig,
    /// Settings `defaultThinkingLevel` (camelCase JSON), if any.
    pub default_thinking_level: Option<String>,
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
            skills: self.skills,
            compaction_threshold: self.compaction_threshold,
            cwd: self.cwd,
            compaction_settings: self.compaction_settings,
            permission: self.permission,
            steering_mode: self.steering_mode,
            follow_up_mode: self.follow_up_mode,
            event_sink: None,
            hooks_config: self.hooks_config,
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
    let cli_config_path = input.config_path.as_deref();
    let mut warnings: Vec<BootstrapWarning> = Vec::new();

    // ── Step 1: load YAML config ──────────────────────────────────
    let loaded = match load_app_config_detailed(cli_config_path) {
        Ok(loaded) => loaded,
        Err(e) => return Err(BootstrapError::ConfigLoadFailed(e.to_string())),
    };
    let from_yaml_layers = loaded.from_yaml_layers;
    let app_config = Some(loaded.config);
    timing::time("config.load");

    // ── Step 2: build ModelRegistry ───────────────────────────────
    let secret_resolver: Arc<dyn crate::protocol::ports::XySecretResolver> =
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

            let levels = match crate::protocol::types::ThinkingLevel::resolve_configured_levels(
                entry.thinking,
                entry.thinking_levels.as_deref(),
            ) {
                Ok(ls) => ls,
                Err(e) => {
                    warnings.push(BootstrapWarning::ModelEntrySkipped(format!(
                        "models.{alias}: {e}"
                    )));
                    continue;
                }
            };
            if let Some(map) = &entry.thinking_level_map
                && let Err(e) = crate::protocol::types::validate_thinking_level_map(map)
            {
                warnings.push(BootstrapWarning::ModelEntrySkipped(format!(
                    "models.{alias}: {e}"
                )));
                continue;
            }
            let thinking_levels = levels.iter().map(|l| l.as_str().to_string()).collect();
            let thinking_level_map = entry.thinking_level_map.clone().unwrap_or_default();

            model_registry.register(XyModelMeta {
                id: alias.clone(),
                config: crate::protocol::model_config::XyModelConfig {
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
                thinking_levels,
                thinking_level_map,
            });
        }
    }

    // ce2: YAML present but zero registerable models → hard fail (no env gpt-4o).
    if from_yaml_layers && model_registry.is_empty() {
        return Err(BootstrapError::ConfigLoadedZeroModels);
    }

    // No YAML layers: env keys may populate the registry for discovery / --model.
    // MUST NOT auto-select here (m12); selection only via --model or profile model.
    if model_registry.is_empty() {
        for (provider_name, env_var, kind) in [
            (
                "openai",
                "OPENAI_API_KEY",
                crate::protocol::model_config::XyModelKind::OpenAi,
            ),
            (
                "anthropic",
                "ANTHROPIC_API_KEY",
                crate::protocol::model_config::XyModelKind::Anthropic,
            ),
        ] {
            if let Ok(key) = std::env::var(env_var)
                && let Some(model_id) =
                    crate::agent::model::registry::default_model_id_for_provider(provider_name)
            {
                model_registry.register(XyModelMeta {
                    id: model_id.to_string(),
                    config: crate::protocol::model_config::XyModelConfig {
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
                    thinking_levels: crate::protocol::types::ThinkingLevel::STANDARD
                        .iter()
                        .map(|l| l.as_str().to_string())
                        .collect(),
                    thinking_level_map: Default::default(),
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

    // ── Step 3: resolve default profile (drives system_prompt/model) ──
    let resolved_profile = app_config
        .as_ref()
        .and_then(|cfg| cfg.resolve_default_profile().ok());
    let config_system_prompt = resolved_profile
        .as_ref()
        .and_then(|p| p.system_prompt.clone());
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

    let (discovered_templates, context_files, loader_system_prompt, append_system_prompt, skills) = {
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
        let skills = loader.get_skills().0.to_vec();
        (templates, ctx, sys, append, skills)
    };
    log::debug!(
        "resource discovery resolved caller={} trusted={} cwd={} context_files={} templates={} append_system_prompt={} loader_system_prompt={} skills={}",
        input.caller,
        project_trusted,
        cwd,
        context_files.len(),
        discovered_templates.len(),
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
    let (compaction_settings, steering_mode, follow_up_mode, default_thinking_level) = {
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
        let default_thinking_level = settings_mgr.get_settings().default_thinking_level.clone();
        (
            compaction,
            steering_mode,
            follow_up_mode,
            default_thinking_level,
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

    Ok(ResolvedAssembly {
        model_registry,
        system_prompt,
        context_files,
        append_system_prompt,
        skills,
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
        hooks_config,
        default_thinking_level,
    })
}

/// Bootstrap a fully-assembled agent: resolve inputs, build once, select model.
///
/// Used by print / tui / server — surfaces that hold a single agent for their
/// lifetime. Ingredient-only paths (e.g. --list-models) use `resolve_assembly` directly.
pub fn bootstrap(input: BootstrapInput) -> Result<BootstrappedAgent, BootstrapError> {
    let model = input.model.clone();

    let mut assembly = resolve_assembly(&input)?;
    let discovered_templates = assembly.discovered_templates.clone();
    let session_id = assembly.session_id.clone();
    let mcp_servers = assembly.mcp_servers.clone();
    let default_thinking_level = assembly.default_thinking_level.clone();
    let target_model = model.or_else(|| assembly.default_profile_model.clone());
    let mut warnings = std::mem::take(&mut assembly.warnings);

    let mut agent = build_agent(assembly.into_build_options())
        .map_err(|e| BootstrapError::BuildFailed(e.to_string()))?;
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

    agent
        .inner_mut()
        .apply_default_thinking_level(default_thinking_level.as_deref());

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

/// Read the API key for a provider from environment variables.
fn resolve_api_key(kind: crate::protocol::model_config::XyModelKind) -> Option<String> {
    match kind {
        crate::protocol::model_config::XyModelKind::OpenAi => std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_KEY"))
            .ok(),
        crate::protocol::model_config::XyModelKind::Anthropic => std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_KEY"))
            .ok(),
        crate::protocol::model_config::XyModelKind::Fake => Some(String::new()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::composition::{BuildAgentOptions, build_agent};
    use crate::app::core::driver::XyInProcessDriver;
    use crate::protocol::ports::XySessionStore;
    use std::sync::Arc;

    fn make_driver() -> XyInProcessDriver {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        XyInProcessDriver::new(agent, store)
    }

    #[test]
    fn reload_prompt_context_trusted_injects_agents() {
        let project = tempfile::tempdir().unwrap();
        let agent_dir = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "TRUSTED_AGENTS_BODY").unwrap();

        let mut driver = make_driver();
        let report =
            reload_prompt_context(&mut driver, project.path(), agent_dir.path(), true, None);
        assert!(report.context_file_count >= 1);
        let sp = driver.system_prompt_for_test().unwrap_or_default();
        assert!(
            sp.contains("TRUSTED_AGENTS_BODY"),
            "trusted reload must inject project AGENTS.md"
        );
    }

    #[test]
    fn reload_prompt_context_untrusted_skips_project_agents() {
        let project = tempfile::tempdir().unwrap();
        let agent_dir = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "SECRET_PROJECT_AGENTS").unwrap();

        let mut driver = make_driver();
        let _ = reload_prompt_context(&mut driver, project.path(), agent_dir.path(), false, None);
        let sp = driver.system_prompt_for_test().unwrap_or_default();
        assert!(
            !sp.contains("SECRET_PROJECT_AGENTS"),
            "untrusted reload must not inject project AGENTS.md"
        );
    }

    fn write_skill(dir: &std::path::Path, name: &str) {
        let skill_dir = dir.join(".xylitol").join("skills").join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: test\n---\n\nbody\n"),
        )
        .unwrap();
    }

    #[test]
    fn reload_skills_trusted_injects_into_system_prompt() {
        let project = tempfile::tempdir().unwrap();
        let agent_dir = tempfile::tempdir().unwrap();
        write_skill(project.path(), "reload-demo");

        let mut driver = make_driver();
        let before_len = driver.system_prompt_for_test().unwrap_or_default().len();
        let report = reload_skills(&mut driver, project.path(), agent_dir.path(), true);
        assert!(report.names.iter().any(|n| n == "reload-demo"));
        let sp = driver.system_prompt_for_test().unwrap_or_default();
        assert!(sp.contains("reload-demo") || sp.contains("<available_skills>"));
        assert!(sp.len() >= before_len);
        assert!(
            driver
                .loaded_skill_names()
                .iter()
                .any(|n| n == "reload-demo")
        );
    }

    #[test]
    fn reload_skills_untrusted_skips_project_skill() {
        let project = tempfile::tempdir().unwrap();
        let agent_dir = tempfile::tempdir().unwrap();
        write_skill(project.path(), "secret-reload");

        let mut driver = make_driver();
        let _ = reload_skills(&mut driver, project.path(), agent_dir.path(), false);
        let sp = driver.system_prompt_for_test().unwrap_or_default();
        assert!(!sp.contains("secret-reload"));
        assert!(
            !driver
                .loaded_skill_names()
                .iter()
                .any(|n| n == "secret-reload")
        );
    }

    /// RAII env restore for bootstrap path tests.
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, val: &str) -> Self {
            let prev = std::env::var(key).ok();
            unsafe { std::env::set_var(key, val) };
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn config_template_error_is_hard_fail() {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("proj");
        let proj_xy = project.join(".xylitol");
        std::fs::create_dir_all(&proj_xy).unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();

        let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
        let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
        let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
        let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");

        std::fs::write(
            proj_xy.join("config.yaml"),
            "# doc {{ secret.KEY }}\nmodels:\n  default_model: q\n  models:\n    q:\n      provider: openai\n      model: m\n",
        )
        .unwrap();

        let err = match resolve_assembly(&BootstrapInput {
            config_path: None,
            session: None,
            model: None,
            trust_override: Some(true),
            interactive: false,
            caller: "test",
        }) {
            Ok(_) => panic!("template in comment must fail closed"),
            Err(e) => e,
        };
        assert!(
            matches!(err, BootstrapError::ConfigLoadFailed(_)),
            "got {err}"
        );
    }

    #[test]
    fn yaml_zero_models_hard_fail_no_env_gpt4o() {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("proj");
        let proj_xy = project.join(".xylitol");
        std::fs::create_dir_all(&proj_xy).unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();

        let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
        let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
        let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
        let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");

        // Explicit empty models map (config present, zero registerable models).
        std::fs::write(
            proj_xy.join("config.yaml"),
            "models:\n  default_model: x\n  models: {}\n",
        )
        .unwrap();

        let err = match resolve_assembly(&BootstrapInput {
            config_path: None,
            session: None,
            model: None,
            trust_override: Some(true),
            interactive: false,
            caller: "test",
        }) {
            Ok(_) => panic!("zero models from yaml must hard fail"),
            Err(e) => e,
        };
        assert!(
            matches!(err, BootstrapError::ConfigLoadedZeroModels),
            "got {err}"
        );
    }

    #[test]
    fn env_only_registers_but_bootstrap_does_not_select() {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();

        let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
        let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project.to_str().unwrap());
        let _cfg = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
        let _key = EnvGuard::set("OPENAI_API_KEY", "sk-test");
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };

        let assembly = resolve_assembly(&BootstrapInput {
            config_path: None,
            session: None,
            model: None,
            trust_override: Some(true),
            interactive: false,
            caller: "test",
        })
        .expect("env-only assembly");
        assert!(
            !assembly.model_registry.list().is_empty(),
            "env key should populate registry for discovery"
        );

        let boot = bootstrap(BootstrapInput {
            config_path: None,
            session: None,
            model: None,
            trust_override: Some(true),
            interactive: false,
            caller: "test",
        })
        .expect("bootstrap without --model");
        assert!(
            boot.agent.inner().current_model().is_none(),
            "must not silent-select gpt-4o"
        );
        assert_eq!(UNSET_MODEL_DISPLAY, "NOT-SET");
    }

    #[test]
    fn build_agent_with_skills_injects_available_skills_section() {
        use crate::protocol::resource::SkillInfo;
        use crate::protocol::source_info::{SourceInfo, SourceOrigin, SourceScope};

        let mut opts = BuildAgentOptions::default();
        opts.skills = vec![SkillInfo {
            name: "boot-skill".into(),
            description: Some("from build options".into()),
            source_info: SourceInfo {
                path: std::path::PathBuf::from("/tmp/boot/SKILL.md"),
                source: "user".into(),
                scope: SourceScope::User,
                origin: SourceOrigin::TopLevel,
                base_dir: None,
            },
            disable_model_invocation: false,
        }];
        let agent = build_agent(opts).expect("build");
        let sp = agent.inner().system_prompt().unwrap_or("");
        assert!(sp.contains("<available_skills>"));
        assert!(sp.contains("boot-skill"));
        assert!(sp.contains("Use the read tool"));
        assert_eq!(agent.loaded_skill_names(), vec!["boot-skill".to_string()]);
    }

    #[test]
    fn untrusted_reload_still_loads_user_global_skills() {
        let project = tempfile::tempdir().unwrap();
        let agent_dir = tempfile::tempdir().unwrap();
        write_skill(project.path(), "project-only");
        let user_skill = agent_dir.path().join("skills").join("user-global");
        std::fs::create_dir_all(&user_skill).unwrap();
        std::fs::write(
            user_skill.join("SKILL.md"),
            "---\nname: user-global\ndescription: always\n---\n",
        )
        .unwrap();

        let mut driver = make_driver();
        let report = reload_skills(&mut driver, project.path(), agent_dir.path(), false);
        assert!(
            report.names.iter().any(|n| n == "user-global"),
            "untrusted must still load user skills; got {:?}",
            report.names
        );
        assert!(!report.names.iter().any(|n| n == "project-only"));
        let sp = driver.system_prompt_for_test().unwrap_or_default();
        assert!(sp.contains("user-global"));
        assert!(!sp.contains("project-only"));
    }
}
