use std::sync::Arc;

use serde_json::{Value, json};

use crate::occupancy::host::{HostState, handle_unary};
use xylitol_protocol::codec;
use xylitol_protocol::envelope::RpcResult;
use xylitol_protocol::registry;

const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

const REVERSE_METHODS: [&str; 2] = ["approve_tool", "answer_question"];

fn id_key(id: &Value) -> Option<String> {
    match id {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub async fn dispatch_raw(host: &Arc<HostState>, request: &str) -> Option<(Value, Option<String>)> {
    if request.len() > MAX_BODY_BYTES {
        return None;
    }
    let v: Value = serde_json::from_str(request).ok()?;
    if v.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return None;
    }
    let id = v.get("id").cloned().unwrap_or(Value::Null);
    let method = v.get("method").and_then(Value::as_str)?;
    let payload = v.get("params").cloned().unwrap_or_else(|| json!({}));

    if REVERSE_METHODS.contains(&method) {
        let call_id = payload
            .get("call_id")
            .or_else(|| payload.get("id"))
            .and_then(Value::as_str)
            .or_else(|| id.as_str())
            .unwrap_or_default()
            .to_string();
        let answered = host.respond(&call_id, payload).await;
        let _ = answered;
        return Some((
            codec::jsonrpc_response(&id, &RpcResult::ok_value(json!({}))),
            None,
        ));
    }

    if registry::lookup(method).is_none() {
        return Some((codec::jsonrpc_method_not_found(&id), None));
    }

    let rpc_id = id_key(&id);
    let result = handle_unary(host, rpc_id.as_deref(), method, payload).await;
    Some((codec::jsonrpc_response(&id, &result), None))
}

#[cfg(any())]
mod tests {
    use super::*;

    fn host() -> Arc<HostState> {
        HostState::for_test().expect("host")
    }

    #[tokio::test]
    async fn describe_ok_and_unknown_is_method_not_found() {
        let host = host();
        let (v, token) = dispatch_raw(
            &host,
            r#"{"jsonrpc":"2.0","id":"1","method":"host.describe","params":{}}"#,
        )
        .await
        .expect("describe");
        assert!(token.is_none(), "describe is readonly");
        assert_eq!(v["jsonrpc"], "2.0", "{v}");
        assert!(v.get("result").is_some(), "{v}");
        assert!(v.get("error").is_none(), "{v}");

        let (v, _) = dispatch_raw(
            &host,
            r#"{"jsonrpc":"2.0","id":"2","method":"no_such_method","params":{}}"#,
        )
        .await
        .expect("unknown");
        assert_eq!(v["error"]["code"], -32601, "{v}");
        assert_eq!(v["error"]["data"]["code"], "unregistered_method", "{v}");
    }

    #[tokio::test]
    async fn id_is_echoed_verbatim_in_json_type() {
        let host = host();
        let (v, _) = dispatch_raw(
            &host,
            r#"{"jsonrpc":"2.0","id":7,"method":"host.describe"}"#,
        )
        .await
        .expect("numeric id");
        assert!(v["id"].is_number(), "数字 id MUST 仍是数字: {v}");
        assert_eq!(v["id"], 7);

        let (v, _) = dispatch_raw(
            &host,
            r#"{"jsonrpc":"2.0","id":"j-1","method":"host.describe"}"#,
        )
        .await
        .expect("string id");
        assert_eq!(v["id"], "j-1", "字符串 id MUST 仍是字符串");
    }

    #[tokio::test]
    async fn illegal_envelopes_are_rejected() {
        let host = host();
        for raw in [
            r#"{"jsonrpc":"2.0","id":"1"}"#,
            r#"{"jsonrpc":"1.0","id":"1","method":"host.describe"}"#,
            "not json",
            r#"{"type":"client-request","rpcId":"1","method":"host.describe"}"#,
            r#"[{"jsonrpc":"2.0","id":"1","method":"host.describe"},{"jsonrpc":"2.0","id":"2","method":"host.describe"}]"#,
        ] {
            assert!(
                dispatch_raw(&host, raw).await.is_none(),
                "非法信封 MUST 判否: {raw}"
            );
        }
    }

    #[tokio::test]
    async fn oversized_body_is_rejected() {
        let host = host();
        let big = format!(
            r#"{{"jsonrpc":"2.0","id":"1","method":"host.describe","params":{{"pad":"{}"}}}}"#,
            "x".repeat(MAX_BODY_BYTES)
        );
        assert!(
            dispatch_raw(&host, &big).await.is_none(),
            "超过 4 MiB MUST 判否"
        );
    }

    #[tokio::test]
    async fn missing_params_default_to_empty_object() {
        let host = host();
        let (v, _) = dispatch_raw(
            &host,
            r#"{"jsonrpc":"2.0","id":"3","method":"get_commands"}"#,
        )
        .await
        .expect("no params");
        assert!(v.get("result").is_some(), "{v}");
    }

    #[tokio::test]
    async fn writer_lease_window_across_connections() {
        let host = host();
        let clear = r#"{"clear_steer":true,"clear_follow_up":true}"#;

        let (v, token) = dispatch_raw(
            &host,
            &format!(r#"{{"jsonrpc":"2.0","id":"w1","method":"clear_queue","params":{clear}}}"#),
        )
        .await
        .expect("first write");
        assert!(v.get("result").is_some(), "{v}");
        assert!(token.is_none(), "MUST NOT mint writer token");

        let (v, token) = dispatch_raw(
            &host,
            &format!(r#"{{"jsonrpc":"2.0","id":"w2","method":"clear_queue","params":{clear}}}"#),
        )
        .await
        .expect("second write");
        assert!(v.get("result").is_some(), "same process continues: {v}");
        assert!(token.is_none());

        let (_, readonly) = dispatch_raw(
            &host,
            r#"{"jsonrpc":"2.0","id":"r1","method":"host.describe"}"#,
        )
        .await
        .expect("readonly");
        assert!(readonly.is_none(), "只读 MUST NOT 占用租约");
    }

    #[tokio::test]
    async fn reverse_answer_carries_empty_result() {
        let host = host();
        let (v, token) = dispatch_raw(
            &host,
            r#"{"jsonrpc":"2.0","id":"r1","method":"approve_tool","params":{"call_id":"c1","approved":true}}"#,
        )
        .await
        .expect("reverse answer");
        assert!(token.is_none(), "反向应答不占写者租约");
        assert_eq!(v["id"], "r1", "{v}");
        assert!(v.get("result").is_some(), "MUST 有 result: {v}");
        assert!(v.get("error").is_none(), "{v}");
    }
}
