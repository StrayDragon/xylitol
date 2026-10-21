use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RpcMessage {
    ClientRequest {
        #[serde(rename = "rpcId")]
        rpc_id: String,
        method: String,
        #[serde(default)]
        payload: Value,
        #[serde(default, rename = "writerToken")]
        writer_token: Option<String>,
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
        payload: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcResult {
    pub ok: bool,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub error: Option<RpcError>,
    #[serde(default, skip)]
    pub writer_token: Option<String>,
}

impl RpcResult {
    pub fn ok_value(value: Value) -> Self {
        Self {
            ok: true,
            value: Some(value),
            error: None,
            writer_token: None,
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
            writer_token: None,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEventPayload {
    pub session_id: String,
    pub seq: u64,
    pub event: crate::Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSubscribedPayload {
    pub session_id: String,
    pub seq: u64,
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
pub struct ApprovalRequestedPayload {
    pub call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionRequestedPayload {
    pub call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostDescribeValue {
    pub protocol: u32,
    #[serde(default)]
    pub formats: Vec<String>,
}

pub const WIRE_FORMAT_JSONRPC: &str = "jsonrpc";
pub const WIRE_FORMAT_FORY_V3: &str = "fory-v3";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_request_roundtrip() {
        let msg = RpcMessage::ClientRequest {
            rpc_id: "r1".into(),
            method: "prompt".into(),
            payload: serde_json::json!({"message": "hi"}),
            writer_token: None,
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
        let mut leased = RpcResult::ok_value(serde_json::json!({"seq": 1}));
        leased.writer_token = Some("tok".into());
        let v = serde_json::to_value(&leased).unwrap();
        assert!(v.get("writerToken").is_none(), "{v}");
        assert!(v.get("writer_token").is_none(), "{v}");
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
