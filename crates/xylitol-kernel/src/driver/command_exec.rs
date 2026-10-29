use async_trait::async_trait;

use super::XyDriver;
use super::XyDriverError;
use super::XyInProcessDriver;

use crate::dispatch::{DispatchOutcome, transport_variant_error, validate_nonempty_thinking_level};
use xylitol_protocol::Command;

#[async_trait]
impl crate::dispatch::SessionCommandExecutor for XyInProcessDriver {
    async fn execute_session_command(
        &mut self,
        cmd: Command,
    ) -> Result<DispatchOutcome, XyDriverError> {
        match cmd {
            Command::Abort { .. } => Ok(exec_abort(self)),
            Command::GetState { .. } => Ok(DispatchOutcome::State(XyDriver::get_state(self))),
            Command::SetModel { model_id, .. } => exec_set_model(self, &model_id).await,
            Command::CycleModel { .. } => exec_cycle_model(self).await,
            Command::GetAvailableModels { .. } => {
                Ok(DispatchOutcome::Models(XyDriver::available_models(self)))
            }
            Command::SetThinkingLevel { level, .. } => exec_set_thinking_level(self, level).await,
            Command::SetPlanMode { mode, plan_id } => {
                exec_set_plan_mode(self, &mode, plan_id).await
            }
            Command::Bash {
                command,
                exclude_from_context,
                ..
            } => exec_bash(self, &command, exclude_from_context).await,
            Command::Compact { instructions, .. } => exec_compact(self, instructions).await,
            Command::GetSessionStats { .. } => exec_session_stats(self).await,
            Command::ExportHtml { output_path, .. } => exec_export_html(self, output_path).await,
            Command::ExportJsonl { output_path, .. } => exec_export_jsonl(self, output_path).await,
            Command::ImportJsonl { input_path, .. } => exec_import_jsonl(self, &input_path).await,
            Command::SwitchSession { session_path, .. } => {
                exec_switch_session(self, &session_path).await
            }
            Command::Fork {
                entry_id, position, ..
            } => exec_fork(self, &entry_id, position.as_deref()).await,
            Command::GetMessages {
                limit,
                before_entry_id,
            } => exec_get_messages(self, limit, before_entry_id.as_deref()).await,
            Command::EstimateContext { .. } => exec_estimate_context(self).await,
            Command::SessionTree { kind, .. } => exec_session_tree(self, kind).await,
            Command::TravelSessionTree { kind, entry_id, .. } => {
                exec_travel_session_tree(self, kind, &entry_id).await
            }
            Command::AppendEntryLabel {
                target_id, label, ..
            } => exec_append_entry_label(self, &target_id, label.as_deref()).await,
            Command::ListSessions { .. } => exec_list_sessions(self).await,
            Command::LoadSessionEntries { session_id, .. } => {
                exec_load_session_entries(self, &session_id).await
            }
            Command::NewSession { .. } => exec_new_session(self).await,
            Command::GetSessionName { .. } => exec_get_session_name(self).await,
            Command::SetSessionName { name, .. } => exec_set_session_name(self, &name).await,
            Command::SetSessionNameFor {
                session_id, name, ..
            } => exec_set_session_name_for(self, &session_id, &name).await,
            Command::DeleteSession { session_id, .. } => {
                exec_delete_session(self, &session_id).await
            }
            Command::Reload { .. } => exec_reload(self).await,
            Command::LoadedResources { .. } => exec_loaded_resources(self).await,
            Command::GetQueueStats { .. } => Ok(queue_stats_outcome(self)),
            Command::GetCommands { .. } => {
                Ok(DispatchOutcome::Commands(XyDriver::get_commands(self)))
            }
            Command::GetCapabilities { .. } => Ok(DispatchOutcome::Capabilities(
                XyDriver::capability_snapshot(self),
            )),
            Command::SetCapability { id, enabled } => {
                XyDriver::set_capability(self, &id, enabled).map(DispatchOutcome::Capabilities)
            }
            Command::ClearCapabilities { .. } => {
                XyDriver::clear_capabilities(self).map(DispatchOutcome::Capabilities)
            }
            Command::Steer { message, .. } => exec_steer(self, &message).await,
            Command::FollowUp { message, .. } => exec_follow_up(self, &message).await,
            Command::ClearQueue {
                clear_steer,
                clear_follow_up,
                ..
            } => exec_clear_queue(self, clear_steer, clear_follow_up).await,
            Command::Prompt { .. }
            | Command::Quit { .. }
            | Command::Subscribe { .. }
            | Command::ApproveTool { .. }
            | Command::AnswerQuestion { .. } => Err(transport_variant_error(&cmd)),
        }
    }
}

