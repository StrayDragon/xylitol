use std::sync::Arc;

use serde_json::{Value, json};

use crate::occupancy::host::{HostState, handle_unary};
use xylitol_protocol::codec;
use xylitol_protocol::message::{INVALID_PARAMS, METHOD_NOT_FOUND};
use xylitol_protocol::method::{ANSWER_QUESTION, APPROVE_TOOL, RESERVED_PLUGIN_PREFIX};
use xylitol_protocol::registry::{self, METHOD_HOST_DESCRIBE, METHOD_SET_MODEL};
use xylitol_protocol::{
    ErrorObject, HostDescribeValue, Outcome, PROTOCOL_VERSION, RequestId, Response,
    WIRE_FORMAT_JSONRPC,
};

enum Route {
    Describe,
    Product,
    Reverse { call_id: String },
}

pub async fn dispatch_request(
    host: &Arc<HostState>,
    id: &RequestId,
    method: &str,
    params: &Value,
    mux_id: Option<u64>,
) -> Response {
    match route(method, params) {
        Err(error) => respond_error(id, error),
        Ok(Route::Describe) => respond_value(id, describe_value()),
        Ok(Route::Reverse { call_id }) => {
            host.respond(&call_id, params.clone()).await;
            respond_value(id, json!({}))
        }
        Ok(Route::Product) => {
            let key = id.key();
            let result = handle_unary(host, Some(&key), method, params.clone(), mux_id).await;
            codec::response_for(id, &result)
        }
    }
}

pub fn describe_value() -> Value {
    serde_json::to_value(HostDescribeValue {
        protocol: PROTOCOL_VERSION,
        formats: vec![WIRE_FORMAT_JSONRPC.into()],
    })
    .unwrap_or(Value::Null)
}

fn route(method: &str, params: &Value) -> Result<Route, ErrorObject> {
    let reverse = matches!(method, APPROVE_TOOL | ANSWER_QUESTION);
    if method.starts_with(RESERVED_PLUGIN_PREFIX)
        || (!reverse && registry::lookup(method).is_none())
    {
        return Err(unregistered_method());
    }
    if !params.is_object() {
        return Err(invalid_input("params must be an object"));
    }
    if reverse {
        return reverse_route(method, params);
    }
    if method == METHOD_HOST_DESCRIBE {
        return Ok(Route::Describe);
    }
    if method == METHOD_SET_MODEL && params.get("model_id").and_then(Value::as_str).is_none() {
        return Err(invalid_input("model_id must be a string"));
    }
    Ok(Route::Product)
}

