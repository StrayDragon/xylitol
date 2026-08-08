//! Layer-1 **native** protocol implementations (first language).
//!
//! | Module | `api` |
//! |---|---|
//! | [`openai_responses`] | `openai-responses` |
//! | [`openai_completions`] / [`openai`] | `openai-completions` |
//! | [`anthropic_messages`] | `anthropic-messages` |
//!
//! Third-party quirks that *adjust* these natives live in [`super::dialect`].

pub mod anthropic_messages;
pub mod assembler;
pub mod openai;
pub mod openai_client;
pub mod openai_completions;
pub mod openai_hooks_mw;
pub mod openai_responses;

pub use anthropic_messages::AnthropicMessagesAdapter;
pub use assembler::ResponsesAssembler;
pub use openai_completions::OpenAiCompletionsAdapter;
pub use openai_responses::{
    OpenAiResponsesAdapter, ResponsesStreamState, extract_embedded_provider_error_message,
    format_responses_error, map_responses_sse_event, messages_to_responses_input,
    messages_to_responses_input_with_diagnostics, messages_to_responses_input_with_options,
};
