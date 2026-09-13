use std::sync::Arc;

use super::types::LoadedResourcesSnapshot;

pub(super) type McpToolList = Vec<Arc<dyn crate::protocol::ports::XyTool>>;
pub(super) type McpDiscoverOk = Option<(Arc<crate::infra::mcp::McpClientManager>, McpToolList)>;

/// Whether connecting progress should invalidate the TUI loaded-resources strip.
///
/// `last_ui_label`: `None` = never published; `Some(label)` = last published value.
pub(super) fn mcp_progress_needs_ui_refresh(
    last_ui_label: &mut Option<Option<String>>,
    current: Option<String>,
) -> bool {
    if last_ui_label.as_ref() == Some(&current) {
        return false;
    }
    *last_ui_label = Some(current);
    true
}

pub(super) enum McpBootState {
    Idle,
    Running {
        handle: tokio::task::JoinHandle<McpDiscoverOk>,
        progress: Arc<tokio::sync::Mutex<crate::infra::mcp::McpConnectProgress>>,
        /// Last connecting label published to the TUI; skip refresh when unchanged.
        last_ui_label: Option<Option<String>>,
    },
    /// Rebuild ToolSet off the TUI tick (default_tools + MCP merge can hitch).
    Rebuilding {
        handle: tokio::task::JoinHandle<crate::agent::tools::ToolSet>,
        manager: Arc<crate::infra::mcp::McpClientManager>,
        tool_n: usize,
        settle_started: std::time::Instant,
    },
    /// Tools already applied; system prompt text building off the tick path.
    Settling {
        handle: tokio::task::JoinHandle<String>,
    },
    Settled,
}

impl super::XyInProcessDriver {
    /// One-line MCP status for startup logs (empty when reload/MCP disabled).
    pub async fn mcp_status_summary(&self) -> Option<String> {
        let state = self.reload.as_ref()?;
        if state.mcp_servers.is_empty() {
            return None;
        }
        let connected = state.mcp.connected_servers().await;
        let diags = state.mcp.diagnostics().await;
        let ids: Vec<_> = connected.iter().map(|s| s.id.as_str()).collect();
        let mut line = format!(
            "MCP: {} configured, {} connected [{}]",
            state.mcp_servers.len(),
            connected.len(),
            ids.join(", ")
        );
        if !diags.is_empty() {
            let detail = diags
                .iter()
                .map(|d| format!("{}: {}", d.server, d.message))
                .collect::<Vec<_>>()
                .join("; ");
            line.push_str(&format!("; diagnostics: {detail}"));
        }
        Some(line)
    }