fn reverse_route(method: &str, params: &Value) -> Result<Route, ErrorObject> {
    let call_id = params
        .get("call_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| invalid_input("call_id must be a non-empty string"))?;
    let shaped = if method == APPROVE_TOOL {
        params.get("approved").is_some_and(Value::is_boolean)
    } else {
        params.get("answer").is_some_and(Value::is_string)
    };
    if !shaped {
        let field = if method == APPROVE_TOOL {
            "approved must be a boolean"
        } else {
            "answer must be a string"
        };
        return Err(invalid_input(field));
    }
    Ok(Route::Reverse {
        call_id: call_id.to_string(),
    })
}

fn unregistered_method() -> ErrorObject {
    ErrorObject::new(METHOD_NOT_FOUND, "Method not found", "unregistered_method")
}

fn invalid_input(message: &str) -> ErrorObject {
    ErrorObject::new(INVALID_PARAMS, message, "invalid_input")
}

fn respond_value(id: &RequestId, value: Value) -> Response {
    Response {
        id: Some(id.clone()),
        outcome: Outcome::Result(value),
    }
}

fn respond_error(id: &RequestId, error: ErrorObject) -> Response {
    Response {
        id: Some(id.clone()),
        outcome: Outcome::Error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route_error(method: &str, params: Value) -> ErrorObject {
        match route(method, &params) {
            Err(error) => error,
            Ok(_) => panic!("expected a routing error for {method}"),
        }
    }

    fn is_reverse(method: &str, params: Value, call_id: &str) -> bool {
        matches!(route(method, &params), Ok(Route::Reverse { call_id: ref got }) if got == call_id)
    }

    #[test]
    fn unregistered_method_is_method_not_found() {
        let error = route_error("no_such_method", json!({}));
        assert_eq!(error.code, -32601);
        assert_eq!(error.data_code.as_deref(), Some("unregistered_method"));
        assert_eq!(error.message, "Method not found");
    }

    #[test]
    fn quit_and_plugin_namespace_are_not_wire_methods() {
        for method in ["quit", "plugin/anything", "plugin/"] {
            let error = route_error(method, json!({}));
            assert_eq!(error.code, -32601, "{method}");
            assert_eq!(error.data_code.as_deref(), Some("unregistered_method"));
        }
    }

    #[test]
    fn array_params_are_invalid_params_for_every_routed_method() {
        for method in ["prompt", "host.describe", APPROVE_TOOL, ANSWER_QUESTION] {
            let error = route_error(method, json!([]));
            assert_eq!(error.code, -32602, "{method}");
            assert_eq!(error.data_code.as_deref(), Some("invalid_input"));
        }
    }

    #[test]
    fn unregistered_wins_over_array_params() {
        let error = route_error("no_such_method", json!([]));
        assert_eq!(error.code, -32601);
    }

    #[test]
    fn registry_methods_route_to_the_domain() {
        assert!(matches!(
            route("prompt", &json!({"message": "hi"})),
            Ok(Route::Product)
        ));
        assert!(matches!(
            route("subscribe", &json!({"session_id": "s0", "last_seq": 0})),
            Ok(Route::Product)
        ));
        assert!(matches!(
            route("host.describe", &json!({})),
            Ok(Route::Describe)
        ));
    }

    #[test]
    fn reverse_methods_are_not_registry_entries_but_route_by_call_id() {
        assert!(registry::lookup(APPROVE_TOOL).is_none());
        assert!(registry::lookup(ANSWER_QUESTION).is_none());
        assert!(is_reverse(
            APPROVE_TOOL,
            json!({"call_id": "call-1", "approved": true}),
            "call-1"
        ));
        assert!(is_reverse(
            ANSWER_QUESTION,
            json!({"call_id": "call-2", "answer": "42"}),
            "call-2"
        ));
    }

    #[test]
    fn reverse_methods_require_a_call_id_and_their_payload_shape() {
        for params in [
            json!({"approved": true}),
            json!({"call_id": "", "approved": true}),
            json!({"call_id": 9, "approved": true}),
            json!({"call_id": "c", "approved": "yes"}),
        ] {
            let error = route_error(APPROVE_TOOL, params.clone());
            assert_eq!(error.code, -32602, "{params}");
            assert_eq!(error.data_code.as_deref(), Some("invalid_input"));
        }
        for params in [
            json!({"call_id": "c"}),
            json!({"call_id": "c", "answer": 3}),
            json!({"answer": "x"}),
        ] {
            let error = route_error(ANSWER_QUESTION, params.clone());
            assert_eq!(error.code, -32602, "{params}");
            assert_eq!(error.data_code.as_deref(), Some("invalid_input"));
        }
    }

    #[test]
    fn set_model_without_model_id_is_invalid_input() {
        let error = route_error("set_model", json!({}));
        assert_eq!(error.code, -32602);
        assert_eq!(error.data_code.as_deref(), Some("invalid_input"));
        assert!(matches!(
            route("set_model", &json!({"model_id": "m1"})),
            Ok(Route::Product)
        ));
    }

    #[test]
    fn unregistered_error_serializes_as_jsonrpc_method_not_found() {
        let resp = respond_error(&RequestId::Str("q".into()), route_error("nope", json!({})));
        let v = codec::response_value(&resp);
        assert_eq!(v["id"], "q");
        assert_eq!(v["error"]["code"], -32601);
        assert_eq!(v["error"]["data"]["code"], "unregistered_method");
    }

    #[test]
    fn reverse_answer_is_an_empty_object_result_with_the_request_id() {
        let resp = respond_value(&RequestId::Int(3), json!({}));
        let v = codec::response_value(&resp);
        assert_eq!(v["id"], 3);
        assert_eq!(v["result"], json!({}));
        assert!(v.get("error").is_none());
    }

    #[test]
    fn describe_names_jsonrpc_and_the_protocol_version() {
        let v = describe_value();
        assert_eq!(v["protocol"], PROTOCOL_VERSION);
        assert_eq!(v["formats"], json!(["jsonrpc"]));
    }
}
