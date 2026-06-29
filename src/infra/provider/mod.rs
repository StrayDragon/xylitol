//! LLM providers for xylitol.
//!
//! Direct integrations with LLM APIs live under [`adapter`]: each vendor API
//! dialect is an [`adapter::LlmAdapter`] that emits the internal [`XyChunk`]
//! stream. The top-level [`factory`] wraps an adapter in an
//! [`adapter::AdapterXyModel`] so the agent only sees `Arc<dyn XyModel>`.
//! - [`adapter::OpenAiResponsesAdapter`]: OpenAI Responses API
//! - [`adapter::AnthropicMessagesAdapter`]: Anthropic Messages API
//! - [`adapter::OpenAiCompletionsAdapter`]: OpenAI Chat Completions (via async-openai)
//! - [`FakeProvider`] (dev-only): scenario-based mock for offline testing
//! - [`MockXyModel`] (test-only): returns a fixed text response

pub mod adapter;
pub mod factory;
pub(crate) mod openai;

mod fake;
#[cfg(test)]
mod mock;

pub use fake::*;
#[cfg(test)]
pub use mock::MockXyModel;
