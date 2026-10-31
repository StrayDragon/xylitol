use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use xylitol_runtime::InProcessReloadHook;
use xylitol_runtime::XyInProcessDriver;
use xylitol_runtime::driver::{
    ReloadContext, ReloadParticipant, ReloadRegistry, ReloadStepReport, RuntimeReloadReport,
    XyDriverError,
};

pub struct ProductReload;

pub fn default_reload_registry() -> Arc<ReloadRegistry> {
    let registry = Arc::new(ReloadRegistry::new());
    registry.register(Arc::new(SkillsParticipant));
    registry.register(Arc::new(PrepareParticipant));
    registry.register(Arc::new(ContextParticipant));
    registry.register(Arc::new(ModelsParticipant));
    registry
}

#[async_trait]
impl InProcessReloadHook for ProductReload {
    async fn reload(
        &self,
        driver: &mut XyInProcessDriver,
        cancel: &CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        run_participants(driver, cancel).await
    }
}

async fn run_participants(
    driver: &mut XyInProcessDriver,
    cancel: &CancellationToken,
) -> Result<RuntimeReloadReport, XyDriverError> {
    let Some(cwd) = driver.reload_cwd().cloned() else {
        return Ok(RuntimeReloadReport::noop());
    };
    let agent_dir = driver
        .reload_agent_dir()
        .cloned()
        .unwrap_or_else(xylitol_runtime::trust::TrustManager::default_dir);
    let project_trusted = xylitol_runtime::trust::TrustManager::new(agent_dir.clone())
        .is_trusted(&cwd.display().to_string());
    let ctx = ReloadContext {
        cwd,
        agent_dir,
        project_trusted,
    };
    let mut steps = Vec::new();
    for participant in driver.reload_registry().snapshot() {
        if cancel.is_cancelled() {
            return Ok(RuntimeReloadReport {
                steps,
                cancelled: true,
            });
        }
        let step = match participant.reload(driver, &ctx).await {
            Ok(message) => ReloadStepReport {
                step: participant.name(),
                ok: true,
                message,
            },
            Err(message) => ReloadStepReport {
                step: participant.name(),
                ok: false,
                message,
            },
        };
        steps.push(step);
    }
    Ok(RuntimeReloadReport {
        steps,
        cancelled: false,
    })
}

struct SkillsParticipant;

#[async_trait]
impl ReloadParticipant for SkillsParticipant {
    fn name(&self) -> &'static str {
        "skills"
    }

    async fn reload(
        &self,
        driver: &mut XyInProcessDriver,
        ctx: &ReloadContext,
    ) -> Result<String, String> {
        let skills =
            crate::bootstrap::reload_skills(driver, &ctx.cwd, &ctx.agent_dir, ctx.project_trusted);
        Ok(if skills.names.is_empty() {
            "0 skills".into()
        } else {
            format!("{} skill(s): {}", skills.count, skills.names.join(", "))
        })
    }
}

struct PrepareParticipant;

#[async_trait]
impl ReloadParticipant for PrepareParticipant {
    fn name(&self) -> &'static str {
        "mcp"
    }

    async fn reload(
        &self,
        _driver: &mut XyInProcessDriver,
        _ctx: &ReloadContext,
    ) -> Result<String, String> {
        Ok("session/prepare reload fired".into())
    }
}

struct ContextParticipant;

#[async_trait]
impl ReloadParticipant for ContextParticipant {
    fn name(&self) -> &'static str {
        "context"
    }

    async fn reload(
        &self,
        driver: &mut XyInProcessDriver,
        ctx: &ReloadContext,
    ) -> Result<String, String> {
        let context = crate::bootstrap::reload_prompt_context(
            driver,
            &ctx.cwd,
            &ctx.agent_dir,
            ctx.project_trusted,
            config_system_prompt(driver),
        );
        Ok(format!(
            "{} context file(s), system={}, append={}",
            context.context_file_count, context.has_system_prompt, context.append_count
        ))
    }
}

struct ModelsParticipant;

#[async_trait]
impl ReloadParticipant for ModelsParticipant {
    fn name(&self) -> &'static str {
        "models"
    }

    async fn reload(
        &self,
        driver: &mut XyInProcessDriver,
        _ctx: &ReloadContext,
    ) -> Result<String, String> {
        let Some(config) = load_config(driver) else {
            return Err("config load failed; kept previous registry".into());
        };
        let mut warnings = Vec::new();
        let registry = crate::bootstrap::build_model_registry(Some(&config), &mut warnings);
        if registry.is_empty() {
            return Err("config has no models; kept previous registry".into());
        }
        let previous = driver.agent.model_registry().len();
        driver.agent.replace_model_registry(registry);
        let current = driver
            .agent
            .current_model()
            .map(|meta| meta.id.clone())
            .unwrap_or_else(|| "none".into());
        Ok(format!(
            "{} model(s) (was {}), current {current}",
            driver.agent.model_registry().len(),
            previous
        ))
    }
}

fn load_config(
    driver: &XyInProcessDriver,
) -> Option<xylitol_official_plugin_config::types::AppConfig> {
    xylitol_official_plugin_config::loader::load_app_config(
        driver.reload_config_path().map(|path| path.as_path()),
    )
    .ok()
}

fn config_system_prompt(driver: &XyInProcessDriver) -> Option<String> {
    load_config(driver)?
        .resolve_default_profile()
        .ok()?
        .system_prompt
        .clone()
}
