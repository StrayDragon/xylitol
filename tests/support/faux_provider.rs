use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::agent::error::XyError;
use crate::agent::traits::{XyModel, XyStream};
use crate::agent::types::{XyChunk, XyContent, XyFinishReason, XyToolSchema};

type FauxFactory = Arc<dyn Fn(&[XyContent], usize) -> FauxMessage + Send + Sync>;

#[derive(Clone)]
pub(crate) enum FauxResponseStep {
    Message(FauxMessage),
    Factory(FauxFactory),
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
    captured_messages: Mutex<Vec<Vec<XyContent>>>,
}

impl FauxProvider {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(FauxProviderInner {
                name: name.into(),
                steps: Mutex::new(VecDeque::new()),
                call_count: AtomicUsize::new(0),
                captured_messages: Mutex::new(Vec::new()),
            }),
        }
    }

    pub(crate) fn call_count(&self) -> usize {
        self.inner.call_count.load(Ordering::Relaxed)
    }

    pub(crate) fn captured_messages(&self) -> Vec<Vec<XyContent>> {
        self.inner
            .captured_messages
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

    pub(crate) fn clone_box(&self) -> Arc<dyn XyModel> {
        Arc::new(self.clone()) as Arc<dyn XyModel>
    }

    pub(crate) fn captured_requests(&self) -> Vec<Vec<XyContent>> {
        self.captured_messages()
    }
}

#[async_trait]
impl XyModel for FauxProvider {
    fn name(&self) -> &str {
        &self.inner.name
    }

    async fn generate_stream(
        &self,
        messages: Vec<XyContent>,
        _tools: &[XyToolSchema],
        _stream: bool,
    ) -> Result<XyStream, XyError> {
        self.inner.call_count.fetch_add(1, Ordering::Relaxed);
        self.inner
            .captured_messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(messages.clone());

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
            FauxResponseStep::Factory(f) => f(&messages, self.call_count()),
        };

        match msg {
            FauxMessage::ToolCall { name, args } => {
                let id = format!("faux-call-{name}");
                Ok(Box::pin(futures::stream::iter(vec![
                    Ok(XyChunk::FunctionCall { name, args, id }),
                    Ok(XyChunk::Done {
                        finish_reason: XyFinishReason::Stop,
                    }),
                ])))
            }
            FauxMessage::Text(text) => Ok(Box::pin(futures::stream::iter(vec![
                Ok(XyChunk::TextDelta(text)),
                Ok(XyChunk::Done {
                    finish_reason: XyFinishReason::Stop,
                }),
            ]))),
            FauxMessage::Thinking(thinking) => Ok(Box::pin(futures::stream::iter(vec![
                Ok(XyChunk::ThinkingDelta(thinking)),
                Ok(XyChunk::Done {
                    finish_reason: XyFinishReason::Stop,
                }),
            ]))),
        }
    }
}
