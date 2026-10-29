use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::composition::{BuildAgentOptions, build_agent};
use xylitol_official_plugin_config::loader::load_app_config_detailed_with;
use xylitol_official_plugin_obs as timing;
use xylitol_official_plugin_session::SessionManager;
use xylitol_plugin_sdk::model::XyModelMeta;
use xylitol_runtime::agent::ModelRegistry;
use xylitol_runtime::agent::model::resolver;

fn process_get_env(k: &str) -> Option<String> {
    if let Ok(v) = std::env::var(k) {
        return Some(v);
    }
    if k == "HOME" {
        return dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
    }
    None
}

fn xylitol_home_dir(get_env: &impl Fn(&str) -> Option<String>) -> PathBuf {
    get_env("HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
}

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

pub struct BootstrapInput {
    pub config_path: Option<std::path::PathBuf>,
    pub session: Option<String>,
    pub model: Option<String>,
    pub trust_override: Option<bool>,
    pub interactive: bool,
    pub caller: &'static str,
    pub ephemeral_sessions: bool,
    pub static_tools: Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>,
    pub agent_profile: Option<String>,
}

#[derive(Debug)]
pub enum BootstrapWarning {
    ModelEntrySkipped(String),
    NoApiKey { provider: String },
    ProjectNotTrusted { reason: String },
    RestoringSession { session: String, path: String },
    ModelResolutionWarning(String),
    ModelResolutionFailed(String),
}

pub const UNSET_MODEL_DISPLAY: &str = "NOT-SET";

pub struct BootstrappedAgent {
    pub(crate) agent: xylitol_runtime::agent::AgentRuntime,
    pub session_id: String,
    pub warnings: Vec<BootstrapWarning>,
    pub store: Arc<dyn xylitol_plugin_sdk::XySessionStore>,
    pub mcp_servers: Option<Vec<crate::mcp_spec::McpServerSpec>>,
    tool_features: crate::driver::ToolFeatures,
    driver_context: crate::driver::DriverContext,
}

pub struct BootstrappedRuntime {
    pub driver: crate::driver::XyInProcessDriver,
    pub session_id: String,
    pub warnings: Vec<BootstrapWarning>,
    pub mcp_servers: Option<Vec<crate::mcp_spec::McpServerSpec>>,
}

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
    store: Arc<dyn xylitol_plugin_sdk::XySessionStore>,
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
            self.agent.restore_thinking_level(level);
        }
        if let Some(name) = restored_session.session_name {
            self.agent.set_session_name_fact(Some(name.clone()));
            xylitol_official_plugin_provider_openai::provider::set_obs_session_name(Some(
                name.as_str(),
            ));
        }
        let mcp_ids = self
            .mcp_servers
            .as_ref()
            .map(|servers| servers.iter().map(|server| server.name.clone()).collect())
            .unwrap_or_default();
        let mut driver = crate::driver::assemble_driver_with_features_and_context(
            self.agent,
            self.store,
            self.tool_features,
            self.driver_context,
        );
        driver.set_mcp_server_ids(mcp_ids);
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

    pub fn into_driver(self) -> crate::driver::XyInProcessDriver {
        self.into_runtime().driver
    }
}

pub struct ResolvedAssembly {
    pub model_registry: ModelRegistry,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    pub skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>,
    pub cwd: String,
    pub compaction_settings: Option<xylitol_runtime::agent::compaction::CompactionSettings>,
    pub permission: Option<Arc<dyn xylitol_plugin_sdk::XyPermission>>,
    pub steering_mode: xylitol_runtime::agent::QueueMode,
    pub follow_up_mode: xylitol_runtime::agent::QueueMode,
    pub default_profile_model: Option<String>,
    pub session_id: String,
    pub session_file_exists: bool,
    pub warnings: Vec<BootstrapWarning>,
    pub mcp_servers: Option<Vec<crate::mcp_spec::McpServerSpec>>,
    pub hooks_config: xylitol_official_plugin_config::types::HooksConfig,
    pub default_thinking_level: Option<String>,
    pub thinking_budgets: Option<xylitol_plugin_sdk::model::ThinkingBudgets>,
    pub batch_mode: xylitol_plugin_sdk::XyBatchMode,
    pub max_turns: Option<u32>,
    pub tool_features: crate::driver::ToolFeatures,
    pub project_trusted: bool,
}

impl ResolvedAssembly {
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

#[derive(Debug)]
pub enum BootstrapError {
    ConfigLoadFailed(String),
    ConfigLoadedZeroModels,
    NoModelsAvailable,
    BuildFailed(String),
    KernelFailed(String),
    AgentProfileNotFound(String),
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
            BootstrapError::KernelFailed(e) => write!(f, "kernel boot failed: {e}"),
            BootstrapError::AgentProfileNotFound(name) => write!(
                f,
                "agent profile `{name}` not found (check `agents.profiles` in config)"
            ),
        }
    }
}

