//! Four-quadrant RPC envelope (product wire outer layer).
//!
//! Command/Event stay payload / frame content. Not JSON-RPC 2.0.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

/// Stable protocol version returned by `host.describe`.
pub const PROTOCOL_VERSION: u32 = 1;

/// Discriminated four-quadrant message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RpcMessage {
    ClientRequest {
        #[serde(rename = "rpcId")]
        rpc_id: String,
        method: String,
        #[serde(default)]
        #[specta(type = specta_typescript::Any)]
        payload: Value,
    },
    ServerResponse {
        #[serde(rename = "rpcId")]
        rpc_id: String,
        result: RpcResult,
    },
    ServerRequest {
        #[serde(rename = "rpcId")]
        rpc_id: String,
        method: String,
        #[serde(default)]
        #[specta(type = specta_typescript::Any)]
        payload: Value,
    },
    ClientResponse {
        #[serde(rename = "rpcId")]
        rpc_id: String,
        #[serde(default)]
        #[specta(type = specta_typescript::Any)]
        payload: Value,
    },
}

/// Unary / respond result. HTTP 200 means the envelope parsed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct RpcResult {
    pub ok: bool,
    #[serde(default)]
    #[specta(type = Option<specta_typescript::Any>)]
    pub value: Option<Value>,
    #[serde(default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RpcError {
    pub code: String,
    pub details: String,
}

/// Downlink payload for `session/event`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SessionEventPayload {
    pub session_id: String,
    #[specta(type = specta_typescript::Number)]
    pub seq: u64,
    pub event: crate::protocol::Event,
}

/// Downlink payload for `session/subscribed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SessionSubscribedPayload {
    pub session_id: String,
    #[specta(type = specta_typescript::Number)]
    pub seq: u64,
}

/// Downlink payload for `session/resync_required`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SessionResyncRequiredPayload {
    pub session_id: String,
}

/// Downlink payload for `host/hello`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HostHelloPayload {
    pub protocol: u32,
}

/// Downlink payload for `approval/requested` (stable `rpcId` on the envelope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ApprovalRequestedPayload {
    pub call_id: String,
}

/// Downlink payload for `question/requested` (stable `rpcId` on the envelope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct QuestionRequestedPayload {
    pub call_id: String,
}

/// `host.describe` result.value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HostDescribeValue {
    pub protocol: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_request_roundtrip() {
        let msg = RpcMessage::ClientRequest {
            rpc_id: "r1".into(),
            method: "prompt".into(),
            payload: serde_json::json!({"message": "hi"}),
        };
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["type"], "client-request");
        assert_eq!(v["rpcId"], "r1");
        let back: RpcMessage = serde_json::from_value(v).unwrap();
        assert_eq!(back, msg);
    }

    #[test]
    fn rpc_result_ok_and_err() {
        let ok = RpcResult::ok_value(serde_json::json!({"seq": 1}));
        let v = serde_json::to_value(&ok).unwrap();
        assert_eq!(v["ok"], true);
        let err = RpcResult::error("not_found", "no session");
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "not_found");
    }

    #[test]
    fn response_echoes_rpc_id() {
        let msg = RpcMessage::ServerResponse {
            rpc_id: "r1".into(),
            result: RpcResult::ok_value(Value::Null),
        };
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["type"], "server-response");
        assert_eq!(v["rpcId"], "r1");
    }
}