fn exec_abort(driver: &mut XyInProcessDriver) -> DispatchOutcome {
    XyDriver::abort(driver);
    DispatchOutcome::Aborted { cancelled: true }
}

async fn exec_set_model(
    driver: &mut XyInProcessDriver,
    model_id: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    let model = driver.select_model(model_id).await?;
    Ok(DispatchOutcome::Model(model))
}

async fn exec_cycle_model(
    driver: &mut XyInProcessDriver,
) -> Result<DispatchOutcome, XyDriverError> {
    let model = driver.cycle_model().await?;
    Ok(DispatchOutcome::Model(model))
}

async fn exec_set_thinking_level(
    driver: &mut XyInProcessDriver,
    level: String,
) -> Result<DispatchOutcome, XyDriverError> {
    validate_nonempty_thinking_level(&level)?;
    driver.set_thinking_level(level.clone()).await?;
    Ok(DispatchOutcome::ThinkingLevel(level))
}

async fn exec_set_plan_mode(
    driver: &mut XyInProcessDriver,
    mode: &str,
    plan_id: Option<String>,
) -> Result<DispatchOutcome, XyDriverError> {
    let session_id = XyDriver::session_id(driver)
        .ok_or_else(|| XyDriverError::invalid_input("no active session"))?;
    let branch = load_plan_branch(driver, &session_id).await?;
    let next = next_plan_mode(mode, plan_id, &branch)?;
    driver
        .store
        .append_session_entry(&session_id, &next.to_custom_entry_shell())
        .await
        .map_err(XyDriverError::from)?;
    Ok(DispatchOutcome::PlanMode {
        mode: plan_mode_label(next.mode),
        plan_id: next.plan_id,
    })
}

async fn load_plan_branch(
    driver: &XyInProcessDriver,
    session_id: &str,
) -> Result<Vec<xylitol_plugin_sdk::session::SessionEntry>, XyDriverError> {
    match driver.store.load_leaf_branch(session_id).await {
        Ok(branch) => Ok(branch),
        Err(xylitol_plugin_sdk::error::XySessionStoreError::NotFound { .. }) => Ok(Vec::new()),
        Err(err) => Err(XyDriverError::from(err)),
    }
}

async fn exec_bash(
    driver: &mut XyInProcessDriver,
    command: &str,
    exclude_from_context: bool,
) -> Result<DispatchOutcome, XyDriverError> {
    let run = XyDriver::bash_run(driver, command, exclude_from_context).await?;
    let result = run.await?;
    Ok(DispatchOutcome::Bash(result))
}

async fn exec_compact(
    driver: &mut XyInProcessDriver,
    instructions: Option<String>,
) -> Result<DispatchOutcome, XyDriverError> {
    let compacted = driver.compact(instructions).await?;
    Ok(DispatchOutcome::Compacted(compacted))
}

async fn exec_session_stats(driver: &XyInProcessDriver) -> Result<DispatchOutcome, XyDriverError> {
    let stats = driver.get_session_stats().await?;
    Ok(DispatchOutcome::SessionStats(session_stats_json(stats)))
}

async fn exec_export_html(
    driver: &mut XyInProcessDriver,
    output_path: Option<String>,
) -> Result<DispatchOutcome, XyDriverError> {
    let path = export_path(output_path, "export.html");
    let written = driver.export_html(&path).await?;
    Ok(DispatchOutcome::ExportedPath(written))
}

