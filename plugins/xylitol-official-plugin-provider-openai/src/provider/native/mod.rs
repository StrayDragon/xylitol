pub mod assembler;
pub mod openai;
pub mod openai_client;
pub mod openai_completions;
pub mod openai_hooks_mw;
pub mod openai_responses;

pub use assembler::ResponsesAssembler;
pub use openai_completions::OpenAiCompletionsAdapter;
pub use openai_responses::{
    OpenAiResponsesAdapter, ResponsesStreamState, extract_embedded_provider_error_message,
    format_responses_error, map_responses_sse_event, messages_to_responses_input,
    messages_to_responses_input_with_diagnostics, messages_to_responses_input_with_options,
};

pub(crate) fn image_data_url(image: &crate::dto::ImageContent) -> String {
    image.url.clone().unwrap_or_else(|| {
        format!(
            "data:{};base64,{}",
            image.media_type,
            image.data.as_deref().unwrap_or("")
        )
    })
}

pub mod wait_bounds {
    use std::time::Duration;
    pub const CONNECT: Duration = Duration::from_secs(10);
    pub const SSE_IDLE: Duration = Duration::from_secs(90);
    pub const NON_STREAM_TOTAL: Duration = Duration::from_secs(300);
}