impl std::error::Error for BootstrapError {}

fn resolve_profile_assembly(
    app_config: Option<&xylitol_official_plugin_config::types::AppConfig>,
    agent_profile: Option<&str>,
) -> Result<(Option<String>, Option<String>, Vec<String>), BootstrapError> {
    let resolved = match agent_profile {
        Some(name) => {
            let Some(cfg) = app_config else {
                return Err(BootstrapError::AgentProfileNotFound(name.to_string()));
            };
            if !cfg.agents.profiles.contains_key(name) {
                return Err(BootstrapError::AgentProfileNotFound(name.to_string()));
            }
            Some(
                cfg.resolve_profile(name)
                    .map_err(|e| BootstrapError::ConfigLoadFailed(e.to_string()))?,
            )
        }
        None => app_config.and_then(|cfg| cfg.resolve_default_profile().ok()),
    };
    Ok((
        resolved
            .as_ref()
            .and_then(|profile| profile.system_prompt.clone()),
        resolved.as_ref().map(|profile| profile.model_id.clone()),
        resolved
            .as_ref()
            .map(|profile| profile.tools.clone())
            .unwrap_or_default(),
    ))
}

fn build_model_registry(
    app_config: Option<&xylitol_official_plugin_config::types::AppConfig>,
    warnings: &mut Vec<BootstrapWarning>,
) -> ModelRegistry {
    let mut model_registry = ModelRegistry::new();
    let Some(cfg) = app_config else {
        return model_registry;
    };
    let mut missing_key_providers = std::collections::BTreeSet::new();
    for (alias, entry) in &cfg.model.models {
        let api_key = entry.normalized_api_key();
        if api_key.is_empty() {
            missing_key_providers.insert(entry.provider.provider_name().to_string());
        }
        match entry.to_model_meta(alias).map_err(|e| e.to_string()) {
            Ok(meta) => model_registry.register(meta),
            Err(e) => warnings.push(BootstrapWarning::ModelEntrySkipped(format!(
                "models.{alias}: {e}"
            ))),
        }
    }
    for provider in missing_key_providers {
        warnings.push(BootstrapWarning::NoApiKey { provider });
    }
    model_registry
}

fn restore_session(
    input: &BootstrapInput,
    agent_home: &Path,
    warnings: &mut Vec<BootstrapWarning>,
) -> bool {
    let sessions_dir = agent_home.join("sessions");
    let session_mgr = if input.ephemeral_sessions {
        SessionManager::in_memory()
    } else {
        std::fs::create_dir_all(&sessions_dir).ok();
        SessionManager::new(sessions_dir.clone())
    };
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
    session_file_exists
}

fn resolve_project_trust(
    agent_home: &Path,
    cwd: &str,
    input: &BootstrapInput,
    warnings: &mut Vec<BootstrapWarning>,
) -> bool {
    let trust_manager = xylitol_runtime::trust::TrustManager::new(agent_home);
    let trust_cwd = cwd.to_string();
    let trust_resolution = xylitol_runtime::trust::resolve_project_trusted(
        &trust_manager,
        cwd,
        input.trust_override,
        xylitol_runtime::trust::DefaultProjectTrust::default(),
        input.interactive,
        |options| {
            if input.interactive {
                xylitol_runtime::trust::prompt_trust_options_stdio(&trust_cwd, options)
            } else {
                None
            }
        },
    );
    let project_trusted = trust_resolution.trusted;
    if !project_trusted {
        warnings.push(BootstrapWarning::ProjectNotTrusted {
            reason: match trust_resolution.reason {
                xylitol_runtime::trust::TrustReason::Override => "overridden via --no-trust".into(),
                xylitol_runtime::trust::TrustReason::Store => "denied in trust store".into(),
                xylitol_runtime::trust::TrustReason::FallbackNoUi => {
                    "no interactive prompt available".into()
                }
                _ => "policy".into(),
            },
        });
    }
    project_trusted
}

struct ResourceBundle {
    context_files: Vec<(String, String)>,
    loader_system_prompt: Option<String>,
    append_system_prompt: Vec<String>,
    skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>,
}

