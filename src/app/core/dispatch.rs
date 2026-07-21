//! Shared Command dispatch — the single execution path for `protocol::Command`
//! variants, consumed by tui (spec ce10).
//!
//! This module is a **pure dispatcher**: it maps each non-transport Command
//! variant to a [`XyDriver`] method call and returns a [`DispatchOutcome`]. It
//! holds no state of its own — all execution state lives behind the XyDriver.
//!
//! What does NOT live here (by design, spec ip9):
//! - `Prompt` — starts an event stream + is tied to the caller's run loop, so
//!   the caller (tui) handles it directly.
//! - `Quit` — controls the caller's loop, handled by the caller.
//! - `Subscribe` / `ApproveTool` / `AnswerQuestion` — WebSocket-transport
//!   specific; they remain in `app::server::ws`.
//!
//! The `id: Option<String>` carried by every Command is a transport-level
//! correlation handle; dispatch ignores it and
//! lets the caller extract/echo it around the dispatch call.
//!
//! NOTE: dispatch is consumed by server REST and tui slash/effects.
//! Under default features without `server`/`tui` the dispatcher still appears
//! lightly used; unit tests cover it. `DispatchOutcome` payloads are allowed
//! for dead_code at the enum (feature-gated readers).
//!
//! Outcome payload fields are read by server REST and tui slash wiring
//! (`/model`, `/compact`, `/export`, …).

use std::path::PathBuf;

use crate::app::core::driver::{CommandInfo, ModelInfo, SessionState, XyDriver};
pub use crate::app::core::driver_error::XyDriverError;
use crate::domain::session_types::SessionEntry;
use crate::domain::types::ThinkingLevel;
use crate::protocol::Command;
use crate::runtime_protocol::XyBashResult;

/// The result of executing a (non-Prompt, non-Quit, non-WS) Command.
///
/// Callers translate this into their transport-specific response shape
/// (tui's inline message, server REST JSON, ...).
///
/// Payload fields are matched under `server` / tui feature paths; default
/// `cargo build` may not see those reads — allow here instead of module-wide.
#[derive(Debug)]
#[allow(dead_code)] // fields read by server REST + tui slash (feature-gated)
pub enum DispatchOutcome {
    /// `Abort` — whether an active loop was actually cancelled.
    Aborted { cancelled: bool },
    /// `GetState` — session snapshot.
    State(SessionState),
    /// `SetModel` / `CycleModel` — the now-selected model.
    Model(ModelInfo),
    /// `GetAvailableModels` — the registered models.
    Models(Vec<ModelInfo>),
    /// `SetThinkingLevel` — the level now in effect.
    ThinkingLevel(ThinkingLevel),
    /// `Bash` — the execution result.
    Bash(XyBashResult),
    /// `Compact` — whether a compaction occurred.
    Compacted(bool),
    /// `GetSessionStats` — serialized stats payload (kept as serde_json::Value
    /// to avoid leaking the agent's SessionStats struct verbatim through the
    /// outcome; callers already serialize it).
    SessionStats(serde_json::Value),
    /// `ExportHtml` / `ExportJsonl` — the path written.
    ExportedPath(String),
    /// `ImportJsonl` / `Fork` — the new session id produced.
    NewSession(String),
    /// `SwitchSession` — the session id now active.
    SwitchedSession(String),
    /// `GetMessages` — the loaded entries.
    Messages {
        session_id: String,
        entries: Vec<SessionEntry>,
    },
    /// `GetCommands` — the available slash commands.
    Commands(Vec<CommandInfo>),
    /// `Steer` / `FollowUp` / `ClearQueue` — current queue depths.
    QueueStats {
        steer_count: usize,
        follow_up_count: usize,
    },
}

/// Parse a thinking-level string into the typed enum.
pub fn parse_thinking_level(s: &str) -> Result<ThinkingLevel, XyDriverError> {
    ThinkingLevel::parse(s)
        .ok_or_else(|| XyDriverError::invalid_input(format!("unknown thinking level: {s}")))
}

