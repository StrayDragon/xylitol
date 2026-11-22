use async_trait::async_trait;

use super::XyDriver;
use super::XyDriverError;
use super::XyInProcessDriver;
use super::XySessionOps;

use crate::dispatch::{
    DispatchOutcome, DispatchRejection, transport_variant_error, validate_nonempty_thinking_level,
};
use xylitol_protocol::Command;

pub struct CommandExecutorView<'a>(pub &'a mut XyInProcessDriver);

#[async_trait]
impl crate::dispatch::SessionCommandExecutor for CommandExecutorView<'_> {
    async fn execute_session_command(
        &mut self,
        cmd: Command,
    ) -> Result<DispatchOutcome, DispatchRejection> {
        let driver: &mut XyInProcessDriver = &mut *self.0;
        let outcome: Result<DispatchOutcome, XyDriverError> = match cmd {
            Command::Abort { .. } => Ok(exec_abort(driver)),
            Command::GetState { .. } => Ok(DispatchOutcome::State(XyDriver::get_state(driver))),
            Command::SetModel { model_id, .. } => exec_set_model(driver, &model_id).await,
            Command::CycleModel { .. } => exec_cycle_model(driver).await,
            Command::GetAvailableModels { .. } => {
                Ok(DispatchOutcome::Models(XyDriver::available_models(driver)))
            }
            Command::SetThinkingLevel { level, .. } => exec_set_thinking_level(driver, level).await,
            Command::Bash {
                command,
                exclude_from_context,
                ..
            } => exec_bash(driver, &command, exclude_from_context).await,
            Command::Compact { instructions, .. } => exec_compact(driver, instructions).await,
            Command::GetSessionStats { .. } => exec_session_stats(&*driver).await,
            Command::ExportHtml { output_path, .. } => exec_export_html(driver, output_path).await,
            Command::ExportJsonl { output_path, .. } => {
                exec_export_jsonl(driver, output_path).await
            }
            Command::ImportJsonl { input_path, .. } => exec_import_jsonl(driver, &input_path).await,
            Command::SwitchSession { session_path, .. } => {
                exec_switch_session(&mut *driver, &session_path).await
            }
            Command::Fork {
                entry_id, position, ..
            } => exec_fork(&mut *driver, &entry_id, position.as_deref()).await,
            Command::GetMessages {
                limit,
                before_entry_id,
            } => {
                exec_get_messages(
                    &*driver,
                    XyDriver::session_id(driver).unwrap_or_default(),
                    limit,
                    before_entry_id.as_deref(),
                )
                .await
            }
            Command::EstimateContext { .. } => exec_estimate_context(&*driver).await,
            Command::SessionTree { kind, .. } => exec_session_tree(driver, kind).await,
            Command::TravelSessionTree { kind, entry_id, .. } => {
                exec_travel_session_tree(driver, kind, &entry_id).await
            }
            Command::AppendEntryLabel {
                target_id, label, ..
            } => exec_append_entry_label(&mut *driver, &target_id, label.as_deref()).await,
            cmd @ (Command::ListSessions { .. }
            | Command::LoadSessionEntries { .. }
            | Command::NewSession { .. }
            | Command::GetSessionName { .. }
            | Command::SetSessionName { .. }
            | Command::SetSessionNameFor { .. }
            | Command::GenerateSessionName { .. }
            | Command::DeleteSession { .. }) => exec_session_meta(&mut *driver, cmd).await,
            Command::Reload { .. } => exec_reload(driver).await,
            Command::LoadedResources { .. } => exec_loaded_resources(driver).await,
            Command::GetQueueStats { .. } => Ok(queue_items_outcome(driver)),
            Command::GetCommands { .. } => {
                Ok(DispatchOutcome::Commands(XyDriver::get_commands(driver)))
            }
            Command::PluginCommand { plugin, args } => {
                exec_plugin_command(driver, &plugin, &args).await
            }
            cmd @ (Command::GetCapabilities { .. }
            | Command::SetCapability { .. }
            | Command::ClearCapabilities { .. }) => exec_capability(driver, cmd).await,
            Command::Steer { message, .. } => exec_steer(driver, &message).await,
            Command::FollowUp { message, .. } => exec_follow_up(driver, &message).await,
            Command::ClearQueue {
                clear_steer,
                clear_follow_up,
                ..
            } => exec_clear_queue(driver, clear_steer, clear_follow_up).await,
            Command::RevokeQueueItem { id } => exec_revoke_queue_item(driver, &id).await,
            Command::SessionDetach { .. } => Ok(DispatchOutcome::Empty),
            Command::Prompt { .. }
            | Command::Quit { .. }
            | Command::Subscribe { .. }
            | Command::ApproveTool { .. }
            | Command::AnswerQuestion { .. }
            | Command::McpCatalog { .. }
            | Command::McpResourceRead { .. }
            | Command::McpPromptGet { .. }
            | Command::McpOAuthStart { .. }
            | Command::McpOAuthCancel { .. }
            | Command::McpOAuthStatus { .. } => Err(transport_variant_error(&cmd).into()),
        };
        outcome.map_err(DispatchRejection::from)
    }
}

