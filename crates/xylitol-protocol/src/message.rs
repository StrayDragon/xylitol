use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Event;

pub const PROTOCOL_VERSION: u32 = 5;

pub const WIRE_FORMAT_JSONRPC: &str = "jsonrpc";

pub const JSONRPC_VERSION: &str = "2.0";

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;
pub const BUSINESS_ERROR: i64 = -32000;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestId {
    Str(String),
    Int(i64),
}

impl RequestId {
    pub fn key(&self) -> String {
        match self {
            Self::Str(s) => s.clone(),
            Self::Int(n) => n.to_string(),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Str(s) => Value::String(s.clone()),
            Self::Int(n) => Value::from(*n),
        }
    }

    pub(crate) fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::String(s) => Some(Self::Str(s.clone())),
            Value::Number(n) => n.as_i64().map(Self::Int),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub id: Option<RequestId>,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Result(Value),
    Error(ErrorObject),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorObject {
    pub code: i64,
    pub message: String,
    pub data_code: Option<String>,
}

impl ErrorObject {
    pub fn new(code: i64, message: impl Into<String>, data_code: &str) -> Self {
        Self {
            code,
            message: message.into(),
            data_code: Some(data_code.to_string()),
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(PARSE_ERROR, message, "parse_error")
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(INVALID_REQUEST, message, "invalid_request")
    }

    pub fn payload_too_large() -> Self {
        Self::new(INVALID_REQUEST, "payload too large", "payload_too_large")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request(Request),
    Notification(Notification),
    Response(Response),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpcResult {
    pub ok: bool,
    pub value: Option<Value>,
    pub error: Option<RpcError>,
}

impl RpcResult {
    pub fn ok_value(value: Value) -> Self {
        Self {
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    pub fn error(code: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            ok: false,
            value: None,
            error: Some(RpcError {
                code: code.into(),
                details: details.into(),
            }),
        }
    }

    pub fn into_std(self) -> Result<Value, RpcError> {
        if self.ok {
            Ok(self.value.unwrap_or(Value::Null))
        } else {
            Err(self.error.unwrap_or(RpcError {
                code: "internal_error".into(),
                details: "ok=false without error object".into(),
            }))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEventPayload {
    pub session_id: String,
    pub seq: u64,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSubscribedPayload {
    pub session_id: String,
    pub seq: u64,
    #[serde(default)]
    pub inflight: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionResyncRequiredPayload {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionResourcesPayload {
    pub session_id: String,
    pub snapshot: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionModelPayload {
    pub session_id: String,
    pub provider: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashOutputPayload {
    pub session_id: String,
    pub bash_id: String,
    pub seq: u64,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestedPayload {
    pub call_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionRequestedPayload {
    pub call_id: String,
    #[serde(default)]
    pub questions: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostDescribeValue {
    pub protocol: u32,
    #[serde(default)]
    pub formats: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_id_key_is_string_form() {
        assert_eq!(RequestId::Str("r1".into()).key(), "r1");
        assert_eq!(RequestId::Int(7).key(), "7");
    }

    #[test]
    fn rpc_result_ok_and_err_shapes() {
        let ok = RpcResult::ok_value(serde_json::json!({"seq": 1}));
        assert!(ok.ok);
        let err = RpcResult::error("not_found", "no session");
        assert_eq!(
            err.error.as_ref().map(|e| e.code.as_str()),
            Some("not_found")
        );
        assert!(err.into_std().is_err());
    }

    #[test]
    fn protocol_version_is_four() {
        assert_eq!(PROTOCOL_VERSION, 5);
        assert_eq!(WIRE_FORMAT_JSONRPC, "jsonrpc");
    }
}