fn load_resources(agent_home: &Path, cwd_path: &Path, project_trusted: bool) -> ResourceBundle {
    let loader_cwd = if project_trusted {
        cwd_path.to_path_buf()
    } else {
        std::env::temp_dir()
    };
    let loader =
        xylitol_runtime::resource::DefaultResourceLoader::new(loader_cwd, agent_home.to_path_buf());
    let context_files = loader
        .get_agents_files()
        .iter()
        .map(|f| (f.path.to_string_lossy().into_owned(), f.content.clone()))
        .collect();
    let loader_system_prompt = loader.get_system_prompt().map(String::from);
    let append_system_prompt = loader.get_append_system_prompt().to_vec();
    let skills = loader.get_skills().0.to_vec();
    ResourceBundle {
        context_files,
        loader_system_prompt,
        append_system_prompt,
        skills,
    }
}

struct RuntimeSettingsBundle {
    compaction_settings: Option<xylitol_runtime::agent::compaction::CompactionSettings>,
    steering_mode: xylitol_runtime::agent::QueueMode,
    follow_up_mode: xylitol_runtime::agent::QueueMode,
    default_thinking_level: Option<String>,
    thinking_budgets: Option<xylitol_plugin_sdk::model::ThinkingBudgets>,
}

fn load_runtime_settings(
    app_config: Option<&xylitol_official_plugin_config::types::AppConfig>,
    agent_home: &Path,
    cwd_path: &Path,
    project_trusted: bool,
) -> RuntimeSettingsBundle {
    let settings_cwd = if project_trusted {
        cwd_path.to_path_buf()
    } else {
        std::env::temp_dir()
    };
    let settings_mgr = xylitol_official_plugin_config::settings::SettingsManager::from_files(
        &settings_cwd,
        agent_home,
        project_trusted,
    );
    let compaction = {
        let from_settings = settings_mgr.get_settings().compaction.as_ref();
        let from_config = app_config.and_then(|c| c.compaction.as_ref());
        Some(xylitol_plugin_sdk::merge_compaction_runtime(
            from_config,
            from_settings,
        ))
    };
    RuntimeSettingsBundle {
        compaction_settings: compaction,
        steering_mode: queue_mode_from_settings(settings_mgr.get_steering_mode()),
        follow_up_mode: queue_mode_from_settings(settings_mgr.get_follow_up_mode()),
        default_thinking_level: settings_mgr.get_settings().default_thinking_level.clone(),
        thinking_budgets: settings_mgr.get_thinking_budgets().map(Into::into),
    }
}

fn resolve_tool_features(
    app_config: Option<&xylitol_official_plugin_config::types::AppConfig>,
    profile_tools: Vec<String>,
) -> crate::driver::ToolFeatures {
    let mut features = app_config
        .map(crate::driver::ToolFeatures::from_config)
        .unwrap_or_default();
    if !profile_tools.is_empty() {
        features.tool_selection = profile_tools;
    }
    features
}

pub fn resolve_assembly(input: &BootstrapInput) -> Result<ResolvedAssembly, BootstrapError> {
    resolve_assembly_with(
        input,
        process_get_env,
        std::env::current_dir().ok().as_deref(),
    )
}

pub fn resolve_assembly_with(
    input: &BootstrapInput,
    get_env: impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> Result<ResolvedAssembly, BootstrapError> {
    let cli_config_path = input.config_path.as_deref();
    let mut warnings: Vec<BootstrapWarning> = Vec::new();

    let loaded = match load_app_config_detailed_with(cli_config_path, &get_env, cwd) {
        Ok(loaded) => loaded,
        Err(e) => return Err(BootstrapError::ConfigLoadFailed(e.to_string())),
    };
    let from_yaml_layers = loaded.from_yaml_layers;
    let app_config = Some(loaded.config);
    timing::time("config.load");

    let mut warnings_for_registry = Vec::new();
    let model_registry = build_model_registry(app_config.as_ref(), &mut warnings_for_registry);
    warnings.extend(warnings_for_registry);

    if from_yaml_layers && model_registry.is_empty() {
        return Err(BootstrapError::ConfigLoadedZeroModels);
    }

    timing::time("model_registry.load");

    if model_registry.is_empty() {
        return Err(BootstrapError::NoModelsAvailable);
    }

    let agent_home = xylitol_home_dir(&get_env);
    let session_file_exists = restore_session(input, &agent_home, &mut warnings);
    timing::time("session.restore");

    let (config_system_prompt, default_profile_model, profile_tools) =
        resolve_profile_assembly(app_config.as_ref(), input.agent_profile.as_deref())?;

    let cwd_path = cwd
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    let cwd = cwd_path.to_string_lossy().to_string();

    let project_trusted = resolve_project_trust(&agent_home, &cwd, input, &mut warnings);

    let resources = load_resources(&agent_home, &cwd_path, project_trusted);
    let context_files = resources.context_files;
    let loader_system_prompt = resources.loader_system_prompt;
    let append_system_prompt = resources.append_system_prompt;
    let skills = resources.skills;

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

    let settings =
        load_runtime_settings(app_config.as_ref(), &agent_home, &cwd_path, project_trusted);
    let compaction_settings = settings.compaction_settings;
    let steering_mode = settings.steering_mode;
    let follow_up_mode = settings.follow_up_mode;
    let default_thinking_level = settings.default_thinking_level;
    let thinking_budgets = settings.thinking_budgets;

    let permission_engine = app_config.as_ref().and_then(|cfg| {
        cfg.security
            .permission
            .as_ref()
            .filter(|sc| sc.enabled)
            .map(|sc| sc.to_permission())
    });

    let system_prompt = loader_system_prompt.or(config_system_prompt);

    let session_id = input
        .session
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mcp_servers = crate::mcp_spec::McpServerSpec::from_infra_list(
        app_config.as_ref().and_then(|c| c.mcp_servers.clone()),
    );

    let hooks_config = app_config
        .as_ref()
        .map(|c| c.hooks.clone())
        .unwrap_or_default();

    let batch_mode = app_config
        .as_ref()
        .map(|c| xylitol_plugin_sdk::XyBatchMode::from(c.tool_batch.mode))
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
        tool_features: resolve_tool_features(app_config.as_ref(), profile_tools),
        project_trusted,
    })
}

