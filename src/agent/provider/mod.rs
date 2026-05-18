//! Fake LLM provider for offline testing and development.
//!
//! Provides [`FakeProvider`] which implements [`adk_core::Llm`] with
//! scenario-based orchestration, delay simulation, and error injection.
//! Useful for testing agent loop, tool system, and mode flows without
//! a real LLM API.
//!
//! ## Feature gate
//!
//! Only available behind the `dev-fake-provider` feature flag.

mod fake;

pub(crate) use fake::*;
