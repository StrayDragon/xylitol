use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{EnumString, IntoStaticStr};

use super::message::{XyStopReason, XyUsage};
use crate::error::XyError;

#[derive(Debug, Clone)]
pub enum XyChunk {
    TextDelta(String),
    ThinkingDelta(String),
    ThinkingEnd {
        thinking: String,
        thinking_signature: Option<String>,
    },
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        name: String,
        args_delta: String,
        args: Value,
    },
    ToolCallEnd {
        id: String,
        name: String,
        args: Value,
    },
    Done {
        finish_reason: XyStopReason,
        usage: Option<XyUsage>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct XyToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub type XyStream = Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "PascalCase")]
pub enum TokenProvenance {
    Api,
    Heuristic,
    Unknown,
}

impl TokenProvenance {
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    pub fn from_key(s: &str) -> Self {
        s.parse().unwrap_or(Self::Unknown)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTokenEstimate {
    pub tokens: u64,
    pub provenance: TokenProvenance,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_key_is_pascal_case_strum_ssot() {
        assert_eq!(TokenProvenance::Api.as_str(), "Api");
        assert_eq!(TokenProvenance::Heuristic.as_str(), "Heuristic");
        assert_eq!(TokenProvenance::Unknown.as_str(), "Unknown");
        assert_eq!(TokenProvenance::from_key("Api"), TokenProvenance::Api);
        assert_eq!(
            TokenProvenance::from_key("Heuristic"),
            TokenProvenance::Heuristic
        );
        assert_eq!(
            TokenProvenance::from_key("LocalTokenizer"),
            TokenProvenance::Unknown
        );
        assert_eq!(
            TokenProvenance::from_key("RemoteCount"),
            TokenProvenance::Unknown
        );
        assert_eq!(
            serde_json::to_value(TokenProvenance::Api).unwrap(),
            serde_json::json!("api")
        );
    }
}
