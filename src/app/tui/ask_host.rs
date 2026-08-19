//! Process-local ask gateway: tool execute ↔ ChoicePrompt oneshot (c1850).

use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::oneshot;
use xylitol_tui::{ChoiceMode, ChoiceOption, ChoiceQuestion};

use crate::protocol::error::XyToolError;
use crate::protocol::ports::ask::{AskArgs, AskModeArg, AskQuestionArg, AskUserGateway};

/// Pending ask waiting for the TUI Choice slot.
pub struct PendingAsk {
    pub questions: Vec<AskQuestionArg>,
    pub reply: oneshot::Sender<Result<String, XyToolError>>,
}

/// Channel-based [`AskUserGateway`] shared by AskTool and the TUI host.
///
/// When a [`HostClient`] is set (product TUI attach), reverse-RPC answers go
/// through `POST /api/respond` instead of a local fake.
pub struct AskHostGateway {
    pending: Mutex<Option<PendingAsk>>,
    host: Mutex<Option<std::sync::Arc<dyn crate::app::core::host_client::HostClient>>>,
}

impl AskHostGateway {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(None),
            host: Mutex::new(None),
        }
    }

    /// Attach-mode: answers are posted to Host via [`HostClient::respond`].
    pub fn set_host_client(
        &self,
        client: std::sync::Arc<dyn crate::app::core::host_client::HostClient>,
    ) {
        if let Ok(mut slot) = self.host.lock() {
            *slot = Some(client);
        }
    }

    /// Take one pending ask (if any) for the host to mount ChoicePrompt.
    pub fn take_pending(&self) -> Option<PendingAsk> {
        self.pending.lock().ok()?.take()
    }

    /// Mux `approval/requested` / `question/requested` → local ChoicePrompt, then respond.
    pub fn push_from_server(&self, rpc_id: String, method: &str, payload: serde_json::Value) {
        use crate::protocol::ports::ask::{AskModeArg, AskOptionArg, AskQuestionArg};
        let questions = if method == "approval/requested" {
            vec![AskQuestionArg {
                id: "approved".into(),
                prompt: "Host requests tool approval".into(),
                label: Some("Approval".into()),
                mode: AskModeArg::Single,
                options: vec![
                    AskOptionArg {
                        value: "true".into(),
                        label: "Approve".into(),
                        description: None,
                        recommended: true,
                    },
                    AskOptionArg {
                        value: "false".into(),
                        label: "Deny".into(),
                        description: None,
                        recommended: false,
                    },
                ],
                allow_other: false,
            }]
        } else {
            payload
                .get("questions")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default()
        };
        let (tx, rx) = oneshot::channel();
        {
            let Ok(mut slot) = self.pending.lock() else {
                return;
            };
            *slot = Some(PendingAsk {
                questions,
                reply: tx,
            });
        }
        let host = self.host.lock().ok().and_then(|g| g.clone());
        let kind_approval = method == "approval/requested";
        tokio::spawn(async move {
            let answered = rx.await;
            let Some(host) = host else {
                return;
            };
            let payload = match answered {
                Ok(Ok(text)) if kind_approval => {
                    let approved = text.contains("true") && !text.contains("false");
                    serde_json::json!({ "approved": approved })
                }
                Ok(Ok(text)) => serde_json::json!({ "answer": text }),
                Ok(Err(_)) | Err(_) => {
                    if kind_approval {
                        serde_json::json!({ "approved": false })
                    } else {
                        serde_json::json!({ "answer": "" })
                    }
                }
            };
            let _ = host.respond(&rpc_id, payload).await;
        });
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
