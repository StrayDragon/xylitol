use async_trait::async_trait;

use crate::driver::{
    CommandInfo, LoadedResourcesSnapshot, ModelInfo, RuntimeReloadReport, SessionListEntry,
    SessionState, XyDriver,
};
pub use crate::driver_error::XyDriverError;
use xylitol_plugin_sdk::XyBashResult;
use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeNode, SessionTreeTravel};
use xylitol_protocol::Command;

#[async_trait]
pub trait SessionCommandExecutor: Send {
    async fn execute_session_command(
        &mut self,
        cmd: Command,
    ) -> Result<DispatchOutcome, XyDriverError>;
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum DispatchOutcome {
    Aborted {
        cancelled: bool,
    },
    State(SessionState),
    Model(ModelInfo),
    Models(Vec<ModelInfo>),
    ThinkingLevel(String),
    PlanMode {
        mode: String,
        plan_id: Option<String>,
    },
    Bash(XyBashResult),
    Compacted(bool),
    SessionStats(serde_json::Value),
    ExportedPath(String),
    NewSession(String),
    SwitchedSession(String),
    Messages {
        session_id: String,
        entries: Vec<SessionEntry>,
        has_more: bool,
    },
    EstimateContext(xylitol_plugin_sdk::model::ContextTokenEstimate),
    SessionTree(Vec<SessionTreeNode>),
    SessionTreeTravel(SessionTreeTravel),
    Sessions(Vec<SessionListEntry>),
    SessionEntries(Vec<SessionEntry>),
    SessionName(Option<String>),
    Reload(RuntimeReloadReport),
    LoadedResources(LoadedResourcesSnapshot),
    Capabilities(xylitol_plugin_sdk::session::CapabilitySnapshot),
    Empty,
    Commands(Vec<CommandInfo>),
    QueueStats {
        steer_count: usize,
        follow_up_count: usize,
    },
}

pub fn validate_nonempty_thinking_level(s: &str) -> Result<(), XyDriverError> {
    if s.trim().is_empty() {
        return Err(XyDriverError::invalid_input(
            "thinking level must not be empty",
        ));
    }
    Ok(())
}

fn variant_name(cmd: &Command) -> &'static str {
    cmd.into()
}

pub async fn dispatch(
    driver: &mut dyn XyDriver,
    cmd: Command,
) -> Result<DispatchOutcome, XyDriverError> {
    let where_ = format!("dispatch.{}", variant_name(&cmd));
    match driver.execute_session_command(cmd).await {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            err.log_failure(&where_);
            Err(err)
        }
    }
}

pub fn transport_variant_error(cmd: &Command) -> XyDriverError {
    let name: &'static str = cmd.into();
    XyDriverError::invalid_input(format!(
        "Command variant {name:?} is not handled by shared dispatch; the caller must handle it before calling dispatch()"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::ModelInfo;
    use async_trait::async_trait;

    struct StubDriver {
        thinking: String,
    }

    #[async_trait]
    impl XyDriver for StubDriver {
        async fn run(&mut self, _prompt: &str) -> crate::driver::EventStream {
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
        fn session_id(&self) -> Option<String> {
            None
        }
        fn get_commands(&self) -> Vec<CommandInfo> {
            Vec::new()
        }
        async fn estimate_context_tokens(
            &self,
        ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
            Err(XyDriverError::unsupported("stub"))
        }
        fn leaf_entry_id(&self) -> Option<String> {
            None
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
