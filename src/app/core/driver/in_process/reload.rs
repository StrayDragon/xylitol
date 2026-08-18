use super::XyDriverError;
use super::mcp::McpBootState;
use super::types::{
    ProjectTrustMode, ProjectTrustPersistReport, ReloadStepReport, RuntimeReloadReport,
};

impl super::XyInProcessDriver {
    pub(super) async fn reload_runtime(
        &mut self,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        use crate::app::core::composition::McpReloadOutcome;

        let mut state = self.reload.take();
        if state.is_none() {
            return Ok(RuntimeReloadReport::noop());
        }
        use futures::FutureExt;

        // Panic-safe put-back: `state` is taken out of `self.reload` so the body can
        // pass `&mut self` to helpers while owning the reload state. If the body
        // panics mid-flight, the catch arm below restores `reload` so the driver is
        // not left without reload state.
        let body = async {
            let st = state.as_mut().expect("reload state present");
            let mut steps = Vec::new();
            let mut cancelled = false;

            // Re-read trust store so `/trust` + later `/reload` picks up new decisions (c1105).
            // Prefer reload `agent_dir` (same as product `~/.xylitol`) so tests need not mutate HOME.
            let trust_mgr = crate::infra::trust::TrustManager::new(st.agent_dir.clone());
            let cwd_str = st.cwd.display().to_string();
            st.project_trusted = trust_mgr.is_trusted(&cwd_str);

            let app_config = crate::infra::config::loader::load_app_config(None).ok();
            st.mcp_servers = crate::app::core::mcp_spec::McpServerSpec::from_infra_list(
                app_config.as_ref().and_then(|c| c.mcp_servers.clone()),
            )
            .unwrap_or_default();
            let config_system_prompt = app_config
                .as_ref()
                .and_then(|cfg| cfg.resolve_default_profile().ok())
                .and_then(|p| p.system_prompt.clone());

            if cancel.is_cancelled() {
                self.reload = state.take();
                return Ok(RuntimeReloadReport {
                    steps,
                    cancelled: true,
                });
            }

            let (cwd, agent_dir, project_trusted) =
                (st.cwd.clone(), st.agent_dir.clone(), st.project_trusted);
            let skills =
                crate::app::core::bootstrap::reload_skills(self, &cwd, &agent_dir, project_trusted);
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
                self.reload = state.take();
                return Ok(RuntimeReloadReport {
                    steps,
                    cancelled: true,
                });
            }

            let mcp_servers = st.mcp_servers.clone();
            let mcp_result = st.mcp.reload(self, &mcp_servers, cancel).await;

            match mcp_result {
                Ok(McpReloadOutcome::Cancelled) => {
                    cancelled = true;
                    steps.push(ReloadStepReport {
                        step: "mcp",
                        ok: true,
                        message: "cancelled before install".into(),
                    });
                }
                Ok(McpReloadOutcome::Installed) => {
                    // c1900: reload is an explicit re-freeze; bootstrap mark settled.
                    self.mcp_boot = McpBootState::Settled;
                    let connected = st.mcp.connected_servers().await;
                    let diags = st.mcp.diagnostics().await;
                    let configured = st.mcp_servers.len();
                    if diags.is_empty() {
                        let ids: Vec<_> = connected.iter().map(|s| s.id.as_str()).collect();
                        steps.push(ReloadStepReport {
                            step: "mcp",
                            ok: true,
                            message: format!(
                                "{configured} configured, {} connected [{}]",
                                connected.len(),
                                ids.join(", ")
                            ),
                        });
                    } else {
                        let detail = diags
                            .iter()
                            .map(|d| format!("{}: {}", d.server, d.message))
                            .collect::<Vec<_>>()
                            .join("; ");
                        steps.push(ReloadStepReport {
                            step: "mcp",
                            ok: !connected.is_empty(),
                            message: format!(
                                "{configured} configured, {} connected; diagnostics: {detail}",
                                connected.len()
                            ),
                        });
                    }
                }
                Err(e) => steps.push(ReloadStepReport {
                    step: "mcp",
                    ok: false,
                    message: e.to_string(),
                }),
            }

            if cancelled || cancel.is_cancelled() {
                self.reload = state.take();
                return Ok(RuntimeReloadReport {
                    steps,
                    cancelled: true,
                });
            }

            let (cwd, agent_dir, project_trusted) =
                (st.cwd.clone(), st.agent_dir.clone(), st.project_trusted);
            let ctx = crate::app::core::bootstrap::reload_prompt_context(
                self,
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

            self.reload = state.take();
            Ok(RuntimeReloadReport {
                steps,
                cancelled: false,
            })
        };

        match std::panic::AssertUnwindSafe(body).catch_unwind().await {
            Ok(result) => result,
            Err(payload) => {
                // Restore the taken reload state after an unwind, then re-raise.
                self.reload = state;
                std::panic::resume_unwind(payload);
            }
        }
    }

    pub(super) fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        let cwd = self
            .reload
            .as_ref()
            .map(|s| s.cwd.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let cwd_str = cwd.display().to_string();
        let trust_dir = self
            .reload
            .as_ref()
            .map(|s| s.agent_dir.clone())
            .unwrap_or_else(crate::infra::trust::TrustManager::default_dir);
        let mgr = crate::infra::trust::TrustManager::new(trust_dir);
        let options = mgr.get_trust_options(&cwd_str, false);
        let opt = match mode {
            ProjectTrustMode::TrustCwd => options.first(),
            ProjectTrustMode::TrustParent => {
                options.iter().find(|o| o.label.starts_with("Trust parent"))
            }
            ProjectTrustMode::Deny => options.iter().find(|o| o.label == "Do not trust"),
        };
        let Some(opt) = opt else {
            return Err(match mode {
                ProjectTrustMode::TrustParent => {
                    XyDriverError::invalid_input("no parent folder to trust")
                }
                _ => XyDriverError::invalid_input("trust option unavailable"),
            });
        };
        if !opt.updates.is_empty() {
            mgr.apply_updates(&opt.updates)?;
        }
        let saved = opt.saved_path.clone().unwrap_or_else(|| cwd_str.clone());
        let verb = if opt.trusted { "trusted" } else { "denied" };
        Ok(ProjectTrustPersistReport {
            trusted: opt.trusted,
            saved_path: opt.saved_path.clone(),
            message: format!(
                "Project {verb} at {saved}. {}",
                ProjectTrustPersistReport::RELOAD_HINT
            ),
        })
    }
}
