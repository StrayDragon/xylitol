//! Shared Command execution — the single execution path for `protocol::Command`
//! variants, consumed by tui (spec ce10) and the server Host.
//!
//! The session operation table IS the [`Command`] enum (c2710): a `Command`
//! variant is dispatched to a [`SessionCommandExecutor`] — in-process drives the
//! runtime directly, remote forwards via a single unary. [`XyDriver`] no longer
//! mirrors the command table; it carries surface/transport capabilities only.
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

use async_trait::async_trait;

use crate::app::core::driver::{
    CommandInfo, LoadedResourcesSnapshot, ModelInfo, RuntimeReloadReport, SessionListEntry,
    SessionState, XyDriver,
};
pub use crate::app::core::driver_error::XyDriverError;
use crate::protocol::Command;
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{SessionEntry, SessionTreeNode, SessionTreeTravel};

/// Executor for session-level Commands (c2710): a `Command` is the SSOT of a
/// session operation. In-process handlers drive the runtime directly; remote
/// forwards via a single typed unary.
///
/// `Prompt`, `Quit`, `Subscribe`, `ApproveTool`, `AnswerQuestion` are NOT
/// handled by executors — callers must match those before dispatching.
#[async_trait]
pub trait SessionCommandExecutor: Send {
    async fn execute_session_command(
        &mut self,
        cmd: Command,
    ) -> Result<DispatchOutcome, XyDriverError>;
}

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
    ThinkingLevel(String),
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
    /// `SessionTree` — the current session message tree.
    SessionTree(Vec<SessionTreeNode>),
    /// `TravelSessionTree` — the selected tree position and editor prefill.
    SessionTreeTravel(SessionTreeTravel),
    /// `ListSessions` — resumable session rows.
    Sessions(Vec<SessionListEntry>),
    /// `LoadSessionEntries` — raw entries for a named session.
    SessionEntries(Vec<SessionEntry>),
    /// `GetSessionName` / `SetSessionName` — current display name.
    SessionName(Option<String>),
    /// `Reload` — host runtime resource reload report.
    Reload(RuntimeReloadReport),
    /// `LoadedResources` — host resource snapshot.
    LoadedResources(LoadedResourcesSnapshot),
    /// Mutating command with no value payload.
    Empty,
    /// `GetCommands` — the available slash commands.
    Commands(Vec<CommandInfo>),
    /// `Steer` / `FollowUp` / `ClearQueue` — current queue depths.
    QueueStats {
        steer_count: usize,
        follow_up_count: usize,
    },
}

/// Validate a freeform thinking-level request without changing its spelling.
pub fn validate_nonempty_thinking_level(s: &str) -> Result<(), XyDriverError> {
    if s.trim().is_empty() {
        return Err(XyDriverError::invalid_input(
            "thinking level must not be empty",
        ));
    }
    Ok(())
}

/// Dispatch a non-Prompt, non-Quit, non-WS Command against `driver`.
///
/// `Prompt`, `Quit`, `Subscribe`, `ApproveTool`, `AnswerQuestion` are NOT
/// handled here — callers must match those before calling this function.
/// Reaching one of them here is a caller bug and returns an error.
/// Variant name for a Command (log `where` / error messages); SSOT is the
/// `strum::IntoStaticStr` derive on [`Command`].
fn variant_name(cmd: &Command) -> &'static str {
    cmd.into()
}

pub async fn dispatch(
    driver: &mut dyn XyDriver,
    cmd: Command,
) -> Result<DispatchOutcome, XyDriverError> {
    // `XyDriver: SessionCommandExecutor` — every driver executes commands;
    // in-process directly, remote over a single typed unary (c2710).
    let where_ = format!("dispatch.{}", variant_name(&cmd));
    match driver.execute_session_command(cmd).await {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            err.log_failure(&where_);
            Err(err)
        }
    }
}

