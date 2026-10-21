use crate::error::XyToolError;

#[async_trait::async_trait]
pub trait AskUserGateway: Send + Sync {
    async fn prompt(&self, args: AskArgs) -> Result<String, XyToolError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AskOptionArg {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub recommended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AskModeArg {
    Single,
    Multi,
}

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AskArgs {
    pub questions: Vec<AskQuestionArg>,
}
