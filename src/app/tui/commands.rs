//! Slash command parsing for the TUI, delegating execution to the shared
//! [`app::core::dispatch`](crate::app::core::dispatch) (spec ce10).
//!
//! This module turns a typed slash body (`model`, `exit`, ...) into a
//! `protocol::Command` and hands it to the shared dispatcher. It does NOT
//! implement command execution itself — that lives behind the Driver, shared
//! across surfaces. `/exit`/`/quit` map to `Quit` (handled locally, controls
//! the REPL loop). WS-only variants (Subscribe/ApproveTool/AnswerQuestion) are
//! not reachable from the TUI and are rejected by the shared dispatch.

use crate::app::core::dispatch::{DispatchOutcome, dispatch as shared_dispatch};
use crate::app::core::driver::Driver;

/// The result of evaluating a parsed slash command.
pub enum CommandOutcome {
    /// `/exit` — the REPL should stop.
    Quit,
    /// Command handled (e.g. model switched); `text` is an optional inline
    /// confirmation message to show.
    Handled(Option<String>),
    /// Unknown command or dispatch error: `text` is shown inline.
    Unknown(String),
}

/// Parse and dispatch a slash command body (the part after `/`).
///
/// `driver` is the live agent handle; commands that need agent interaction
/// (`/model`) go through the shared dispatch → Driver path.
pub async fn dispatch(body: &str, driver: &mut dyn Driver) -> CommandOutcome {
    let body = body.trim();
    let (name, rest) = split_once_space(body);
    match name {
        "exit" | "quit" => CommandOutcome::Quit,
        "model" => {
            // `/model` → cycle; `/model <id>` → set. The shared dispatch matches
            // by model id or config.model alias, so the bare alias works.
            let cmd = if rest.is_empty() {
                crate::protocol::Command::CycleModel { id: None }
            } else {
                crate::protocol::Command::SetModel {
                    id: None,
                    provider: String::new(),
                    model_id: rest.to_string(),
                }
            };
            match shared_dispatch(driver, cmd).await {
                Ok(DispatchOutcome::Model(m)) => {
                    CommandOutcome::Handled(Some(format!("model: {}", m.display_name)))
                }
                Ok(_) => CommandOutcome::Handled(None),
                Err(e) => CommandOutcome::Unknown(e.0),
            }
        }
        "help" => CommandOutcome::Handled(Some("commands: /exit, /model [<id>]".into())),
        other => CommandOutcome::Unknown(format!("unknown command: /{other}")),
    }
}

fn split_once_space(s: &str) -> (&str, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::driver::{CommandInfo, ModelInfo, SessionState};
    use crate::domain::session_types::SessionEntry;
    use crate::domain::types::ThinkingLevel;
    use crate::runtime_protocol::XyBashResult;
    use async_trait::async_trait;

    /// Stub driver with one fake model "m1", so `/model` cycle + `/model m1`
    /// set can both succeed end-to-end through the shared dispatch.
    struct StubDriver {
        selected: String,
    }

    #[async_trait]
    impl Driver for StubDriver {
        async fn run(&mut self, _prompt: &str) -> crate::app::core::driver::EventStream {
            unimplemented!()
        }
        fn abort(&self) {}
        fn current_model(&self) -> Option<ModelInfo> {
            Some(ModelInfo {
                id: self.selected.clone(),
                display_name: self.selected.clone(),
                thinking: true,
                context_window: 100,
            })
        }
        fn available_models(&self) -> Vec<ModelInfo> {
            vec![ModelInfo {
                id: "m1".into(),
                display_name: "M1".into(),
                thinking: true,
                context_window: 100,
            }]
        }
        fn select_model(&mut self, id: &str) -> Result<ModelInfo, String> {
            self.selected = id.into();
            Ok(self.current_model().unwrap())
        }
        fn cycle_model(&mut self) -> Result<ModelInfo, String> {
            self.selected = "m1".into();
            Ok(self.current_model().unwrap())
        }
        fn set_thinking_level(&mut self, _level: ThinkingLevel) {}
        fn thinking_level(&self) -> ThinkingLevel {
            ThinkingLevel::Medium
        }
        fn session_id(&self) -> Option<String> {
            Some("s1".into())
        }
        async fn execute_bash(
            &mut self,
            _command: &str,
            _exclude_from_context: bool,
        ) -> Result<XyBashResult, String> {
            unimplemented!()
        }
        async fn compact(&mut self) -> Result<bool, String> {
            Ok(false)
        }
        async fn export_html(&mut self, _path: &std::path::Path) -> Result<String, String> {
            unimplemented!()
        }
        async fn export_jsonl(&mut self, _path: &std::path::Path) -> Result<String, String> {
            unimplemented!()
        }
        async fn import_jsonl(&mut self, _path: &std::path::Path) -> Result<String, String> {
            unimplemented!()
        }
        async fn fork_session(&mut self, _entry_id: &str) -> Result<String, String> {
            unimplemented!()
        }
        async fn switch_session(&mut self, _id: &str) -> Result<String, String> {
            unimplemented!()
        }
        async fn get_messages(&self) -> Result<Vec<SessionEntry>, String> {
            unimplemented!()
        }
        async fn get_session_stats(
            &self,
        ) -> Result<crate::app::core::driver::SessionStats, String> {
            unimplemented!()
        }
        fn get_commands(&self) -> Vec<CommandInfo> {
            Vec::new()
        }
    }

    #[tokio::test]
    async fn exit_quits() {
        assert!(matches!(
            dispatch(
                "exit",
                &mut StubDriver {
                    selected: "m1".into()
                }
            )
            .await,
            CommandOutcome::Quit
        ));
        assert!(matches!(
            dispatch(
                "quit",
                &mut StubDriver {
                    selected: "m1".into()
                }
            )
            .await,
            CommandOutcome::Quit
        ));
    }

    #[tokio::test]
    async fn model_no_arg_cycles_via_shared_dispatch() {
        let outcome = dispatch(
            "model",
            &mut StubDriver {
                selected: "x".into(),
            },
        )
        .await;
        match outcome {
            CommandOutcome::Handled(Some(msg)) => assert!(msg.contains("model:")),
            other => panic!("expected Handled(Some), got {:?}", other_kind(other)),
        }
    }

    #[tokio::test]
    async fn model_with_arg_sets_via_shared_dispatch() {
        let outcome = dispatch(
            "model m1",
            &mut StubDriver {
                selected: "x".into(),
            },
        )
        .await;
        assert!(matches!(outcome, CommandOutcome::Handled(Some(_))));
    }

    #[tokio::test]
    async fn unknown_reports() {
        match dispatch(
            "nope",
            &mut StubDriver {
                selected: "m1".into(),
            },
        )
        .await
        {
            CommandOutcome::Unknown(msg) => assert!(msg.contains("/nope")),
            other => panic!("expected Unknown, got {:?}", other_kind(other)),
        }
    }

    // Variant debug helper kept out of the enum to avoid deriving Debug.
    fn other_kind(_: CommandOutcome) -> &'static str {
        "other"
    }

    // SessionState unused in stub but referenced for type resolution.
    #[allow(dead_code)]
    type _Unused = SessionState;
}
