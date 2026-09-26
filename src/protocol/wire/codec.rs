//! Product wire bytes for the RPC envelope.
//!
//! Decode accepts JSON-RPC 2.0 objects (`"jsonrpc":"2.0"`) only. Product
//! encode helpers are `jsonrpc_request` / `jsonrpc_response` /
//! `jsonrpc_notification`. Internal [`RpcMessage`] serde (`type` tags) is not
//! a wire dialect.

use serde::de::Error as _;
use serde_json::{Value, json};

use super::envelope::{RpcError, RpcMessage, RpcResult};

pub fn encode(msg: &RpcMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(msg)
}

pub fn encode_to_string(msg: &RpcMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

pub fn decode(bytes: &[u8]) -> Result<RpcMessage, serde_json::Error> {
    let v: Value = serde_json::from_slice(bytes)?;
    decode_value(v)
}

pub fn decode_str(text: &str) -> Result<RpcMessage, serde_json::Error> {
    let v: Value = serde_json::from_str(text)?;
    decode_value(v)
}

fn decode_value(v: Value) -> Result<RpcMessage, serde_json::Error> {
    if v.get("jsonrpc").and_then(Value::as_str) == Some("2.0") {
        jsonrpc_to_message(&v)
    } else {
        Err(serde_json::Error::custom(
            "expected JSON-RPC 2.0 object (\"jsonrpc\":\"2.0\")",
        ))
    }
}

fn jsonrpc_id(v: &Value) -> Option<String> {
    match v.get("id")? {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn jsonrpc_to_message(v: &Value) -> Result<RpcMessage, serde_json::Error> {
    let method = v.get("method").and_then(Value::as_str);
    let id = jsonrpc_id(v);
    if let Some(method) = method {
        let params = v.get("params").cloned().unwrap_or_else(|| json!({}));
        if let Some(rpc_id) = id {
            return Ok(RpcMessage::ClientRequest {
                rpc_id,
                method: method.to_string(),
                payload: params,
                writer_token: None,
            });
        }
        return Ok(RpcMessage::ServerRequest {
            rpc_id: String::new(),
            method: method.to_string(),
            payload: params,
        });
    }
    let rpc_id = id.ok_or_else(|| serde_json::Error::custom("JSON-RPC result/error missing id"))?;
    if let Some(result) = v.get("result") {
        return Ok(RpcMessage::ServerResponse {
            rpc_id,
            result: RpcResult::ok_value(result.clone()),
        });
    }
    if let Some(error) = v.get("error") {
        let details = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let code = error
            .get("data")
            .and_then(|d| d.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("rpc_error")
            .to_string();
        return Ok(RpcMessage::ServerResponse {
            rpc_id,
            result: RpcResult {
                ok: false,
                value: None,
                error: Some(RpcError { code, details }),
                writer_token: None,
            },
        });
    }
    Err(serde_json::Error::custom(
        "JSON-RPC object is neither request, result, nor error",
    ))
}

/// JSON-RPC 2.0 success or application error. Envelope numeric codes are
/// carriers; product codes live in `error.data.code`.
pub fn jsonrpc_response(id: &str, result: &RpcResult) -> Value {
    if result.ok {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result.value.clone().unwrap_or(json!({})),
        })
    } else {
        let err = result.error.as_ref();
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": err.map(|e| e.details.as_str()).unwrap_or(""),
                "data": { "code": err.map(|e| e.code.as_str()).unwrap_or("rpc_error") },
            },
        })
    }
}

pub fn jsonrpc_method_not_found(id: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32601,
            "message": "Method not found",
            "data": { "code": "unregistered_method" },
        },
    })
}

pub fn jsonrpc_request(id: &str, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
}

pub fn jsonrpc_notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::RpcMessage;
    use serde_json::json;

    #[test]
    fn four_quadrant_type_tag_is_not_decoded() {
        let msg = RpcMessage::ClientRequest {
            rpc_id: "r1".into(),
            method: "prompt".into(),
            payload: json!({"message": "hi"}),
            writer_token: None,
        };
        let bytes = encode(&msg).expect("encode");
        let text = std::str::from_utf8(&bytes).expect("utf8");
        assert!(text.contains("\"type\":\"client-request\""), "{text}");
        assert!(
            decode(&bytes).is_err(),
            "four-quadrant type tag is not wire"
        );
        assert!(decode_str(text).is_err());
    }

    #[test]
    fn jsonrpc_request_result_notification_decode() {
        let req = decode_str(r#"{"jsonrpc":"2.0","id":"r1","method":"prompt","params":{}}"#)
            .expect("request");
        assert!(matches!(
            req,
            RpcMessage::ClientRequest { ref rpc_id, ref method, .. }
                if rpc_id == "r1" && method == "prompt"
        ));
        let result = decode_str(r#"{"jsonrpc":"2.0","id":"r1","result":{}}"#).expect("result");
        assert!(matches!(
            result,
            RpcMessage::ServerResponse { ref rpc_id, .. } if rpc_id == "r1"
        ));
        let note = decode_str(r#"{"jsonrpc":"2.0","method":"session/event","params":{}}"#)
            .expect("notification");
        assert!(matches!(
            note,
            RpcMessage::ServerRequest { ref method, ref rpc_id, .. }
                if method == "session/event" && rpc_id.is_empty()
        ));
    }
}
