//! Prompt-processing result type (spec c255 / as32).

/// Result of processing user input through the prompt interceptor.
#[derive(Debug, Clone)]
pub enum PromptResult {
    /// A slash command was matched and handled. No LLM call needed.
    Handled { command: String, args: String },
    /// A /template:name was expanded. The caller should send the content to the LLM.
    Expanded(String),
    /// A `!cmd` / `!!cmd` bash execution request. The caller should invoke
    /// `execute_bash` with the parsed command; `exclude_from_context` reflects
    /// the bang prefix.
    Bash {
        exclude_from_context: bool,
        command: String,
    },
    /// Normal input — pass through to LLM unchanged.
    PassThrough(String),
}