/// Dispatch a non-Prompt, non-Quit, non-WS Command against `driver`.
///
/// `Prompt`, `Quit`, `Subscribe`, `ApproveTool`, `AnswerQuestion` are NOT
/// handled here — callers must match those before calling this function.
/// Reaching one of them here is a caller bug and returns an error.
pub async fn dispatch(
    driver: &mut dyn XyDriver,
    cmd: Command,
) -> Result<DispatchOutcome, XyDriverError> {
    match cmd {
        Command::Abort { .. } => {
            // The XyDriver::abort cancels the active run loop. Whether something
            // was actually running is caller/transport-dependent; we report
            // cancelled=true optimistically (tui only calls Abort when a
            // loop is active).
            driver.abort();
            Ok(DispatchOutcome::Aborted { cancelled: true })
        }
        Command::GetState { .. } => Ok(DispatchOutcome::State(driver.get_state())),
        Command::SetModel { model_id, .. } => {
            let m = driver.select_model(&model_id)?;
            Ok(DispatchOutcome::Model(m))
        }
        Command::CycleModel { .. } => {
            let m = driver.cycle_model()?;
            Ok(DispatchOutcome::Model(m))
        }
        Command::GetAvailableModels { .. } => {
            Ok(DispatchOutcome::Models(driver.available_models()))
        }
        Command::SetThinkingLevel { level, .. } => {
            let tl = parse_thinking_level(&level)?;
            driver.set_thinking_level(tl)?;
            Ok(DispatchOutcome::ThinkingLevel(tl))
        }
        Command::Bash {
            command,
            exclude_from_context,
            ..
        } => {
            let r = driver
                .execute_bash(&command, exclude_from_context, None)
                .await?;
            Ok(DispatchOutcome::Bash(r))
        }
        Command::Compact { .. } => {
            let did = driver.compact().await?;
            Ok(DispatchOutcome::Compacted(did))
        }
        Command::GetSessionStats { .. } => {
            let stats = driver.get_session_stats().await?;
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
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("export.html"));
            let written = driver.export_html(&path).await?;
            Ok(DispatchOutcome::ExportedPath(written))
        }
        Command::ExportJsonl { output_path, .. } => {
            let path = output_path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("export.jsonl"));
            let written = driver.export_jsonl(&path).await?;
            Ok(DispatchOutcome::ExportedPath(written))
        }
        Command::ImportJsonl { input_path, .. } => {
            let path = PathBuf::from(&input_path);
            let new_id = driver.import_jsonl(&path).await?;
            Ok(DispatchOutcome::NewSession(new_id))
        }
        Command::SwitchSession { session_path, .. } => {
            // Derive session id from path (file stem).
            let new_id = std::path::Path::new(&session_path)
                .file_stem()
                .and_then(|st| st.to_str())
                .unwrap_or(&session_path)
                .to_string();
            let switched = driver.switch_session(&new_id).await?;
            Ok(DispatchOutcome::SwitchedSession(switched))
        }
        Command::Fork {
            entry_id, position, ..
        } => {
            let pos = match position.as_deref() {
                Some("before") => crate::domain::session_types::ForkPosition::Before,
                _ => crate::domain::session_types::ForkPosition::At,
            };
            let new_id = driver.fork_session(&entry_id, pos).await?;
            Ok(DispatchOutcome::NewSession(new_id))
        }
        Command::GetMessages { .. } => {
            let entries = driver.get_messages().await?;
            let session_id = driver.session_id().unwrap_or_default();
            Ok(DispatchOutcome::Messages {
                session_id,
                entries,
            })
        }
        Command::GetCommands { .. } => Ok(DispatchOutcome::Commands(driver.get_commands())),
        Command::Steer { message, .. } => {
            driver.steer(&message)?;
            let stats = driver.queue_stats();
            Ok(DispatchOutcome::QueueStats {
                steer_count: stats.steer_count,
                follow_up_count: stats.follow_up_count,
            })
        }
        Command::FollowUp { message, .. } => {
            driver.follow_up(&message)?;
            let stats = driver.queue_stats();
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
            driver.clear_queue(clear_steer, clear_follow_up)?;
            let stats = driver.queue_stats();
            Ok(DispatchOutcome::QueueStats {
                steer_count: stats.steer_count,
                follow_up_count: stats.follow_up_count,
            })
        }

        // These variants are the caller's responsibility (see module docs).
        Command::Prompt { .. }
        | Command::Quit { .. }
        | Command::Subscribe { .. }
        | Command::ApproveTool { .. }
        | Command::AnswerQuestion { .. } => Err(XyDriverError::invalid_input(format!(
            "Command variant {:?} is not handled by shared dispatch; the caller must handle it before calling dispatch()",
            cmd_variant_name(&cmd)
        ))),
    }
}

