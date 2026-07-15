//! LLM providers for xylitol.
//!
//! Direct integrations with LLM APIs live under [`adapter`]: each vendor API
//! dialect is an [`crate::infra::provider::adapter::LlmAdapter`] that emits the internal [`crate::domain::types::XyChunk`]
//! stream. The top-level [`factory`] wraps an adapter in an
//! [`adapter::AdapterXyModel`] so the agent only sees `Arc<dyn XyModel>`.
//! - [`adapter::OpenAiResponsesAdapter`]: OpenAI Responses API
//! - [`adapter::AnthropicMessagesAdapter`]: Anthropic Messages API
//! - [`adapter::OpenAiCompletionsAdapter`]: OpenAI Chat Completions (reqwest + hook seams)
//! - [`FakeProvider`] (dev-only): scenario-based mock for offline testing
//! - `MockXyModel` (test-only): returns a fixed text response
//!
//! HTTP client types stay inside adapters / `reqwest_bridge`; script hooks see
//! only portable header bags (`infra::hooks::http`).

pub mod adapter;
pub mod factory;
pub(crate) mod openai;
pub(crate) mod reqwest_bridge;
pub(crate) mod trace;

mod fake;
#[cfg(test)]
mod mock;

pub use fake::*;
#[cfg(test)]
pub use mock::MockXyModel;