async fn exec_export_jsonl(
    driver: &mut XyInProcessDriver,
    output_path: Option<String>,
) -> Result<DispatchOutcome, XyDriverError> {
    let path = export_path(output_path, "export.jsonl");
    let written = driver.export_jsonl(&path).await?;
    Ok(DispatchOutcome::ExportedPath(written))
}

async fn exec_import_jsonl(
    driver: &mut XyInProcessDriver,
    input_path: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    let path = std::path::PathBuf::from(input_path);
    let new_id = driver.import_jsonl(&path).await?;
    Ok(DispatchOutcome::NewSession(new_id))
}

async fn exec_switch_session(
    driver: &mut XyInProcessDriver,
    session_path: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    let new_id = session_id_from_path(session_path);
    let switched = driver.switch_session(&new_id).await?;
    Ok(DispatchOutcome::SwitchedSession(switched))
}

async fn exec_fork(
    driver: &mut XyInProcessDriver,
    entry_id: &str,
    position: Option<&str>,
) -> Result<DispatchOutcome, XyDriverError> {
    let new_id = driver
        .fork_session(entry_id, fork_position(position))
        .await?;
    Ok(DispatchOutcome::NewSession(new_id))
}

async fn exec_get_messages(
    driver: &XyInProcessDriver,
    limit: Option<usize>,
    before_entry_id: Option<&str>,
) -> Result<DispatchOutcome, XyDriverError> {
    let (entries, has_more) = driver.get_messages_page(limit, before_entry_id).await?;
    let session_id = XyDriver::session_id(driver).unwrap_or_default();
    Ok(DispatchOutcome::Messages {
        session_id,
        entries,
        has_more,
    })
}

async fn exec_estimate_context(
    driver: &XyInProcessDriver,
) -> Result<DispatchOutcome, XyDriverError> {
    let estimate = XyInProcessDriver::estimate_context_tokens(driver).await?;
    Ok(DispatchOutcome::EstimateContext(estimate))
}

async fn exec_session_tree(
    driver: &XyInProcessDriver,
    kind: xylitol_protocol::SessionTreeKind,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionTree(
        driver.session_tree(sdk_session_tree_kind(kind)).await?,
    ))
}

async fn exec_travel_session_tree(
    driver: &XyInProcessDriver,
    kind: xylitol_protocol::SessionTreeKind,
    entry_id: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionTreeTravel(
        driver
            .travel_session_tree(sdk_session_tree_kind(kind), entry_id)
            .await?,
    ))
}

async fn exec_append_entry_label(
    driver: &mut XyInProcessDriver,
    target_id: &str,
    label: Option<&str>,
) -> Result<DispatchOutcome, XyDriverError> {
    driver.append_entry_label(target_id, label).await?;
    Ok(DispatchOutcome::Empty)
}

async fn exec_list_sessions(driver: &XyInProcessDriver) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::Sessions(driver.list_sessions().await?))
}

async fn exec_load_session_entries(
    driver: &XyInProcessDriver,
    session_id: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionEntries(
        driver.load_session_entries(session_id).await?,
    ))
}

async fn exec_new_session(
    driver: &mut XyInProcessDriver,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::NewSession(driver.new_session().await?))
}

async fn exec_get_session_name(
    driver: &XyInProcessDriver,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(
        driver.get_session_name().await?,
    ))
}

async fn exec_set_session_name(
    driver: &mut XyInProcessDriver,
    name: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(Some(
        driver.set_session_name(name).await?,
    )))
}

async fn exec_set_session_name_for(
    driver: &mut XyInProcessDriver,
    session_id: &str,
    name: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(Some(
        driver.set_session_name_for(session_id, name).await?,
    )))
}

async fn exec_delete_session(
    driver: &mut XyInProcessDriver,
    session_id: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    driver.delete_session(session_id).await?;
    Ok(DispatchOutcome::Empty)
}

