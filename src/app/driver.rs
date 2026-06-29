//! Driver abstraction — the single dependency of interactive layers.
//!
//! All interactive clients (cli/print/rpc) interact with the core through a
//! [`Driver`]; they never import `agent::facade` or `infra` directly.
//!
//! - [`InProcessDriver`]: wraps the local agent facade (composition root wires
//!   ports and agent together).
//! - [`RemoteDriver`]: speaks the protocol over REST/WS to a remote server.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use tokio_util::sync::CancellationToken;

use crate::agent::facade::{Agent, AgentEvent, AgentHooks};
use crate::runtime_protocol::ToolExecutionMode;

#[cfg(feature = "server")]
use crate::protocol::Event as ProtoEvent;
#[cfg(feature = "server")]
use futures::{SinkExt, StreamExt};
#[cfg(feature = "server")]
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[cfg(feature = "server")]
use crate::app::server::ws::{ClientFrame, ServerFrame};

/// A stream of [`AgentEvent`] items.
pub type EventStream = Pin<Box<dyn Stream<Item = AgentEvent> + Send>>;

/// Driver — interact with the core without knowing its internals.
///
/// [`InProcessDriver`] keeps a cached `Agent` and is the local (single-process)
/// implementation. A future `RemoteDriver` will speak the protocol over WS/REST.
#[async_trait]
pub trait Driver {
    /// Submit a prompt and receive a stream of events.
    async fn run(&mut self, prompt: &str) -> EventStream;

    /// Cancel the current turn.
    fn abort(&self);
}

/// In-process driver wrapping the local agent facade.
///
/// Constructed at the composition root (`app::cli`) which wires ports
/// and agent together. This is the **only** place in `interactive/` that
/// imports `agent::facade`.
pub struct InProcessDriver {
    agent: Agent,
}

impl InProcessDriver {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
    }

    pub fn with_hooks(mut self, hooks: AgentHooks) -> Self {
        self.agent = self.agent.with_hooks(hooks);
        self
    }

    pub fn with_tool_mode(mut self, mode: ToolExecutionMode) -> Self {
        self.agent = self.agent.with_tool_mode(mode);
        self
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent.cancel_token()
    }
}

#[async_trait]
impl Driver for InProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let stream = self.agent.run(prompt).await;
        Box::pin(stream)
    }

    fn abort(&self) {
        self.agent.abort();
    }
}

/// Remote driver — speaks protocol over REST/WS to a xylitol server.
///
/// Uses `reqwest` for control commands (prompt, abort) and `tokio-tungstenite`
/// for WebSocket event streaming.
#[cfg(feature = "server")]
pub struct RemoteDriver {
    base_url: String,
    session_id: String,
    client: reqwest::Client,
    cancel: CancellationToken,
}

#[cfg(feature = "server")]
impl RemoteDriver {
    /// Create a new RemoteDriver connected to `base_url`.
    ///
    /// `base_url` should be the server root, e.g. `http://127.0.0.1:8080`.
    pub fn new(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            session_id: session_id.into(),
            client: reqwest::Client::new(),
            cancel: CancellationToken::new(),
        }
    }

    fn run_url(&self) -> String {
        format!("{}/api/v1/session/{}/run", self.base_url, self.session_id)
    }

    fn cancel_url(&self) -> String {
        format!("{}/api/v1/session/{}", self.base_url, self.session_id)
    }

    fn ws_url(&self) -> String {
        // Convert http:// to ws://, https:// to wss://
        let ws_base = self
            .base_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!("{ws_base}/api/v1/session/{}/ws", self.session_id)
    }
}