    /// Wait settle/timeout then freeze the current tool table if not already frozen.
    ///
    /// On gate timeout while still `Running`, detach the discover handle so UI leaves
    /// `connecting i/n` immediately; late results apply via [`Self::poll_mcp_bootstrap`].
    pub(super) async fn ensure_tool_table_frozen(&mut self) {
        use crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT;

        if self.agent.is_tools_frozen() {
            return;
        }
        self.agent.begin_tool_gating();

        if matches!(self.mcp_boot, McpBootState::Idle) {
            if self
                .reload
                .as_ref()
                .is_some_and(|s| !s.mcp_servers.is_empty())
            {
                self.begin_mcp_bootstrap().await;
            } else {
                self.mcp_boot = McpBootState::Settled;
            }
        }

        let deadline = std::time::Instant::now() + MCP_FIRST_TURN_GATE_TIMEOUT;
        while self.mcp_blocks_agent() && std::time::Instant::now() < deadline {
            let _ = self.poll_mcp_bootstrap().await;
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        if matches!(self.mcp_boot, McpBootState::Running { .. }) {
            log::warn!(
                target: "xylitol::mcp",
                "MCP first-turn gate timed out after {:?}; detaching bootstrap and freezing subset",
                MCP_FIRST_TURN_GATE_TIMEOUT
            );
            self.detach_running_bootstrap_after_gate_timeout();
            self.mcp_gate_notice = Some(
                "MCP gate timed out — tools frozen with armed subset (see /mcp). /reload to retry."
                    .into(),
            );
        } else if self.mcp_blocks_agent() {
            // Rebuilding/Settling past deadline: finish promptly (bounded).
            let extra = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while self.mcp_blocks_agent() && std::time::Instant::now() < extra {
                let _ = self.poll_mcp_bootstrap().await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }

        let tools = self.agent.tools_snapshot();
        self.agent.freeze_tools(tools);
    }

    /// Detach in-flight `Running` discover so UI can show Settled; apply later on poll.
    fn detach_running_bootstrap_after_gate_timeout(&mut self) {
        let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Settled);
        if let McpBootState::Running { handle, .. } = prev
            && let Some(old) = self.late_mcp_discover.replace(handle)
        {
            old.abort();
        }
    }

    /// Take one-shot gate timeout notice for TUI fixed zone / scroll.
    pub(super) fn take_mcp_gate_notice_inner(&mut self) -> Option<String> {
        self.mcp_gate_notice.take()
    }

    /// Arm first-turn / re-gate freeze without blocking the host loop (TUI).
    ///
    /// [`Self::poll_mcp_bootstrap`] freezes at settle or [`crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT`].
    pub(super) async fn arm_tool_freeze_gate_inner(&mut self) {
        use crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT;
        if self.agent.is_tools_frozen() {
            return;
        }
        self.agent.begin_tool_gating();
        if self.tool_gate_deadline.is_none() {
            self.tool_gate_deadline = Some(std::time::Instant::now() + MCP_FIRST_TURN_GATE_TIMEOUT);
        }
        if matches!(self.mcp_boot, McpBootState::Idle) {
            if self
                .reload
                .as_ref()
                .is_some_and(|s| !s.mcp_servers.is_empty())
            {
                self.begin_mcp_bootstrap().await;
            } else {
                self.mcp_boot = McpBootState::Settled;
                let _ = self.try_complete_armed_tool_gate();
            }
        } else if matches!(self.mcp_boot, McpBootState::Settled) {
            let _ = self.try_complete_armed_tool_gate();
        }
    }

    fn try_complete_armed_tool_gate(&mut self) -> bool {
        if self.agent.is_tools_frozen() {
            self.tool_gate_deadline = None;
            return false;
        }
        let Some(deadline) = self.tool_gate_deadline else {
            return false;
        };
        let timed_out = std::time::Instant::now() >= deadline;
        if self.mcp_blocks_agent() && !timed_out {
            return false;
        }
        if matches!(self.mcp_boot, McpBootState::Running { .. }) && timed_out {
            log::warn!(
                target: "xylitol::mcp",
                "MCP tool gate timed out; detaching bootstrap and freezing subset"
            );
            self.detach_running_bootstrap_after_gate_timeout();
            self.mcp_gate_notice = Some(
                "MCP gate timed out — tools frozen with armed subset (see /mcp). /reload to retry."
                    .into(),
            );
        } else if self.mcp_blocks_agent() {
            return false;
        }
        let tools = self.agent.tools_snapshot();
        self.agent.freeze_tools(tools);
        self.tool_gate_deadline = None;
        true
    }

    pub(super) async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        let skill_names = self.loaded_skill_names();
        let mcp_connecting_label = match &self.mcp_boot {
            McpBootState::Running { progress, .. } => progress.lock().await.connecting_label(),
            _ => None,
        };
        let connecting = matches!(self.mcp_boot, McpBootState::Running { .. });
        let mcp_bootstrap_complete =
            matches!(self.mcp_boot, McpBootState::Settled | McpBootState::Idle)
                && self.late_mcp_discover.is_none();
        let tools_table_frozen = self.agent.is_tools_frozen();
        let tool_names = self.agent.tool_names();
        let Some(state) = self.reload.as_ref() else {
            return LoadedResourcesSnapshot {
                skill_names,
                mcp_connecting_label,
                mcp_bootstrap_complete: true,
                tools_table_frozen,
                mcp_gate_notice: self.mcp_gate_notice.clone(),
                obs_diag: crate::infra::observability::otel::otlp_disabled_diag(),
                ..LoadedResourcesSnapshot::default()
            };
        };
        let connected = {
            let t0 = std::time::Instant::now();
            let c = state.mcp.connected_servers().await;
            crate::app::core::lag::note_detail(
                "loaded_snap_connected_servers",
                t0,
                &format!("servers={}", c.len()),
            );
            c
        };
        let diags = state.mcp.diagnostics().await;
        let mcp_servers = state
            .mcp_servers
            .iter()
            .map(|spec| {
                let id = spec.name.clone();
                let connected_info = connected.iter().find(|s| s.id == id);
                let failed = diags.iter().any(|d| d.server == id);
                let prefix = crate::protocol::mcp_tool_armed_prefix(&id);
                let armed_count = tool_names.iter().filter(|n| n.starts_with(&prefix)).count();
                let tools_armed = armed_count > 0;
                let phase = if connected_info.is_some() {
                    crate::app::core::driver::McpServerPhase::Connected
                } else if failed {
                    crate::app::core::driver::McpServerPhase::Failed
                } else if connecting {
                    crate::app::core::driver::McpServerPhase::Connecting
                } else {
                    crate::app::core::driver::McpServerPhase::Failed
                };
                crate::app::core::driver::McpServerSnapshot {
                    id,
                    phase,
                    tools_armed,
                    tool_count: connected_info.map(|c| c.tool_count).unwrap_or(armed_count),
                }
            })
            .collect();
        LoadedResourcesSnapshot {
            skill_names,
            mcp_connected: connected
                .into_iter()
                .map(|s| (s.id, s.tool_count))
                .collect(),
            mcp_configured: state.mcp_servers.len(),
            mcp_diag_short: diags
                .into_iter()
                .map(|d| format!("{}: {}", d.server, d.message))
                .collect(),
            mcp_connecting_label,
            mcp_servers,
            mcp_bootstrap_complete,
            tools_table_frozen,
            mcp_gate_notice: self.mcp_gate_notice.clone(),
            obs_diag: crate::infra::observability::otel::otlp_disabled_diag(),
        }
    }