/// Return a stable name for a Command variant (for error messages).
fn cmd_variant_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::Prompt { .. } => "Prompt",
        Command::Quit { .. } => "Quit",
        Command::Subscribe { .. } => "Subscribe",
        Command::ApproveTool { .. } => "ApproveTool",
        Command::AnswerQuestion { .. } => "AnswerQuestion",
        _ => "(other)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::driver::{CommandInfo, ModelInfo, SessionState};
    use crate::domain::session_types::SessionTreeKind;
    use crate::domain::types::ThinkingLevel;
    use crate::runtime_protocol::XyBashResult;
    use async_trait::async_trait;

    /// A stub XyDriver that records calls and returns canned responses, so the
    /// dispatcher's Command→method mapping can be asserted without an agent.
    struct StubDriver {
        thinking: ThinkingLevel,
        session_id: Option<String>,
        steer: usize,
        follow_up: usize,
    }

    #[async_trait]
    impl XyDriver for StubDriver {
        async fn run(&mut self, _prompt: &str) -> crate::app::core::driver::EventStream {
            unimplemented!()
        }
        fn abort(&self) {}
        fn current_model(&self) -> Option<ModelInfo> {
            Some(ModelInfo {
                id: "m1".into(),
                display_name: "M1".into(),
                thinking: true,
                context_window: 100,
            })
        }
        fn available_models(&self) -> Vec<ModelInfo> {
            vec![self.current_model().unwrap()]
        }
        fn select_model(&mut self, id: &str) -> Result<ModelInfo, XyDriverError> {
            Ok(ModelInfo {
                id: id.into(),
                display_name: id.into(),
                thinking: true,
                context_window: 0,
            })
        }
        fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
            Ok(self.current_model().unwrap())
        }
        fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), XyDriverError> {
            self.thinking = level;
            Ok(())
        }
        fn thinking_level(&self) -> ThinkingLevel {
            self.thinking
        }
        fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, XyDriverError> {
            let levels = ThinkingLevel::STANDARD;
            let idx = levels.iter().position(|l| *l == self.thinking).unwrap_or(0);
            let next = levels[(idx + 1) % levels.len()];
            self.thinking = next;
            Ok(next)
        }
        fn session_id(&self) -> Option<String> {
            self.session_id.clone()
        }
        async fn execute_bash(
            &self,
            _command: &str,
            _exclude_from_context: bool,
            _chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
        ) -> Result<XyBashResult, XyDriverError> {
            unimplemented!()
        }
        async fn compact(&mut self) -> Result<bool, XyDriverError> {
            Ok(false)
        }
        async fn export_html(&mut self, path: &std::path::Path) -> Result<String, XyDriverError> {
            Ok(path.to_string_lossy().into_owned())
        }
        async fn export_jsonl(&mut self, path: &std::path::Path) -> Result<String, XyDriverError> {
            Ok(path.to_string_lossy().into_owned())
        }
        async fn import_jsonl(&mut self, _path: &std::path::Path) -> Result<String, XyDriverError> {
            Ok("new-session".into())
        }
        async fn fork_session(
            &mut self,
            _entry_id: &str,
            _position: crate::domain::session_types::ForkPosition,
        ) -> Result<String, XyDriverError> {
            Ok("forked-session".into())
        }
        async fn switch_session(&mut self, id: &str) -> Result<String, XyDriverError> {
            self.session_id = Some(id.into());
            Ok(id.into())
        }
        async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
            Ok(Vec::new())
        }
        async fn get_session_stats(
            &self,
        ) -> Result<crate::app::core::driver::SessionStats, XyDriverError> {
            unimplemented!()
        }
        async fn estimate_context_tokens(
            &self,
        ) -> Result<crate::domain::types::ContextTokenEstimate, XyDriverError> {
            Ok(crate::domain::types::ContextTokenEstimate {
                tokens: 0,
                provenance: crate::domain::types::TokenProvenance::Unknown,
                usage_tokens: 0,
                trailing_tokens: 0,
                last_usage_index: None,
            })
        }
        fn get_commands(&self) -> Vec<CommandInfo> {
            vec![CommandInfo {
                name: "compact".into(),
                description: "compact".into(),
            }]
        }
        fn steer(&mut self, _message: &str) -> Result<(), XyDriverError> {
            self.steer += 1;
            Ok(())
        }
        fn follow_up(&mut self, _message: &str) -> Result<(), XyDriverError> {
            self.follow_up += 1;
            Ok(())
        }
        fn clear_queue(
            &mut self,
            clear_steer: bool,
            clear_follow_up: bool,
        ) -> Result<(), XyDriverError> {
            if clear_steer {
                self.steer = 0;
            }
            if clear_follow_up {
                self.follow_up = 0;
            }
            Ok(())
        }
        fn queue_stats(&self) -> crate::agent::session::QueueStats {
            crate::agent::session::QueueStats {
                steer_count: self.steer,
                follow_up_count: self.follow_up,
            }
        }

        async fn session_tree(
            &self,
            kind: SessionTreeKind,
        ) -> Result<Vec<crate::domain::session_types::SessionTreeNode>, XyDriverError> {
            match kind {
                SessionTreeKind::MessageHistory => Ok(Vec::new()),
                SessionTreeKind::FileBrowser => {
                    Err("session tree kind 'file_browser' is not implemented".into())
                }
            }
        }

        async fn travel_session_tree(
            &self,
            kind: SessionTreeKind,
            _entry_id: &str,
        ) -> Result<crate::domain::session_types::SessionTreeTravel, XyDriverError> {
            match kind {
                SessionTreeKind::MessageHistory => {
                    Err("stub: travel_session_tree not implemented".into())
                }
                SessionTreeKind::FileBrowser => {
                    Err("session tree kind 'file_browser' is not implemented".into())
                }
            }
        }

        async fn append_entry_label(
            &mut self,
            _target_id: &str,
            _label: Option<&str>,
        ) -> Result<(), XyDriverError> {
            Ok(())
        }

        fn leaf_entry_id(&self) -> Option<String> {
            None
        }

        async fn load_debug_scene(
            &mut self,
            _scene: &str,
        ) -> Result<crate::app::core::driver::DebugSceneLoad, XyDriverError> {
            Err("stub: load_debug_scene not implemented".into())
        }

        async fn list_sessions(
            &self,
        ) -> Result<Vec<crate::app::core::driver::SessionListEntry>, XyDriverError> {
            Ok(Vec::new())
        }

        async fn new_session(&mut self) -> Result<String, XyDriverError> {
            Ok("new-session".into())
        }

        async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
            Ok(None)
        }

        async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
            Ok(name.trim().to_string())
        }

        async fn set_session_name_for(
            &mut self,
            _session_id: &str,
            name: &str,
        ) -> Result<String, XyDriverError> {
            Ok(name.trim().to_string())
        }

        async fn delete_session(&mut self, _session_id: &str) -> Result<(), XyDriverError> {
            Err("stub: delete_session not implemented".into())
        }

        async fn loaded_resources_snapshot(
            &self,
        ) -> crate::app::core::driver::LoadedResourcesSnapshot {
            crate::app::core::driver::LoadedResourcesSnapshot::default()
        }
    }

    fn stub() -> StubDriver {
        StubDriver {
            thinking: ThinkingLevel::Medium,
            session_id: Some("s1".into()),
            steer: 0,
            follow_up: 0,
        }
    }

    #[tokio::test]
    async fn get_state_returns_snapshot() {
        let mut d = stub();
        let outcome = dispatch(&mut d, Command::GetState { id: None })
            .await
            .unwrap();
        match outcome {
            DispatchOutcome::State(SessionState {
                session_id, model, ..
            }) => {
                assert_eq!(session_id, "s1");
                assert_eq!(model.unwrap().id, "m1");
            }
            _ => panic!("expected State"),
        }
    }

    #[tokio::test]
    async fn set_thinking_level_round_trips() {
        let mut d = stub();
        let outcome = dispatch(
            &mut d,
            Command::SetThinkingLevel {
                id: None,
                level: "high".into(),
            },
        )
        .await
        .unwrap();
        match outcome {
            DispatchOutcome::ThinkingLevel(ThinkingLevel::High) => {}
            _ => panic!("expected ThinkingLevel High"),
        }
        assert_eq!(d.thinking_level(), ThinkingLevel::High);
    }

    /// c1165: XyDriver level after SetThinkingLevel / cycle MUST map to OpenAI effort.
    #[tokio::test]
    async fn cycle_thinking_level_maps_to_openai_reasoning_effort() {
        use crate::domain::types::{
            ResolvedThinking, ThinkingAdapterKind, resolve_thinking_for_request,
        };

        let mut d = stub();
        assert_eq!(d.thinking_level(), ThinkingLevel::Medium);
        let mid = resolve_thinking_for_request(
            d.thinking_level(),
            None,
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(mid, ResolvedThinking::OpenAiEffort("medium".into()));

        assert_eq!(d.cycle_thinking_level().unwrap(), ThinkingLevel::High);
        let high = resolve_thinking_for_request(
            d.thinking_level(),
            None,
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(high, ResolvedThinking::OpenAiEffort("high".into()));

        dispatch(
            &mut d,
            Command::SetThinkingLevel {
                id: None,
                level: "off".into(),
            },
        )
        .await
        .unwrap();
        let off = resolve_thinking_for_request(
            d.thinking_level(),
            None,
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(off, ResolvedThinking::Omit);
    }

    #[tokio::test]
    async fn reject_invalid_thinking_level() {
        let mut d = stub();
        let err = dispatch(
            &mut d,
            Command::SetThinkingLevel {
                id: None,
                level: "bogus".into(),
            },
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("unknown thinking level"));
    }

    #[tokio::test]
    async fn prompt_is_caller_responsibility() {
        let mut d = stub();
        let err = dispatch(
            &mut d,
            Command::Prompt {
                id: None,
                message: "hi".into(),
            },
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("not handled by shared dispatch"));
    }

    #[tokio::test]
    async fn steer_and_clear_queue_round_trip() {
        let mut d = stub();
        let outcome = dispatch(
            &mut d,
            Command::Steer {
                id: None,
                message: "nudge".into(),
            },
        )
        .await
        .unwrap();
        match outcome {
            DispatchOutcome::QueueStats {
                steer_count,
                follow_up_count,
            } => {
                assert_eq!(steer_count, 1);
                assert_eq!(follow_up_count, 0);
            }
            _ => panic!("expected QueueStats"),
        }

        let outcome = dispatch(
            &mut d,
            Command::ClearQueue {
                id: None,
                clear_steer: true,
                clear_follow_up: false,
            },
        )
        .await
        .unwrap();
        match outcome {
            DispatchOutcome::QueueStats {
                steer_count,
                follow_up_count,
            } => {
                assert_eq!(steer_count, 0);
                assert_eq!(follow_up_count, 0);
            }
            _ => panic!("expected QueueStats"),
        }
    }

    #[tokio::test]
    async fn switch_session_derives_id_from_path_stem() {
        let mut d = stub();
        let outcome = dispatch(
            &mut d,
            Command::SwitchSession {
                id: None,
                session_path: "/tmp/sessions/abc.jsonl".into(),
            },
        )
        .await
        .unwrap();
        match outcome {
            DispatchOutcome::SwitchedSession(id) => assert_eq!(id, "abc"),
            _ => panic!("expected SwitchedSession"),
        }
    }
}