#[cfg(feature = "server")]
#[async_trait]
impl Driver for RemoteDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let client = self.client.clone();
        let run_url = self.run_url();
        let cancel_url = self.cancel_url();
        let ws_url = self.ws_url();
        let cancel = self.cancel.clone();
        let prompt = prompt.to_string();

        let stream = async_stream::stream! {
            // 1. Submit prompt via REST (triggers agent execution)
            let payload = serde_json::json!({"prompt": prompt});
            match client.post(&run_url).json(&payload).send().await {
                Ok(resp) if !resp.status().is_success() => {
                    let status = resp.status();
                    yield AgentEvent::Error(format!("server returned {status}"));
                    return;
                }
                Err(e) => {
                    yield AgentEvent::Error(format!("connection failed: {e}"));
                    return;
                }
                _ => {} // success
            }

            // 2. Connect to WS for event streaming
            let ws_stream = match connect_async(&ws_url).await {
                Ok((ws, _)) => ws,
                Err(e) => {
                    yield AgentEvent::Error(format!("WS connect failed: {e}"));
                    return;
                }
            };

            let (mut ws_writer, mut ws_reader) = ws_stream.split();

            // 3. Send Subscribe
            let subscribe = serde_json::to_string(&ClientFrame::Subscribe {
                session_id: String::new(), // server knows from URL
                last_seq: 0,
            })
            .unwrap();
            if ws_writer.send(Message::Text(subscribe.into())).await.is_err() {
                yield AgentEvent::Error("WS send failed".into());
                return;
            }

            // 4. Read events until AgentEnd or abort
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = client.delete(&cancel_url).send().await;
                        break;
                    }
                    msg = ws_reader.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(frame) = serde_json::from_str::<ServerFrame>(&text) {
                                    match frame {
                                        ServerFrame::ServerHello { .. } | ServerFrame::Ack { .. } => {
                                            // Handshake frames, ignore
                                        }
                                        ServerFrame::Event { event, .. } => {
                                            if let Some(agent_event) = proto_to_agent(&event) {
                                                let is_end = matches!(agent_event, AgentEvent::AgentEnd { .. });
                                                yield agent_event;
                                                if is_end {
                                                    break;
                                                }
                                            }
                                        }
                                        ServerFrame::ResyncRequired { .. } => {
                                            yield AgentEvent::Error("journal truncated, resync required".into());
                                            break;
                                        }
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        };

        Box::pin(stream)
    }

    fn abort(&self) {
        self.cancel.cancel();
        let url = self.cancel_url();
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.delete(&url).send().await;
        });
    }
}

// ── Protocol event → AgentEvent conversion ────────────────────────

#[cfg(feature = "server")]
fn proto_to_agent(event: &ProtoEvent) -> Option<AgentEvent> {
    match event {
        ProtoEvent::TextDelta { text } => Some(AgentEvent::TextDelta(text.clone())),
        ProtoEvent::TurnStart { turn_index } => Some(AgentEvent::TurnStart {
            turn_index: *turn_index,
        }),
        ProtoEvent::TurnEnd { turn_index } => Some(AgentEvent::TurnEnd {
            turn_index: *turn_index,
        }),
        ProtoEvent::MessageStart { role } => Some(AgentEvent::MessageStart { role: role.clone() }),
        ProtoEvent::MessageEnd { role } => Some(AgentEvent::MessageEnd { role: role.clone() }),
        ProtoEvent::MessageUpdate { text, thinking } => Some(AgentEvent::MessageUpdate {
            text: text.clone(),
            thinking: thinking.clone(),
        }),
        ProtoEvent::ToolStart { id, name } => Some(AgentEvent::ToolExecutionStart {
            id: id.clone(),
            name: name.clone(),
            args: serde_json::Value::Null,
        }),
        ProtoEvent::ToolEnd { id, name, result } => Some(AgentEvent::ToolExecutionEnd {
            id: id.clone(),
            name: name.clone(),
            result: result.clone(),
        }),
        ProtoEvent::ToolExecutionUpdate { id, output } => Some(AgentEvent::ToolExecutionUpdate {
            id: id.clone(),
            output: output.clone(),
        }),
        ProtoEvent::ModelSelect { provider, model_id } => Some(AgentEvent::ModelSelect {
            provider: provider.clone(),
            model_id: model_id.clone(),
        }),
        ProtoEvent::CompactionStart { reason } => Some(AgentEvent::CompactionStart {
            reason: reason.clone(),
        }),
        ProtoEvent::CompactionEnd => Some(AgentEvent::CompactionEnd {
            result: None,
            aborted: false,
        }),
        ProtoEvent::AgentEnd => Some(AgentEvent::AgentEnd {
            messages: Vec::new(),
        }),
        ProtoEvent::Error { message, .. } => Some(AgentEvent::Error(message.clone())),
        _ => None,
    }
}
