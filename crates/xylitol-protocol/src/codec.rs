use serde::Deserialize as _;
use serde_json::{Value, json};

use super::message::{
    BUSINESS_ERROR, ErrorObject, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, JSONRPC_VERSION,
    METHOD_NOT_FOUND, Message, Notification, Outcome, Request, RequestId, Response, RpcError,
    RpcResult,
};

pub const JSON_DEPTH_LIMIT: u32 = 2048;

pub const MAX_BATCH_ITEMS: usize = 100;

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

#[derive(Debug, Clone, PartialEq)]
pub struct Rejected {
    pub id: Option<RequestId>,
    pub error: ErrorObject,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inbound {
    Single(Result<Message, Rejected>),
    Batch(Vec<Result<Message, Rejected>>),
}

pub fn parse_bytes(bytes: &[u8]) -> Result<Inbound, Rejected> {
    let text = std::str::from_utf8(bytes).map_err(|_| Rejected {
        id: None,
        error: ErrorObject::parse_error("body is not valid UTF-8"),
    })?;
    parse_text(text)
}

pub fn parse_text(text: &str) -> Result<Inbound, Rejected> {
    if json_nesting_exceeds_limit(text) {
        return Err(Rejected {
            id: None,
            error: ErrorObject::parse_error("json nesting depth limit exceeded"),
        });
    }
    let mut de = serde_json::Deserializer::from_str(text);
    de.disable_recursion_limit();
    let value = Value::deserialize(&mut de).map_err(|e| Rejected {
        id: None,
        error: ErrorObject::parse_error(format!("parse error: {e}")),
    })?;
    match value {
        Value::Array(items) if items.is_empty() => Err(Rejected {
            id: None,
            error: ErrorObject::invalid_request("empty batch"),
        }),
        Value::Array(items) if items.len() > MAX_BATCH_ITEMS => Err(Rejected {
            id: None,
            error: ErrorObject::new(INVALID_REQUEST, "batch too large", "batch_too_large"),
        }),
        Value::Array(items) => Ok(Inbound::Batch(items.iter().map(decode_value).collect())),
        other => Ok(Inbound::Single(decode_value(&other))),
    }
}

pub fn decode_value(value: &Value) -> Result<Message, Rejected> {
    let Some(obj) = value.as_object() else {
        return Err(invalid(None, "JSON-RPC message must be an object"));
    };
    let raw_id = obj.get("id");
    let id_is_null = matches!(raw_id, Some(Value::Null));
    let id = match raw_id {
        None | Some(Value::Null) => None,
        Some(raw) => Some(
            RequestId::from_value(raw)
                .ok_or_else(|| invalid(None, "id must be a string or integer"))?,
        ),
    };
    if obj.get("jsonrpc").and_then(Value::as_str) != Some(JSONRPC_VERSION) {
        return Err(invalid(id, "jsonrpc must be \"2.0\""));
    }
    if let Some(method) = obj.get("method") {
        let Some(method) = method.as_str() else {
            return Err(invalid(id, "method must be a string"));
        };
        if id_is_null {
            return Err(invalid(None, "id must not be null"));
        }
        let params = match obj.get("params") {
            None => Value::Object(serde_json::Map::new()),
            Some(p @ (Value::Object(_) | Value::Array(_))) => p.clone(),
            Some(_) => return Err(invalid(id, "params must be an object or array")),
        };
        return Ok(match id {
            Some(id) => Message::Request(Request {
                id,
                method: method.to_string(),
                params,
            }),
            None => Message::Notification(Notification {
                method: method.to_string(),
                params,
            }),
        });
    }
    if let Some(error) = obj.get("error") {
        let error = decode_error_object(error)
            .ok_or_else(|| invalid(id.clone(), "error must carry an integer code"))?;
        return Ok(Message::Response(Response {
            id,
            outcome: Outcome::Error(error),
        }));
    }
    if let Some(result) = obj.get("result") {
        return Ok(Message::Response(Response {
            id,
            outcome: Outcome::Result(result.clone()),
        }));
    }
    Err(invalid(
        id,
        "JSON-RPC object is neither request, notification nor response",
    ))
}

pub fn decode_message(text: &str) -> Result<Message, Rejected> {
    match parse_text(text)? {
        Inbound::Single(one) => one,
        Inbound::Batch(_) => Err(invalid(None, "batch is not accepted here")),
    }
}

fn invalid(id: Option<RequestId>, message: &str) -> Rejected {
    Rejected {
        id,
        error: ErrorObject::invalid_request(message),
    }
}

fn decode_error_object(v: &Value) -> Option<ErrorObject> {
    let code = v.get("code")?.as_i64()?;
    let message = v
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let data_code = v
        .get("data")
        .and_then(|d| d.get("code"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(ErrorObject {
        code,
        message,
        data_code,
    })
}

pub fn business_error(err: &RpcError) -> ErrorObject {
    let code = match err.code.as_str() {
        "invalid_input" => INVALID_PARAMS,
        "internal_error" => INTERNAL_ERROR,
        "unregistered_method" => METHOD_NOT_FOUND,
        "invalid_request" => INVALID_REQUEST,
        _ => BUSINESS_ERROR,
    };
    ErrorObject {
        code,
        message: err.details.clone(),
        data_code: Some(err.code.clone()),
    }
}

pub fn outcome_of(result: &RpcResult) -> Outcome {
    if result.ok {
        return Outcome::Result(result.value.clone().unwrap_or_else(|| json!({})));
    }
    let err = result.error.clone().unwrap_or(RpcError {
        code: "internal_error".into(),
        details: "ok=false without error object".into(),
    });
    Outcome::Error(business_error(&err))
}

pub fn response_for(id: &RequestId, result: &RpcResult) -> Response {
    Response {
        id: Some(id.clone()),
        outcome: outcome_of(result),
    }
}

pub fn error_value(error: &ErrorObject) -> Value {
    let mut obj = json!({ "code": error.code, "message": error.message });
    if let Some(code) = &error.data_code {
        obj["data"] = json!({ "code": code });
    }
    obj
}

pub fn response_value(resp: &Response) -> Value {
    let id = resp.id.as_ref().map_or(Value::Null, RequestId::to_value);
    match &resp.outcome {
        Outcome::Result(result) => {
            json!({ "jsonrpc": JSONRPC_VERSION, "id": id, "result": result })
        }
        Outcome::Error(error) => {
            json!({ "jsonrpc": JSONRPC_VERSION, "id": id, "error": error_value(error) })
        }
    }
}

pub fn request_value(id: &RequestId, method: &str, params: &Value) -> Value {
    json!({ "jsonrpc": JSONRPC_VERSION, "id": id.to_value(), "method": method, "params": params })
}

pub fn notification_value(method: &str, params: &Value) -> Value {
    json!({ "jsonrpc": JSONRPC_VERSION, "method": method, "params": params })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, Outcome};

    fn nested_arrays(depth: usize) -> String {
        format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
    }

    #[test]
    fn depth_probe_skips_brackets_inside_strings() {
        assert!(!json_nesting_exceeds_limit("{\"s\":\"[[[[[\"}"));
    }

    #[test]
    fn depth_limit_rejects_beyond_2048_and_accepts_session_tree_scale() {
        let at_limit = nested_arrays(JSON_DEPTH_LIMIT as usize);
        assert!(!json_nesting_exceeds_limit(&at_limit));
        assert!(parse_text(&at_limit).is_ok(), "exactly 2048 levels parse");
        let too_deep = nested_arrays(JSON_DEPTH_LIMIT as usize + 1);
        assert!(json_nesting_exceeds_limit(&too_deep));
        let err = parse_text(&too_deep).expect_err("beyond limit");
        assert_eq!(err.error.code, crate::message::PARSE_ERROR);
        let deep_ok = nested_arrays(200);
        assert!(parse_text(&deep_ok).is_ok(), "200 levels must parse");
    }

    #[test]
    fn request_notification_response_decode() {
        let req = decode_message(r#"{"jsonrpc":"2.0","id":"r1","method":"prompt","params":{}}"#)
            .expect("request");
        assert!(
            matches!(req, Message::Request(ref r) if r.method == "prompt" && r.id.key() == "r1")
        );

        let note = decode_message(r#"{"jsonrpc":"2.0","method":"session/event","params":{}}"#)
            .expect("notification");
        assert!(matches!(note, Message::Notification(ref n) if n.method == "session/event"));

        let resp = decode_message(r#"{"jsonrpc":"2.0","id":1,"result":{"protocol":4}}"#)
            .expect("response");
        assert!(matches!(resp, Message::Response(ref r) if r.id == Some(RequestId::Int(1))));
    }

    #[test]
    fn null_id_response_decodes_and_null_id_request_is_rejected() {
        let resp =
            decode_message(r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"x"}}"#)
                .expect("parse-error response");
        assert!(matches!(resp, Message::Response(ref r) if r.id.is_none()));

        let bad = decode_message(r#"{"jsonrpc":"2.0","id":null,"method":"prompt"}"#)
            .expect_err("null id request");
        assert_eq!(bad.error.code, INVALID_REQUEST);
    }

    #[test]
    fn invalid_shapes_are_invalid_request_with_echoed_id() {
        let bad_version =
            decode_message(r#"{"jsonrpc":"1.0","id":"x","method":"prompt"}"#).expect_err("version");
        assert_eq!(bad_version.error.code, INVALID_REQUEST);
        assert_eq!(bad_version.id, Some(RequestId::Str("x".into())));

        let scalar_params =
            decode_message(r#"{"jsonrpc":"2.0","id":"x","method":"prompt","params":3}"#)
                .expect_err("scalar params");
        assert_eq!(scalar_params.error.code, INVALID_REQUEST);

        let float_id = decode_message(r#"{"jsonrpc":"2.0","id":1.5,"method":"prompt"}"#)
            .expect_err("float id");
        assert_eq!(float_id.error.code, INVALID_REQUEST);

        let non_string_method = decode_message(r#"{"jsonrpc":"2.0","id":"x","method":3}"#)
            .expect_err("non-string method");
        assert_eq!(non_string_method.error.code, INVALID_REQUEST);
        assert_eq!(non_string_method.id, Some(RequestId::Str("x".into())));
    }

    #[test]
    fn batch_keeps_order_and_empty_batch_is_invalid() {
        let inbound = parse_text(
            r#"[{"jsonrpc":"2.0","id":1,"method":"a"},{"jsonrpc":"2.0","method":"n"},{"jsonrpc":"2.0","id":2,"method":"b"}]"#,
        )
        .expect("batch");
        let Inbound::Batch(items) = inbound else {
            panic!("expected batch");
        };
        assert_eq!(items.len(), 3);
        assert!(matches!(items[1], Ok(Message::Notification(_))));

        let empty = parse_text("[]").expect_err("empty batch");
        assert_eq!(empty.error.code, INVALID_REQUEST);
    }

    #[test]
    fn batch_limit_accepts_100_and_rejects_101() {
        let batch_of = |n: usize| {
            let items: Vec<String> = (0..n)
                .map(|i| format!(r#"{{"jsonrpc":"2.0","id":{i},"method":"m"}}"#))
                .collect();
            format!("[{}]", items.join(","))
        };
        let ok = parse_text(&batch_of(MAX_BATCH_ITEMS)).expect("100 items accepted");
        assert!(matches!(ok, Inbound::Batch(ref items) if items.len() == MAX_BATCH_ITEMS));
        let err = parse_text(&batch_of(MAX_BATCH_ITEMS + 1)).expect_err("101 items rejected");
        assert_eq!(err.error.code, INVALID_REQUEST);
        assert_eq!(err.error.data_code.as_deref(), Some("batch_too_large"));
    }

    #[test]
    fn parse_error_and_non_utf8_are_parse_errors() {
        let err = parse_text("{").expect_err("truncated");
        assert_eq!(err.error.code, crate::message::PARSE_ERROR);
        let err = parse_bytes(&[0xff, 0xfe]).expect_err("utf8");
        assert_eq!(err.error.code, crate::message::PARSE_ERROR);
    }

    #[test]
    fn business_error_mapping_table() {
        let inv = business_error(&RpcError {
            code: "invalid_input".into(),
            details: "model_id".into(),
        });
        assert_eq!(inv.code, INVALID_PARAMS);
        assert_eq!(inv.data_code.as_deref(), Some("invalid_input"));

        let nf = business_error(&RpcError {
            code: "not_found".into(),
            details: "no session".into(),
        });
        assert_eq!(nf.code, BUSINESS_ERROR);
        assert_eq!(nf.data_code.as_deref(), Some("not_found"));

        let um = business_error(&RpcError {
            code: "unregistered_method".into(),
            details: "x".into(),
        });
        assert_eq!(um.code, METHOD_NOT_FOUND);
    }

    #[test]
    fn response_shapes_round_trip() {
        let ok = response_for(
            &RequestId::Str("1".into()),
            &RpcResult::ok_value(json!({"a": 1})),
        );
        let v = response_value(&ok);
        assert_eq!(v["id"], "1");
        assert_eq!(v["result"]["a"], 1);
        assert!(v.get("error").is_none());

        let err = response_for(&RequestId::Int(2), &RpcResult::error("not_found", "gone"));
        let v = response_value(&err);
        assert_eq!(v["error"]["code"], BUSINESS_ERROR);
        assert_eq!(v["error"]["data"]["code"], "not_found");
        assert!(matches!(err.outcome, Outcome::Error(_)));
    }
}
