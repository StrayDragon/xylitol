use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::oneshot;
use xylitol_tui::{ChoiceMode, ChoiceOption, ChoiceQuestion};

use xylitol_plugin_sdk::ask::{AskArgs, AskModeArg, AskQuestionArg, AskUserGateway};
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_protocol::method::APPROVAL_REQUESTED;

pub struct PendingAsk {
    pub questions: Vec<AskQuestionArg>,
    pub reply: oneshot::Sender<Result<String, XyToolError>>,
}

pub struct AskHostGateway {
    pending: Mutex<Option<PendingAsk>>,
    pending_remote: Mutex<Option<PendingAsk>>,
    notices: Mutex<Vec<String>>,
    host: Mutex<Option<std::sync::Arc<dyn crate::host_client::HostClient>>>,
}

impl AskHostGateway {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(None),
            pending_remote: Mutex::new(None),
            notices: Mutex::new(Vec::new()),
            host: Mutex::new(None),
        }
    }

    pub fn set_host_client(&self, client: std::sync::Arc<dyn crate::host_client::HostClient>) {
        if let Ok(mut slot) = self.host.lock() {
            *slot = Some(client);
        }
    }

    pub fn take_pending(&self) -> Option<PendingAsk> {
        let current = self.pending.lock().ok()?.take();
        if current.is_some() {
            return current;
        }
        self.pending_remote.lock().ok()?.take()
    }

    pub fn take_notices(&self) -> Vec<String> {
        self.notices
            .lock()
            .map(|mut notices| std::mem::take(&mut *notices))
            .unwrap_or_default()
    }

    fn push_notice(&self, text: impl Into<String>) {
        if let Ok(mut notices) = self.notices.lock() {
            notices.push(text.into());
        }
    }

    fn spawn_responder(
        &self,
        host: Option<std::sync::Arc<dyn crate::host_client::HostClient>>,
        call_id: String,
        kind_approval: bool,
        rx: oneshot::Receiver<Result<String, XyToolError>>,
    ) {
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
            let _ = host.respond(&call_id, payload).await;
        });
    }

    pub fn push_from_server(&self, call_id: String, method: &str, payload: serde_json::Value) {
        use xylitol_plugin_sdk::ask::{AskModeArg, AskOptionArg, AskQuestionArg};
        let questions = if method == APPROVAL_REQUESTED {
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
        let host = self.host.lock().ok().and_then(|g| g.clone());
        let kind_approval = method == APPROVAL_REQUESTED;
        {
            let Ok(mut slot) = self.pending.lock() else {
                return;
            };
            if slot.is_none() {
                *slot = Some(PendingAsk {
                    questions,
                    reply: tx,
                });
                drop(slot);
                self.spawn_responder(host, call_id, kind_approval, rx);
                return;
            }
        }
        {
            let Ok(mut remote) = self.pending_remote.lock() else {
                return;
            };
            if remote.is_none() {
                *remote = Some(PendingAsk {
                    questions,
                    reply: tx,
                });
                drop(remote);
                self.spawn_responder(host, call_id, kind_approval, rx);
                return;
            }
        }
        log::warn!(
            target: "xylitol::tui",
            "ask slot busy; rejected remote request call_id={call_id} method={method}"
        );
        self.push_notice("ask slot busy — remote request rejected");
        if let Some(host) = host {
            let payload = if kind_approval {
                serde_json::json!({ "approved": false })
            } else {
                serde_json::json!({ "answer": "" })
            };
            tokio::spawn(async move {
                let _ = host.respond(&call_id, payload).await;
            });
        }
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
