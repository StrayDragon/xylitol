use serde::Deserialize as _;
use serde::de::Error as _;
use serde_json::{Value, json};

use super::envelope::{RpcError, RpcMessage, RpcResult};

pub fn encode(msg: &RpcMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(msg)
}

pub fn encode_to_string(msg: &RpcMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

pub const JSON_DEPTH_LIMIT: u32 = 2048;

pub fn json_nesting_exceeds_limit(text: &str) -> bool {
    let mut depth: u32 = 0;
    let mut in_string = false;
    let mut escape = false;
    for c in text.chars() {
        if in_string {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '[' | '{' => {
                depth = depth.saturating_add(1);
                if depth > JSON_DEPTH_LIMIT {
                    return true;
                }
            }
            ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

fn parse_json_value(text: &str) -> Result<Value, serde_json::Error> {
    if json_nesting_exceeds_limit(text) {
        return Err(serde_json::Error::custom("json depth limit exceeded"));
    }
    let mut de = serde_json::Deserializer::from_str(text);
    de.disable_recursion_limit();
    Value::deserialize(&mut de)
}

pub fn decode(bytes: &[u8]) -> Result<RpcMessage, serde_json::Error> {
    let text = std::str::from_utf8(bytes).map_err(|e| {
        serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })?;
    let v: Value = parse_json_value(text)?;
    decode_value(v)
}

pub fn decode_str(text: &str) -> Result<RpcMessage, serde_json::Error> {
    let v: Value = parse_json_value(text)?;
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

pub fn jsonrpc_response(id: &Value, result: &RpcResult) -> Value {
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

pub fn jsonrpc_method_not_found(id: &Value) -> Value {
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

pub fn jsonrpc_invalid_request(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32600,
            "message": "Invalid request",
            "data": { "code": "illegal_envelope" },
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
    use crate::RpcMessage;
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

    fn nested_arrays(depth: usize) -> String {
        format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
    }

    #[test]
    fn json_depth_probe_skips_brackets_inside_strings() {
        let text = "{\"s\":\"[[[[[\"}";
        assert!(
            !json_nesting_exceeds_limit(text),
            "字符串内括号 MUST NOT 计入嵌套"
        );
    }

    #[test]
    fn parse_json_accepts_session_tree_scale_depth() {
        let text = nested_arrays(200);
        parse_json_value(&text).expect("~200 层（实机深树量级）MUST 通过");
    }

    #[test]
    fn decode_rejects_beyond_json_depth_limit() {
        let too_deep = nested_arrays(JSON_DEPTH_LIMIT as usize + 1);
        assert!(
            json_nesting_exceeds_limit(&too_deep),
            "探测 MUST 在 >{JSON_DEPTH_LIMIT} 层早退"
        );
        assert!(
            parse_json_value(&too_deep).is_err(),
            "超限 MUST 拒解析（服务端非法信封路径）"
        );
        let envelope =
            format!(r#"{{"jsonrpc":"2.0","id":"1","method":"host.describe","params":{too_deep}}}"#);
        assert!(
            decode_str(&envelope).is_err(),
            "超限 envelope MUST decode 失败"
        );
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
