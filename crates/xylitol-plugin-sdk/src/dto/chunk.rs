use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{EnumString, IntoStaticStr};

use super::message::{PromptCacheRead, XyStopReason, XyUsage};
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, IntoStaticStr, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "PascalCase")]
pub enum TokenProvenance {
    Api,
    Heuristic,
    #[default]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextTokenEstimate {
    pub tokens: u64,
    pub provenance: TokenProvenance,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
    #[serde(default)]
    pub prompt_cache_read: PromptCacheRead,
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

pub struct StreamOutcome {
    pub chunks: Vec<XyChunk>,
    pub terminal: bool,
}

impl StreamOutcome {
    pub fn new(chunks: Vec<XyChunk>) -> Self {
        let terminal = matches!(chunks.last(), Some(XyChunk::Done { .. }));
        Self { chunks, terminal }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.terminal {
            return Err("stream ended without terminal Done".into());
        }
        chunk_stream_check(&self.chunks)
    }
}

pub fn chunk_stream_check(chunks: &[XyChunk]) -> Result<(), String> {
    let mut done = false;
    let mut open: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for chunk in chunks {
        if done {
            return Err("chunk emitted after terminal Done".into());
        }
        match chunk {
            XyChunk::TextDelta(_) | XyChunk::ThinkingDelta(_) | XyChunk::ThinkingEnd { .. } => {}
            XyChunk::ToolCallStart { id, name } => {
                if open.insert(id.as_str(), name.as_str()).is_some() {
                    return Err(format!("duplicate ToolCallStart for {id}"));
                }
            }
            XyChunk::ToolCallDelta { id, .. } => {
                if !open.contains_key(id.as_str()) {
                    return Err(format!("ToolCallDelta without ToolCallStart for {id}"));
                }
            }
            XyChunk::ToolCallEnd { id, .. } => {
                if open.remove(id.as_str()).is_none() {
                    return Err(format!("ToolCallEnd without ToolCallStart for {id}"));
                }
            }
            XyChunk::Done { .. } => {
                if !open.is_empty() {
                    return Err("Done emitted with open tool calls".into());
                }
                done = true;
            }
        }
    }
    if !done {
        return Err("stream ended without terminal Done".into());
    }
    Ok(())
}

#[cfg(test)]
mod chunk_stream_tests {
    use super::*;

    fn done() -> XyChunk {
        XyChunk::Done {
            finish_reason: XyStopReason::Stop,
            usage: None,
        }
    }

    fn start(id: &str) -> XyChunk {
        XyChunk::ToolCallStart {
            id: id.into(),
            name: "bash".into(),
        }
    }

    fn delta(id: &str) -> XyChunk {
        XyChunk::ToolCallDelta {
            id: id.into(),
            name: "bash".into(),
            args_delta: "{}".into(),
            args: Value::Object(Default::default()),
        }
    }

    fn end(id: &str) -> XyChunk {
        XyChunk::ToolCallEnd {
            id: id.into(),
            name: "bash".into(),
            args: Value::Object(Default::default()),
        }
    }

    #[test]
    fn accepts_text_then_done() {
        assert!(chunk_stream_check(&[XyChunk::TextDelta("hi".into()), done()]).is_ok());
    }

    #[test]
    fn accepts_tool_lifecycle() {
        assert!(chunk_stream_check(&[start("a"), delta("a"), end("a"), done()]).is_ok());
    }

    #[test]
    fn rejects_open_tool_call() {
        assert!(chunk_stream_check(&[start("a"), done()]).is_err());
    }

    #[test]
    fn rejects_duplicate_done() {
        assert!(chunk_stream_check(&[done(), done()]).is_err());
    }

    #[test]
    fn rejects_chunk_after_done() {
        assert!(chunk_stream_check(&[done(), XyChunk::TextDelta("late".into())]).is_err());
    }

    #[test]
    fn rejects_end_without_start() {
        assert!(chunk_stream_check(&[end("a"), done()]).is_err());
    }

    #[test]
    fn rejects_delta_without_start() {
        assert!(chunk_stream_check(&[delta("a"), done()]).is_err());
    }

    #[test]
    fn rejects_missing_done() {
        assert!(chunk_stream_check(&[XyChunk::TextDelta("hi".into())]).is_err());
    }
}
