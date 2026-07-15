use std::pin::Pin;

use futures::Stream;
use serde::Serialize;
use serde_json::Value;

use super::message::{AiBridgeStopReason, AiBridgeUsage};
use crate::error::AiBridgeError;

#[derive(Debug, Clone)]
pub enum AiBridgeChunk {
    TextDelta(String),
    ThinkingDelta(String),
    FunctionCall {
        name: String,
        args: Value,
        id: String,
    },
    Done {
        finish_reason: AiBridgeStopReason,
        usage: Option<AiBridgeUsage>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct AiBridgeToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub type AiBridgeStream = Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProvenance {
    Api,
    RemoteCount,
    LocalTokenizer,
    Heuristic,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ContextTokenEstimate {
    pub tokens: u64,
    pub provenance: TokenProvenance,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}