fn exec_abort(driver: &mut XyInProcessDriver) -> DispatchOutcome {
    XyDriver::abort(driver);
    DispatchOutcome::Aborted { cancelled: true }
}

async fn exec_plugin_command(
    driver: &mut XyInProcessDriver,
    plugin: &str,
    args: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    let name = format!("plugin:{plugin}");
    let Some(command) = driver.plugin_command(&name) else {
        return Err(XyDriverError::invalid_input(format!(
            "unknown plugin command: {name}"
        )));
    };
    let text = command
        .invoke(args)
        .await
        .map_err(XyDriverError::invalid_input)?;
    Ok(DispatchOutcome::PluginCommandOutput {
        plugin: plugin.to_string(),
        text,
    })
}

async fn exec_capability(
    driver: &mut XyInProcessDriver,
    cmd: Command,
) -> Result<DispatchOutcome, XyDriverError> {
    match cmd {
        Command::GetCapabilities { .. } => Ok(DispatchOutcome::Capabilities(
            XyDriver::capability_snapshot(driver),
        )),
        Command::SetCapability { id, enabled } => XyDriver::set_capability(driver, &id, enabled)
            .await
            .map(DispatchOutcome::Capabilities),
        Command::ClearCapabilities { .. } => XyDriver::clear_capabilities(driver)
            .await
            .map(DispatchOutcome::Capabilities),
        other => Err(transport_variant_error(&other).into()),
    }
}

async fn exec_session_meta(
    driver: &mut dyn XySessionOps,
    cmd: Command,
) -> Result<DispatchOutcome, XyDriverError> {
    match cmd {
        Command::ListSessions { .. } => exec_list_sessions(driver).await,
        Command::LoadSessionEntries { session_id, .. } => {
            exec_load_session_entries(driver, &session_id).await
        }
        Command::NewSession { .. } => exec_new_session(driver).await,
        Command::GetSessionName { .. } => exec_get_session_name(driver).await,
        Command::SetSessionName { name, .. } => exec_set_session_name(driver, &name).await,
        Command::SetSessionNameFor {
            session_id, name, ..
        } => exec_set_session_name_for(driver, &session_id, &name).await,
        Command::GenerateSessionName { force } => exec_generate_session_name(driver, force).await,
        Command::DeleteSession { session_id, .. } => exec_delete_session(driver, &session_id).await,
        other => Err(transport_variant_error(&other).into()),
    }
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

async fn exec_session_stats(driver: &dyn XySessionOps) -> Result<DispatchOutcome, XyDriverError> {
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
    driver: &mut dyn XySessionOps,
    session_path: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    let new_id = session_id_from_path(session_path);
    let switched = driver.switch_session(&new_id).await?;
    Ok(DispatchOutcome::SwitchedSession(switched))
}

async fn exec_fork(
    driver: &mut dyn XySessionOps,
    entry_id: &str,
    position: Option<&str>,
) -> Result<DispatchOutcome, XyDriverError> {
    let new_id = driver
        .fork_session(entry_id, fork_position(position))
        .await?;
    Ok(DispatchOutcome::NewSession(new_id))
}

async fn exec_get_messages(
    driver: &dyn XySessionOps,
    session_id: String,
    limit: Option<usize>,
    before_entry_id: Option<&str>,
) -> Result<DispatchOutcome, XyDriverError> {
    let (entries, has_more) = driver.get_messages_page(limit, before_entry_id).await?;
    Ok(DispatchOutcome::Messages {
        session_id,
        entries,
        has_more,
    })
}

async fn exec_estimate_context(
    driver: &dyn XySessionOps,
) -> Result<DispatchOutcome, XyDriverError> {
    let estimate = XySessionOps::estimate_context_tokens(driver).await?;
    Ok(DispatchOutcome::EstimateContext(estimate))
}

async fn exec_session_tree(
    driver: &mut dyn XySessionOps,
    kind: xylitol_protocol::SessionTreeKind,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionTree(
        driver.session_tree(sdk_session_tree_kind(kind)).await?,
    ))
}

