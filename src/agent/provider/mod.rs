//! LLM providers for xylitol.
//!
//! Direct integrations with LLM APIs:
//! - [`openai::OpenAIProvider`]: OpenAI Chat Completions via raw HTTP + SSE streaming
//! - [`anthropic::AnthropicProvider`]: Anthropic Messages API via raw HTTP + SSE streaming
//! - [`FakeProvider`] (dev-only): scenario-based mock for offline testing
//! - [`MockLlm`] (test-only): simple mock that returns a fixed response

pub(crate) mod anthropic;
pub(crate) mod openai;

#[cfg(feature = "dev-fake-provider")]
mod fake;
#[cfg(test)]
mod mock;

#[cfg(feature = "dev-fake-provider")]
pub(crate) use fake::*;
#[cfg(test)]
pub(crate) use mock::MockLlm;
