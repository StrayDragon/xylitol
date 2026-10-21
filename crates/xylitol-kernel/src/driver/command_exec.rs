//! SessionCommandExecutor for the in-process driver (envelope side).

use async_trait::async_trait;

use super::XyComposedDriver;
use super::XyDriver;
use super::XyDriverError;

// ── Command executor (c2710): Command is the SSOT of session operations ────
#[async_trait]
impl crate::dispatch::SessionCommandExecutor for XyComposedDriver {
    async fn execute_session_command(
        &mut self,
        cmd: xylitol_protocol::Command,
    ) -> Result<crate::dispatch::DispatchOutcome, XyDriverError> {
        use crate::dispatch::DispatchOutcome;
        use xylitol_protocol::Command;
        match cmd {
            Command::Abort { .. } => {
                crate::driver::XyDriver::abort(self);
                Ok(DispatchOutcome::Aborted { cancelled: true })
            }
            Command::GetState { .. } => Ok(DispatchOutcome::State(
                crate::driver::XyDriver::get_state(self),
            )),
            Command::SetModel { model_id, .. } => {
                let m = self.select_model(&model_id).await?;
                Ok(DispatchOutcome::Model(m))
            }
            Command::CycleModel { .. } => {
                let m = self.cycle_model().await?;
                Ok(DispatchOutcome::Model(m))
            }
            Command::GetAvailableModels { .. } => Ok(DispatchOutcome::Models(
                crate::driver::XyDriver::available_models(self),
            )),
            Command::SetThinkingLevel { level, .. } => {
                crate::dispatch::validate_nonempty_thinking_level(&level)?;
                self.set_thinking_level(level.clone()).await?;
                Ok(DispatchOutcome::ThinkingLevel(level))
            }
            Command::Bash {
                command,
                exclude_from_context,
                ..
            } => {
                let run = XyDriver::bash_run(self, &command, exclude_from_context).await?;
                let r = run.await?;
                Ok(DispatchOutcome::Bash(r))
            }
            Command::Compact { instructions, .. } => {
                let did = self.compact(instructions).await?;
                Ok(DispatchOutcome::Compacted(did))
            }
            Command::GetSessionStats { .. } => {
                let stats = self.get_session_stats().await?;
                Ok(DispatchOutcome::SessionStats(serde_json::json!({
                    "session_id": stats.session_id,
                    "user_messages": stats.user_messages,
                    "assistant_messages": stats.assistant_messages,
                    "total_messages": stats.total_messages,
                    "thinking_level": stats.thinking_level,
                    "model": stats.model.map(|(p, m)| {
                        serde_json::json!({ "provider": p, "model_id": m })
                    }),
                })))
            }
            Command::ExportHtml { output_path, .. } => {
                let path = output_path
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from("export.html"));
                let written = self.export_html(&path).await?;
                Ok(DispatchOutcome::ExportedPath(written))
            }
            Command::ExportJsonl { output_path, .. } => {
                let path = output_path
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from("export.jsonl"));
                let written = self.export_jsonl(&path).await?;
                Ok(DispatchOutcome::ExportedPath(written))
            }
            Command::ImportJsonl { input_path, .. } => {
                let path = std::path::PathBuf::from(&input_path);
                let new_id = self.import_jsonl(&path).await?;
                Ok(DispatchOutcome::NewSession(new_id))
            }
            Command::SwitchSession { session_path, .. } => {
                // Derive session id from path (file stem).
                let new_id = std::path::Path::new(&session_path)
                    .file_stem()
                    .and_then(|st| st.to_str())
                    .unwrap_or(&session_path)
                    .to_string();
                let switched = self.switch_session(&new_id).await?;
                Ok(DispatchOutcome::SwitchedSession(switched))
            }
            Command::Fork {
                entry_id, position, ..
            } => {
                let pos = match position.as_deref() {
                    Some("before") => xylitol_plugin_sdk::session::ForkPosition::Before,
                    _ => xylitol_plugin_sdk::session::ForkPosition::At,
                };
                let new_id = self.fork_session(&entry_id, pos).await?;
                Ok(DispatchOutcome::NewSession(new_id))
            }
            Command::GetMessages { .. } => {
                let entries = self.get_messages().await?;
                let session_id = crate::driver::XyDriver::session_id(self).unwrap_or_default();
                Ok(DispatchOutcome::Messages {
                    session_id,
                    entries,
                })
            }
            Command::EstimateContext { .. } => {
                let estimate = XyComposedDriver::estimate_context_tokens(self).await?;
                Ok(DispatchOutcome::EstimateContext(estimate))
            }
            Command::SessionTree { kind, .. } => Ok(DispatchOutcome::SessionTree(
                self.session_tree(xylitol_serve::v3_mapping::sdk_session_tree_kind(kind))
                    .await?,
            )),
            Command::TravelSessionTree { kind, entry_id, .. } => {
                Ok(DispatchOutcome::SessionTreeTravel(
                    self.travel_session_tree(
                        xylitol_serve::v3_mapping::sdk_session_tree_kind(kind),
                        &entry_id,
                    )
                    .await?,
                ))
            }
            Command::AppendEntryLabel {
                target_id, label, ..
            } => {
                self.append_entry_label(&target_id, label.as_deref())
                    .await?;
                Ok(DispatchOutcome::Empty)
            }
            Command::ListSessions { .. } => {
                Ok(DispatchOutcome::Sessions(self.list_sessions().await?))
            }
            Command::LoadSessionEntries { session_id, .. } => Ok(DispatchOutcome::SessionEntries(
                self.load_session_entries(&session_id).await?,
            )),
            Command::NewSession { .. } => {
                Ok(DispatchOutcome::NewSession(self.new_session().await?))
            }
            Command::GetSessionName { .. } => {
                Ok(DispatchOutcome::SessionName(self.get_session_name().await?))
            }
            Command::SetSessionName { name, .. } => Ok(DispatchOutcome::SessionName(Some(
                self.set_session_name(&name).await?,
            ))),
            Command::SetSessionNameFor {
                session_id, name, ..
            } => Ok(DispatchOutcome::SessionName(Some(
                self.set_session_name_for(&session_id, &name).await?,
            ))),
            Command::DeleteSession { session_id, .. } => {
                self.delete_session(&session_id).await?;
                Ok(DispatchOutcome::Empty)
            }
            Command::Reload { .. } => Ok(DispatchOutcome::Reload(
                self.reload_runtime(&tokio_util::sync::CancellationToken::new())
                    .await?,
            )),
            Command::LoadedResources { .. } => Ok(DispatchOutcome::LoadedResources(
                crate::driver::XyDriver::loaded_resources_snapshot(self).await,
            )),
            Command::GetQueueStats { .. } => {
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }
            Command::GetCommands { .. } => Ok(DispatchOutcome::Commands(
                crate::driver::XyDriver::get_commands(self),
            )),
            Command::Steer { message, .. } => {
                self.steer(&message).await?;
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }
            Command::FollowUp { message, .. } => {
                self.follow_up(&message).await?;
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }
            Command::ClearQueue {
                clear_steer,
                clear_follow_up,
                ..
            } => {
                self.clear_queue(clear_steer, clear_follow_up).await?;
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }

            // These variants are the caller's responsibility (see dispatch module docs).
            Command::Prompt { .. }
            | Command::Quit { .. }
            | Command::Subscribe { .. }
            | Command::ApproveTool { .. }
            | Command::AnswerQuestion { .. } => Err(crate::dispatch::transport_variant_error(&cmd)),
        }
    }
}