async fn exec_travel_session_tree(
    driver: &mut dyn XySessionOps,
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
    driver: &mut dyn XySessionOps,
    target_id: &str,
    label: Option<&str>,
) -> Result<DispatchOutcome, XyDriverError> {
    driver.append_entry_label(target_id, label).await?;
    Ok(DispatchOutcome::Empty)
}

async fn exec_list_sessions(driver: &dyn XySessionOps) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::Sessions(driver.list_sessions().await?))
}

async fn exec_load_session_entries(
    driver: &dyn XySessionOps,
    session_id: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionEntries(
        driver.load_session_entries(session_id).await?,
    ))
}

async fn exec_new_session(driver: &mut dyn XySessionOps) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::NewSession(driver.new_session().await?))
}

async fn exec_get_session_name(
    driver: &dyn XySessionOps,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(
        driver.get_session_name().await?,
    ))
}

async fn exec_set_session_name(
    driver: &mut dyn XySessionOps,
    name: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(Some(
        driver.set_session_name(name).await?,
    )))
}

async fn exec_set_session_name_for(
    driver: &mut dyn XySessionOps,
    session_id: &str,
    name: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(Some(
        driver.set_session_name_for(session_id, name).await?,
    )))
}

async fn exec_generate_session_name(
    driver: &mut dyn XySessionOps,
    force: bool,
) -> Result<DispatchOutcome, XyDriverError> {
    Ok(DispatchOutcome::SessionName(Some(
        driver.generate_session_name(force).await?,
    )))
}

async fn exec_delete_session(
    driver: &mut dyn XySessionOps,
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
    let id = XyDriver::steer(driver, message).await?;
    Ok(DispatchOutcome::QueueItemEnqueued {
        id,
        items: XyDriver::queue_items(driver),
    })
}

async fn exec_follow_up(
    driver: &mut XyInProcessDriver,
    message: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    let id = XyDriver::follow_up(driver, message).await?;
    Ok(DispatchOutcome::QueueItemEnqueued {
        id,
        items: XyDriver::queue_items(driver),
    })
}

async fn exec_clear_queue(
    driver: &mut XyInProcessDriver,
    clear_steer: bool,
    clear_follow_up: bool,
) -> Result<DispatchOutcome, XyDriverError> {
    driver.clear_queue(clear_steer, clear_follow_up).await?;
    Ok(queue_items_outcome(driver))
}

async fn exec_revoke_queue_item(
    driver: &XyInProcessDriver,
    id: &str,
) -> Result<DispatchOutcome, XyDriverError> {
    let outcome = XyDriver::revoke_queue_item(driver, id);
    if !outcome.revoked {
        return Err(XyDriverError::not_found(id.to_string()));
    }
    Ok(DispatchOutcome::RevokedQueueItem(outcome))
}

fn queue_items_outcome(driver: &XyInProcessDriver) -> DispatchOutcome {
    DispatchOutcome::QueueItems(XyDriver::queue_items(driver))
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
