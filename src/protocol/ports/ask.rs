//! Ask gateway port — TUI-only builtin `ask` tool ↔ application face callback.
//!
//! Cross-face callback contract: `infra` implements the `ask` tool that calls
//! the gateway; application faces (TUI host, CLI) implement the gateway. The
//! contract lives here so faces never reach `infra` internals to provide the
//! callback.

use crate::protocol::error::XyToolError;

/// Application-face callback: structured clarify / decision questionnaire.
#[async_trait::async_trait]
pub trait AskUserGateway: Send + Sync {
    async fn prompt(&self, args: AskArgs) -> Result<String, XyToolError>;
}

/// One option inside an ask question.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AskOptionArg {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub recommended: bool,
}

/// Selection mode for one question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AskModeArg {
    Single,
    Multi,
}

/// One question in an `ask` call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AskQuestionArg {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub label: Option<String>,
    pub mode: AskModeArg,
    pub options: Vec<AskOptionArg>,
    #[serde(default = "default_allow_other")]
    pub allow_other: bool,
}

fn default_allow_other() -> bool {
    true
}

/// Typed args for the `ask` tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AskArgs {
    pub questions: Vec<AskQuestionArg>,
}