    pub(super) fn mcp_blocks_agent(&self) -> bool {
        matches!(
            self.mcp_boot,
            McpBootState::Running { .. }
                | McpBootState::Rebuilding { .. }
                | McpBootState::Settling { .. }
        )
    }

    pub(super) async fn begin_mcp_bootstrap(&mut self) {
        if !matches!(self.mcp_boot, McpBootState::Idle) {
            return;
        }
        let Some(state) = self.reload.as_ref() else {
            self.mcp_boot = McpBootState::Settled;
            return;
        };
        if state.mcp_servers.is_empty() {
            self.mcp_boot = McpBootState::Settled;
            return;
        }
        let servers = crate::app::core::mcp_spec::McpServerSpec::to_infra_list(&state.mcp_servers);
        let progress = Arc::new(tokio::sync::Mutex::new(
            crate::infra::mcp::McpConnectProgress {
                connecting: true,
                total: servers.len(),
                finished: 0,
                current: None,
            },
        ));
        let progress_task = progress.clone();
        let handle = tokio::spawn(async move {
            crate::infra::mcp::connect_and_discover_with_progress(&servers, Some(progress_task))
                .await
        });
        self.mcp_boot = McpBootState::Running {
            handle,
            progress,
            last_ui_label: None,
        };
    }

    pub(super) async fn poll_mcp_bootstrap(&mut self) -> bool {
        // Late discover after gate timeout detached Running.
        if let Some(handle) = self.late_mcp_discover.as_ref()
            && handle.is_finished()
        {
            let handle = self.late_mcp_discover.take().expect("late handle");
            let mut refreshed = false;
            match handle.await {
                Ok(Some((manager, tools))) => {
                    let tool_n = tools.len();
                    let old = self.reload.as_mut().and_then(|s| s.mcp.take_manager());
                    if let Some(old) = old {
                        tokio::spawn(async move {
                            old.shutdown().await;
                        });
                    }
                    if let Some(state) = self.reload.as_mut() {
                        state.mcp.set_manager(manager);
                    }
                    if self.agent.is_tools_frozen() {
                        log::info!(
                            target: "xylitol::mcp",
                            "late MCP discover after gate: registry only (FROZEN) mcp_tools={tool_n}"
                        );
                        refreshed = true;
                    } else {
                        let builtins = self.builtins_for_reload();
                        let set = tokio::task::spawn_blocking(move || {
                            crate::agent::tools::ToolSet::rebuild_agent_tools(builtins, tools)
                        })
                        .await;
                        if let Ok(set) = set {
                            let opts = self.agent.set_tools_defer_prompt(set);
                            let prompt = tokio::task::spawn_blocking(move || {
                                crate::agent::prompt::build_system_prompt(&opts)
                            })
                            .await;
                            if let Ok(prompt) = prompt {
                                self.agent.install_system_prompt_text(prompt);
                            }
                            refreshed = true;
                        }
                    }
                }
                Ok(None) | Err(_) => {
                    log::warn!(target: "xylitol::mcp", "late MCP discover after gate failed or empty");
                    refreshed = true;
                }
            }
            if refreshed {
                return true;
            }
        }

        // Finish deferred system-prompt install before polling connect progress.
        if matches!(self.mcp_boot, McpBootState::Settling { .. }) {
            let finished = match &self.mcp_boot {
                McpBootState::Settling { handle } => handle.is_finished(),
                _ => false,
            };
            if !finished {
                return false;
            }
            let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Idle);
            let McpBootState::Settling { handle } = prev else {
                return false;
            };
            match handle.await {
                Ok(prompt) => {
                    let t0 = std::time::Instant::now();
                    self.agent.install_system_prompt_text(prompt);
                    crate::app::core::lag::note("mcp_settle_install_prompt", t0);
                }
                Err(e) => {
                    log::warn!(target: "xylitol::mcp", "MCP settle prompt join failed: {e}");
                }
            }
            self.mcp_boot = McpBootState::Settled;
            // MUST refresh UI: bootstrap_complete flips Settling→Settled. Skipping
            // left a stale snap with mcp_bootstrap_complete=false so idle cue
            // sticky-restored "mcp pending" while the welcome card already showed
            // connected (manager/tools applied one phase earlier).
            let _ = self.try_complete_armed_tool_gate();
            return true;
        }

