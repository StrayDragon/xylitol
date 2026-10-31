use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use xylitol_runtime::InProcessReloadHook;
use xylitol_runtime::XyInProcessDriver;
use xylitol_runtime::driver::{ReloadStepReport, RuntimeReloadReport, XyDriverError};

pub struct ProductReload;

#[async_trait]
impl InProcessReloadHook for ProductReload {
    async fn reload(
        &self,
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

        let mut steps = Vec::new();
        let trust_mgr = xylitol_runtime::trust::TrustManager::new(agent_dir.clone());
        let cwd_str = cwd.display().to_string();
        let project_trusted = trust_mgr.is_trusted(&cwd_str);

        let app_config = xylitol_official_plugin_config::loader::load_app_config(
            driver.reload_config_path().map(|path| path.as_path()),
        )
        .ok();
        let config_system_prompt = app_config
            .as_ref()
            .and_then(|cfg| cfg.resolve_default_profile().ok())
            .and_then(|p| p.system_prompt.clone());

        if cancel.is_cancelled() {
            return Ok(RuntimeReloadReport {
                steps,
                cancelled: true,
            });
        }

        let skills = crate::bootstrap::reload_skills(driver, &cwd, &agent_dir, project_trusted);
        let skills_msg = if skills.names.is_empty() {
            "0 skills".into()
        } else {
            format!("{} skill(s): {}", skills.count, skills.names.join(", "))
        };
        steps.push(ReloadStepReport {
            step: "skills",
            ok: true,
            message: skills_msg,
        });

        if cancel.is_cancelled() {
            return Ok(RuntimeReloadReport {
                steps,
                cancelled: true,
            });
        }

        steps.push(ReloadStepReport {
            step: "mcp",
            ok: true,
            message: "session/prepare reload fired".into(),
        });

        if cancel.is_cancelled() {
            return Ok(RuntimeReloadReport {
                steps,
                cancelled: true,
            });
        }

        let ctx = crate::bootstrap::reload_prompt_context(
            driver,
            &cwd,
            &agent_dir,
            project_trusted,
            config_system_prompt,
        );
        steps.push(ReloadStepReport {
            step: "context",
            ok: true,
            message: format!(
                "{} context file(s), system={}, append={}",
                ctx.context_file_count, ctx.has_system_prompt, ctx.append_count
            ),
        });

        if cancel.is_cancelled() {
            return Ok(RuntimeReloadReport {
                steps,
                cancelled: true,
            });
        }

        match app_config.as_ref() {
            Some(cfg) => {
                let mut warnings = Vec::new();
                let registry = crate::bootstrap::build_model_registry(Some(cfg), &mut warnings);
                if registry.is_empty() {
                    steps.push(ReloadStepReport {
                        step: "models",
                        ok: false,
                        message: "config has no models; kept previous registry".into(),
                    });
                } else {
                    let previous = driver.agent.model_registry().len();
                    driver.agent.replace_model_registry(registry);
                    let current = driver
                        .agent
                        .current_model()
                        .map(|meta| meta.id.clone())
                        .unwrap_or_else(|| "none".into());
                    steps.push(ReloadStepReport {
                        step: "models",
                        ok: true,
                        message: format!(
                            "{} model(s) (was {}), current {current}",
                            driver.agent.model_registry().len(),
                            previous
                        ),
                    });
                }
            }
            None => {
                steps.push(ReloadStepReport {
                    step: "models",
                    ok: false,
                    message: "config load failed; kept previous registry".into(),
                });
            }
        }

        Ok(RuntimeReloadReport {
            steps,
            cancelled: false,
        })
    }
}
