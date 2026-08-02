//! Process-local ask gateway: tool execute ↔ ChoicePrompt oneshot (c1850).

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::oneshot;
use xylitol_tui::{ChoiceMode, ChoiceOption, ChoiceQuestion};

use crate::agent::tools::ToolSet;
use crate::infra::tools::{
    AskArgs, AskModeArg, AskQuestionArg, AskUserGateway, default_tools_with_ask,
};
use crate::protocol::error::XyToolError;

/// Pending ask waiting for the TUI Choice slot.
pub struct PendingAsk {
    pub questions: Vec<AskQuestionArg>,
    pub reply: oneshot::Sender<Result<String, XyToolError>>,
}

/// Channel-based [`AskUserGateway`] shared by AskTool and the TUI host.
pub struct AskHostGateway {
    pending: Mutex<Option<PendingAsk>>,
}

impl AskHostGateway {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(None),
        }
    }

    /// Take one pending ask (if any) for the host to mount ChoicePrompt.
    pub fn take_pending(&self) -> Option<PendingAsk> {
        self.pending.lock().ok()?.take()
    }
}

impl Default for AskHostGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AskUserGateway for AskHostGateway {
    async fn prompt(&self, args: AskArgs) -> Result<String, XyToolError> {
        let (tx, rx) = oneshot::channel();
        {
            let mut slot = self.pending.lock().map_err(|_| {
                XyToolError::ExecutionFailed(anyhow::anyhow!("ask gateway poisoned"))
            })?;
            if slot.is_some() {
                return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "ask already waiting for user"
                )));
            }
            *slot = Some(PendingAsk {
                questions: args.questions,
                reply: tx,
            });
        }
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(XyToolError::Aborted),
        }
    }
}

/// `default_tools` + ask — TUI composition / MCP reload only.
pub fn toolset_with_ask(gateway: Arc<dyn AskUserGateway>) -> ToolSet {
    ToolSet::from_iter(default_tools_with_ask(gateway))
}

/// Convert ask tool args → package ChoicePrompt questions.
pub fn ask_questions_to_choice(questions: Vec<AskQuestionArg>) -> Vec<ChoiceQuestion> {
    questions
        .into_iter()
        .map(|q| {
            let label = q
                .label
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| q.id.clone());
            let mode = match q.mode {
                AskModeArg::Single => ChoiceMode::Single,
                AskModeArg::Multi => ChoiceMode::Multi,
            };
            let options = q
                .options
                .into_iter()
                .map(|o| {
                    let mut opt = ChoiceOption::new(o.value, o.label);
                    if let Some(desc) = o.description.filter(|s| !s.is_empty()) {
                        opt = opt.with_description(desc);
                    }
                    if o.recommended {
                        opt = opt.recommended();
                    }
                    opt
                })
                .collect();
            ChoiceQuestion {
                id: q.id,
                label,
                prompt: q.prompt,
                mode,
                options,
                allow_other: q.allow_other,
            }
        })
        .collect()
}