/// Shared helper: reject transport-owned Command variants inside an executor.
///
/// Executors MUST reach this (or an equivalent error) for the five variants
/// the shared path does not own.
pub fn transport_variant_error(cmd: &Command) -> XyDriverError {
    let name: &'static str = cmd.into();
    XyDriverError::invalid_input(format!(
        "Command variant {name:?} is not handled by shared dispatch; the caller must handle it before calling dispatch()"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::driver::ModelInfo;
    use async_trait::async_trait;

    /// Canned executor stub: dispatch is a thin wrapper (c2710) — the executor
    /// owns the Command semantics, so these tests only pin the wrapper contract
    /// (outcome passthrough, transport-variant rejection, error logging path).
    struct StubDriver {
        thinking: String,
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
                thinking_levels: Vec::new(),
                context_window: 100,
            })
        }
        fn available_models(&self) -> Vec<ModelInfo> {
            vec![self.current_model().unwrap()]
        }
        fn thinking_level(&self) -> String {
            self.thinking.clone()
        }
        async fn cycle_thinking_level(&mut self) -> Result<String, XyDriverError> {
            Ok(self.thinking.clone())
        }
        fn session_id(&self) -> Option<String> {
            None
        }
        async fn execute_bash(
            &self,
            _command: &str,
            _exclude_from_context: bool,
            _chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
        ) -> Result<XyBashResult, XyDriverError> {
            unimplemented!()
        }
        fn get_commands(&self) -> Vec<CommandInfo> {
            Vec::new()
        }
        async fn estimate_context_tokens(
            &self,
        ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError> {
            Err(XyDriverError::unsupported("stub"))
        }
        fn leaf_entry_id(&self) -> Option<String> {
            None
        }
        async fn load_debug_scene(
            &mut self,
            _scene: &str,
        ) -> Result<crate::app::core::driver::DebugSceneLoad, XyDriverError> {
            Err(XyDriverError::unsupported("stub"))
        }
        async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
            LoadedResourcesSnapshot::default()
        }
    }

    #[async_trait]
    impl SessionCommandExecutor for StubDriver {
        async fn execute_session_command(
            &mut self,
            cmd: Command,
        ) -> Result<DispatchOutcome, XyDriverError> {
            match cmd {
                Command::SetThinkingLevel { level, .. } => {
                    validate_nonempty_thinking_level(&level)?;
                    self.thinking = level.clone();
                    Ok(DispatchOutcome::ThinkingLevel(level))
                }
                Command::SetModel { model_id, .. } => Ok(DispatchOutcome::Model(ModelInfo {
                    id: model_id.clone(),
                    display_name: model_id,
                    thinking: false,
                    thinking_levels: Vec::new(),
                    context_window: 8_000,
                })),
                Command::GetState { .. } => Ok(DispatchOutcome::State(SessionState {
                    session_id: "s".into(),
                    model: self.current_model(),
                    thinking_level: self.thinking.clone(),
                    leaf_entry_id: None,
                })),
                Command::GetAvailableModels { .. } => {
                    Ok(DispatchOutcome::Models(self.available_models()))
                }
                other => Err(transport_variant_error(&other)),
            }
        }
    }

    fn stub() -> StubDriver {
        StubDriver {
            thinking: "off".into(),
        }
    }

    #[tokio::test]
    async fn dispatch_passes_executor_outcome_through() {
        let mut driver = stub();
        let outcome = dispatch(
            &mut driver,
            Command::SetModel {
                provider: String::new(),
                model_id: "m2".into(),
            },
        )
        .await
        .expect("ok");
        assert!(matches!(outcome, DispatchOutcome::Model(m) if m.id == "m2"));
    }

    #[tokio::test]
    async fn set_thinking_level_validates_and_round_trips_via_executor() {
        let mut driver = stub();
        let outcome = dispatch(
            &mut driver,
            Command::SetThinkingLevel {
                level: "high".into(),
            },
        )
        .await
        .expect("ok");
        assert!(matches!(outcome, DispatchOutcome::ThinkingLevel(l) if l == "high"));
        assert_eq!(driver.thinking, "high");
    }

    #[tokio::test]
    async fn blank_thinking_level_rejected() {
        let mut driver = stub();
        let err = dispatch(
            &mut driver,
            Command::SetThinkingLevel { level: "  ".into() },
        )
        .await
        .expect_err("blank rejected");
        assert_eq!(err.kind(), "InvalidInput");
    }

    #[tokio::test]
    async fn transport_variants_rejected() {
        let mut driver = stub();
        for cmd in [
            Command::Prompt {
                message: "hi".into(),
            },
            Command::Quit {},
            Command::Subscribe {
                session_id: "s".into(),
                last_seq: 0,
            },
            Command::ApproveTool {
                call_id: "c".into(),
                approved: true,
            },
            Command::AnswerQuestion {
                call_id: "c".into(),
                answer: "a".into(),
            },
        ] {
            let err = dispatch(&mut driver, cmd).await.expect_err("rejected");
            assert_eq!(err.kind(), "InvalidInput");
        }
    }

    #[test]
    fn thinking_level_validation_rejects_only_blank_input() {
        assert!(validate_nonempty_thinking_level("vendor-max").is_ok());
        assert!(validate_nonempty_thinking_level(" high ").is_ok());
        assert!(validate_nonempty_thinking_level("").is_err());
        assert!(validate_nonempty_thinking_level("   ").is_err());
    }
}
