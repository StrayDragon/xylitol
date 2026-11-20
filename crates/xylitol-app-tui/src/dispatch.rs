pub use crate::driver_error::XyDriverError;
pub use xylitol_plugin_sdk::dispatch::{
    DispatchOutcome, DispatchRejection, SessionCommandExecutor, transport_variant_error,
    validate_nonempty_thinking_level, variant_name,
};

use crate::driver::XyDriver;
use xylitol_protocol::Command;

pub async fn dispatch(
    driver: &mut dyn XyDriver,
    cmd: Command,
) -> Result<DispatchOutcome, XyDriverError> {
    let where_ = format!("dispatch.{}", variant_name(&cmd));
    match driver.execute_session_command(cmd).await {
        Ok(outcome) => Ok(outcome),
        Err(rejection) => {
            let err = XyDriverError::from(rejection);
            err.log_failure(&where_);
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{CommandInfo, LoadedResourcesSnapshot, ModelInfo, SessionState};
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
        ) -> Result<DispatchOutcome, DispatchRejection> {
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
                contexts: Vec::new(),
            },
            Command::Quit {},
            Command::Subscribe {
                session_id: "s".into(),
                last_seq: 0,
                workspaces: Vec::new(),
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