async fn exec_reload(driver: &mut XyInProcessDriver) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::Reload(
        driver
            .reload_runtime(&tokio_util::sync::CancellationToken::new())
            .await?,
    ))
}

async fn exec_loaded_resources(
    driver: &XyInProcessDriver,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::LoadedResources(
        XyDriver::loaded_resources_snapshot(driver).await,
    ))
}

async fn exec_steer(
    driver: &mut XyInProcessDriver,
    message: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    driver.steer(message).await?;
    Ok(queue_stats_outcome(driver))
}

async fn exec_follow_up(
    driver: &mut XyInProcessDriver,
    message: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    driver.follow_up(message).await?;
    Ok(queue_stats_outcome(driver))
}

async fn exec_clear_queue(
    driver: &mut XyInProcessDriver,
    clear_steer: bool,
    clear_follow_up: bool,
) -> Result<DispatchOutcome, XyDriverError> {
    driver.clear_queue(clear_steer, clear_follow_up).await?;
    Ok(queue_stats_outcome(driver))
}

fn queue_stats_outcome(driver: &XyInProcessDriver) -> DispatchOutcome {
    let stats = driver.queue_stats();
    DispatchOutcome::QueueStats {
        steer_count: stats.steer_count,
        follow_up_count: stats.follow_up_count,
    }
}

fn export_path(output_path: Option<String>, default: &str) -> std::path::PathBuf {
    output_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(default))
}

fn session_id_from_path(session_path: &str) -> String {
    std::path::Path::new(session_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(session_path)
        .to_string()
}

fn fork_position(position: Option<&str>) -> xylitol_plugin_sdk::session::ForkPosition {
    match position {
        Some("before") => xylitol_plugin_sdk::session::ForkPosition::Before,
        _ => xylitol_plugin_sdk::session::ForkPosition::At,
    }
}

fn session_stats_json(stats: crate::driver::SessionStats) -> serde_json::Value {
    serde_json::json!({
        "session_id": stats.session_id,
        "user_messages": stats.user_messages,
        "assistant_messages": stats.assistant_messages,
        "total_messages": stats.total_messages,
        "thinking_level": stats.thinking_level,
        "model": stats.model.map(|(provider, model_id)| {
            serde_json::json!({ "provider": provider, "model_id": model_id })
        }),
    })
}

fn next_plan_mode(
    mode: &str,
    plan_id: Option<String>,
    branch: &[xylitol_plugin_sdk::session::SessionEntry],
) -> Result<xylitol_plugin_sdk::session::PlanMode, XyDriverError> {
    use xylitol_plugin_sdk::session::{PlanMode, PlanModeKind, latest_plan_mode};

    match mode {
        "toggle" => match latest_plan_mode(branch).mode {
            PlanModeKind::Plan => Ok(PlanMode::execute()),
            PlanModeKind::Execute => Ok(PlanMode::plan(plan_id)),
        },
        "plan" => Ok(PlanMode::plan(plan_id)),
        "execute" => Ok(PlanMode::execute()),
        other => Err(XyDriverError::invalid_input(format!(
            "unknown plan mode: {other}"
        ))),
    }
}

fn plan_mode_label(kind: xylitol_plugin_sdk::session::PlanModeKind) -> String {
    match kind {
        xylitol_plugin_sdk::session::PlanModeKind::Plan => "plan".into(),
        xylitol_plugin_sdk::session::PlanModeKind::Execute => "execute".into(),
    }
}

fn sdk_session_tree_kind(
    kind: xylitol_protocol::SessionTreeKind,
) -> xylitol_plugin_sdk::session::SessionTreeKind {
    match kind {
        xylitol_protocol::SessionTreeKind::MessageHistory => {
            xylitol_plugin_sdk::session::SessionTreeKind::MessageHistory
        }
        xylitol_protocol::SessionTreeKind::FileBrowser => {
            xylitol_plugin_sdk::session::SessionTreeKind::FileBrowser
        }
    }
}
