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
use crate::core::ports::ToolExecutionMode;

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
/// Constructed at the composition root (`interactive::cli`) which wires ports
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
/// Uses `reqwest` for control commands (prompt, abort) and polls events
/// via REST. A full WebSocket streaming path will be added in a follow-up.
pub struct RemoteDriver {
    base_url: String,
    session_id: String,
    client: reqwest::Client,
    cancel: CancellationToken,
}

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
        format!(
            "{}/api/v1/session/{}/run",
            self.base_url, self.session_id
        )
    }

    fn cancel_url(&self) -> String {
        format!("{}/api/v1/session/{}", self.base_url, self.session_id)
    }

    fn events_url(&self, seq: u64) -> String {
        format!(
            "{}/api/v1/session/{}/events?seq={}",
            self.base_url, self.session_id, seq
        )
    }

    fn base_events_url(&self) -> String {
        format!(
            "{}/api/v1/session/{}/events",
            self.base_url, self.session_id
        )
    }
}

#[async_trait]
impl Driver for RemoteDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let client = self.client.clone();
        let run_url = self.run_url();
        let cancel_url = self.cancel_url();
        let base_events_url = self.base_events_url();
        let cancel = self.cancel.clone();
        let prompt = prompt.to_string();

        let stream = async_stream::stream! {
            // Submit prompt
            let payload = serde_json::json!({"prompt": prompt});
            let resp = client
                .post(&run_url)
                .json(&payload)
                .send()
                .await;

            match resp {
                Ok(_) => {
                    // Poll events
                    let seq = 0u64;
                    loop {
                        if cancel.is_cancelled() {
                            let _ = client.delete(&cancel_url).send().await;
                            break;
                        }

                        let events_url = format!("{base_events_url}?seq={seq}");

                        match client.get(&events_url).send().await {
                            Ok(resp) => {
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    yield AgentEvent::TextDelta(
                                        format!("[remote] received {}", body)
                                    );
                                }
                                break;
                            }
                            Err(e) => {
                                yield AgentEvent::Error(format!("remote error: {e}"));
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    yield AgentEvent::Error(format!("connection failed: {e}"));
                }
            }
        };

        Box::pin(stream)
    }

    fn abort(&self) {
        self.cancel.cancel();
        // Fire-and-forget DELETE to cancel on server
        let url = self.cancel_url();
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.delete(&url).send().await;
        });
    }
}