        // Apply rebuilt ToolSet + kick deferred prompt (rebuild ran off-tick).
        if matches!(self.mcp_boot, McpBootState::Rebuilding { .. }) {
            let finished = match &self.mcp_boot {
                McpBootState::Rebuilding { handle, .. } => handle.is_finished(),
                _ => false,
            };
            if !finished {
                return false;
            }
            let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Idle);
            let McpBootState::Rebuilding {
                handle,
                manager,
                tool_n,
                settle_started,
            } = prev
            else {
                return false;
            };
            let set = match handle.await {
                Ok(set) => set,
                Err(e) => {
                    log::warn!(target: "xylitol::mcp", "MCP settle rebuild join failed: {e}");
                    self.mcp_boot = McpBootState::Settled;
                    return true;
                }
            };
            crate::app::core::lag::note_detail(
                "mcp_settle_rebuild_tools",
                settle_started,
                &format!("mcp_tools={tool_n}"),
            );
            if let Some(state) = self.reload.as_mut() {
                state.mcp.set_manager(manager);
            }
            // c1900: after FROZEN, settle updates registry only — no provider tools expand.
            if self.agent.is_tools_frozen() {
                log::info!(
                    target: "xylitol::mcp",
                    "MCP settle ignored for provider tools (FROZEN); registry updated mcp_tools={tool_n}"
                );
                self.mcp_boot = McpBootState::Settled;
                return true;
            }
            let t_set = std::time::Instant::now();
            let opts = self.agent.set_tools_defer_prompt(set);
            crate::app::core::lag::note_detail(
                "mcp_settle_set_tools",
                t_set,
                &format!("mcp_tools={tool_n}"),
            );
            let handle = tokio::task::spawn_blocking(move || {
                crate::agent::prompt::build_system_prompt(&opts)
            });
            self.mcp_boot = McpBootState::Settling { handle };
            crate::app::core::lag::note_detail(
                "mcp_settle_total",
                settle_started,
                &format!("mcp_tools={tool_n} deferred_rebuild=1 deferred_prompt=1"),
            );
            return true;
        }

        let finished = match &self.mcp_boot {
            McpBootState::Running { handle, .. } => handle.is_finished(),
            // Settled/Idle with an armed first-turn deadline MUST still freeze;
            // otherwise attach Assembling waits forever after a 0-connected settle.
            _ => return self.try_complete_armed_tool_gate(),
        };
        if !finished {
            // Progress-only: refresh loaded-resources when the connecting label
            // changes (0/n → 1/n …). Unchanged ticks MUST NOT invalidate TUI upper.
            let label = match &self.mcp_boot {
                McpBootState::Running { progress, .. } => progress.lock().await.connecting_label(),
                _ => return false,
            };
            let progress_refresh =
                if let McpBootState::Running { last_ui_label, .. } = &mut self.mcp_boot {
                    mcp_progress_needs_ui_refresh(last_ui_label, label)
                } else {
                    false
                };
            let gated = self.try_complete_armed_tool_gate();
            return progress_refresh || gated;
        }
        let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Idle);
        let McpBootState::Running { handle, .. } = prev else {
            return false;
        };
        let boot_refreshed = match handle.await {
            Ok(Some((manager, tools))) => {
                let tool_n = tools.len();
                let settle_started = std::time::Instant::now();
                let old = self.reload.as_mut().and_then(|s| s.mcp.take_manager());
                // Do not await shutdown on the TUI tick path (spinner hitch).
                if let Some(old) = old {
                    tokio::spawn(async move {
                        old.shutdown().await;
                    });
                }
                let builtins = self.builtins_for_reload();
                let handle = tokio::task::spawn_blocking(move || {
                    crate::agent::tools::ToolSet::rebuild_agent_tools(builtins, tools)
                });
                self.mcp_boot = McpBootState::Rebuilding {
                    handle,
                    manager,
                    tool_n,
                    settle_started,
                };
                // UI refresh waits until tools are applied (Rebuilding → Settling).
                false
            }
            Ok(None) => {
                self.mcp_boot = McpBootState::Settled;
                true
            }
            Err(e) => {
                log::warn!(target: "xylitol::mcp", "MCP bootstrap join failed: {e}");
                self.mcp_boot = McpBootState::Settled;
                true
            }
        };
        self.try_complete_armed_tool_gate() || boot_refreshed
    }
}
