use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use adk_core::{Content, Llm, LlmRequest, LlmResponse, LlmResponseStream, Part};
use async_trait::async_trait;

/// A single pre-configured response step for [`FauxProvider`].
#[derive(Clone)]
pub(crate) enum FauxResponseStep {
    Message(FauxMessage),
    Factory(Arc<dyn Fn(&LlmRequest, usize) -> FauxMessage + Send + Sync>),
}

impl FauxResponseStep {
    pub(crate) fn text(text: impl Into<String>) -> Self {
        Self::Message(FauxMessage::Text(text.into()))
    }

    pub(crate) fn thinking(thinking: impl Into<String>) -> Self {
        Self::Message(FauxMessage::Thinking(thinking.into()))
    }

    pub(crate) fn tool_call(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self::Message(FauxMessage::ToolCall {
            name: name.into(),
            args,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) enum FauxMessage {
    Text(String),
    Thinking(String),
    ToolCall {
        name: String,
        args: serde_json::Value,
    },
}

#[derive(Clone)]
pub(crate) struct FauxProvider {
    inner: Arc<FauxProviderInner>,
}

struct FauxProviderInner {
    name: String,
    steps: Mutex<VecDeque<FauxResponseStep>>,
    call_count: AtomicUsize,
    captured_requests: Mutex<Vec<LlmRequest>>,
}

impl FauxProvider {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(FauxProviderInner {
                name: name.into(),
                steps: Mutex::new(VecDeque::new()),
                call_count: AtomicUsize::new(0),
                captured_requests: Mutex::new(Vec::new()),
            }),
        }
    }

    pub(crate) fn call_count(&self) -> usize {
        self.inner.call_count.load(Ordering::Relaxed)
    }

    pub(crate) fn captured_requests(&self) -> Vec<LlmRequest> {
        self.inner
            .captured_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub(crate) fn set_responses(&self, steps: Vec<FauxResponseStep>) {
        let mut guard = self.inner.steps.lock().unwrap_or_else(|e| e.into_inner());
        *guard = steps.into_iter().collect();
    }

    pub(crate) fn append_responses(&self, steps: Vec<FauxResponseStep>) {
        let mut guard = self.inner.steps.lock().unwrap_or_else(|e| e.into_inner());
        for step in steps {
            guard.push_back(step);
        }
    }

    pub(crate) fn clone_box(&self) -> Arc<dyn Llm> {
        Arc::new(self.clone()) as Arc<dyn Llm>
    }
}

#[async_trait]
impl Llm for FauxProvider {
    fn name(&self) -> &str {
        &self.inner.name
    }

    async fn generate_content(
        &self,
        req: LlmRequest,
        stream: bool,
    ) -> adk_core::Result<LlmResponseStream> {
        self.inner.call_count.fetch_add(1, Ordering::Relaxed);
        self.inner
            .captured_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(req.clone());

        let step = self
            .inner
            .steps
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front();

        let Some(step) = step else {
            return Ok(Box::pin(futures::stream::empty()));
        };

        let msg = match step {
            FauxResponseStep::Message(msg) => msg,
            FauxResponseStep::Factory(f) => f(&req, self.call_count()),
        };

        match msg {
            FauxMessage::ToolCall { name, args } => {
                let content = Content {
                    role: "assistant".into(),
                    parts: vec![Part::FunctionCall {
                        name,
                        args,
                        id: None,
                        thought_signature: None,
                    }],
                };
                let resp = LlmResponse::new(content);
                return Ok(Box::pin(futures::stream::once(async move { Ok(resp) })));
            }
            FauxMessage::Text(text) => {
                return Ok(boxed_text_stream(text, stream, /*thinking*/ false));
            }
            FauxMessage::Thinking(thinking) => {
                return Ok(boxed_thinking_stream(thinking, stream));
            }
        }
    }
}

fn boxed_text_stream(text: String, stream: bool, thinking: bool) -> LlmResponseStream {
    if !stream {
        let mut content = Content::new("assistant");
        if thinking {
            content.parts.push(Part::Thinking {
                thinking: text,
                signature: None,
            });
        } else {
            content.parts.push(Part::Text { text });
        }
        let resp = LlmResponse::new(content);
        return Box::pin(futures::stream::once(async move { Ok(resp) }));
    }

    let chunks = chunk_text(&text, 4);
    let chunks_len = chunks.len();
    let mut out = Vec::new();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        let is_last = idx + 1 == chunks_len;
        let mut content = Content::new("assistant");
        if thinking {
            content.parts.push(Part::Thinking {
                thinking: chunk,
                signature: None,
            });
        } else {
            content.parts.push(Part::Text { text: chunk });
        }
        let mut resp = LlmResponse::new(content);
        resp.partial = !is_last;
        resp.turn_complete = is_last;
        out.push(Ok(resp));
    }

    Box::pin(futures::stream::iter(out))
}

fn boxed_thinking_stream(thinking: String, stream: bool) -> LlmResponseStream {
    boxed_text_stream(thinking, stream, /*thinking*/ true)
}

fn chunk_text(text: &str, chunk_size: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut out = Vec::new();
    let mut buf = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        buf.push(ch);
        count += 1;
        if count >= chunk_size {
            out.push(std::mem::take(&mut buf));
            count = 0;
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}
