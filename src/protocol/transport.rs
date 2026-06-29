//! Transport helpers shared across JSONL, WebSocket, and REST surfaces.

use serde::{Deserialize, Serialize};

/// Error codes for REST envelope responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ErrorCode {
    #[default]
    Ok,
    BadRequest,
    NotFound,
    ServerLocked,
    SessionNotFound,
    InternalError,
    Timeout,
}

/// Uniform REST response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T: Serialize> {
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl<T: Serialize> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: ErrorCode::Ok,
            msg: None,
            data: Some(data),
            request_id: None,
        }
    }

    pub fn error(code: ErrorCode, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: Some(msg.into()),
            data: None,
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, id: Option<String>) -> Self {
        self.request_id = id;
        self
    }
}