pub fn bootstrap(input: BootstrapInput) -> Result<BootstrappedAgent, BootstrapError> {
    bootstrap_with(
        input,
        process_get_env,
        std::env::current_dir().ok().as_deref(),
    )
}

pub fn bootstrap_with(
    input: BootstrapInput,
    get_env: impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> Result<BootstrappedAgent, BootstrapError> {
    let model = input.model.clone();
    let static_tools = input.static_tools.clone();

    let mut assembly = resolve_assembly_with(&input, get_env, cwd)?;
    let session_id = assembly.session_id.clone();
    let mcp_servers = assembly.mcp_servers.clone();
    let default_thinking_level = assembly.default_thinking_level.clone();
    let thinking_budgets = assembly.thinking_budgets.clone();
    let max_turns = assembly.max_turns;
    let session_file_exists = assembly.session_file_exists;
    let tool_features = assembly.tool_features.clone();
    let driver_context = crate::driver::DriverContext {
        project_root: std::path::PathBuf::from(&assembly.cwd),
        trusted: assembly.project_trusted,
        static_tools,
    };
    let target_model = model.or_else(|| assembly.default_profile_model.clone());
    let mut warnings = std::mem::take(&mut assembly.warnings);

    let mut agent = build_agent(assembly.into_build_options())
        .map_err(|e| BootstrapError::BuildFailed(e.to_string()))?;

    if let Some(n) = max_turns.filter(|&n| n >= 1) {
        agent.set_should_stop_after_turn(Some(xylitol_runtime::agent::max_turns_stop_hook(n)));
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

    if !session_file_exists {
        agent.apply_default_thinking_level(default_thinking_level.as_deref());
    }
    if let Some(level) = requested_thinking_level
        && let Err(error) = block_on(agent.set_thinking_level(level.clone()))
    {
        log::warn!("bootstrap model:thinkingLevel rejected level={level} error={error}");
    }
    agent.set_thinking_budgets(thinking_budgets);

    let store = agent.session_store();

    Ok(BootstrappedAgent {
        agent,
        session_id,
        warnings,
        store,
        mcp_servers,
        tool_features,
        driver_context,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsReloadReport {
    pub names: Vec<String>,
    pub count: usize,
}

pub fn reload_skills(
    driver: &mut crate::driver::XyInProcessDriver,
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
        xylitol_runtime::resource::DefaultResourceLoader::new(loader_cwd, agent_dir.to_path_buf());
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptContextReloadReport {
    pub context_file_count: usize,
    pub has_system_prompt: bool,
    pub append_count: usize,
}

pub fn reload_prompt_context(
    driver: &mut crate::driver::XyInProcessDriver,
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
        xylitol_runtime::resource::DefaultResourceLoader::new(loader_cwd, agent_dir.to_path_buf());

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

fn queue_mode_from_settings(
    mode: xylitol_official_plugin_config::settings::types::SteeringMode,
) -> xylitol_runtime::agent::QueueMode {
    match mode {
        xylitol_official_plugin_config::settings::types::SteeringMode::All => {
            xylitol_runtime::agent::QueueMode::All
        }
        xylitol_official_plugin_config::settings::types::SteeringMode::OneAtATime => {
            xylitol_runtime::agent::QueueMode::OneAtATime
        }
    }
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod tests;
