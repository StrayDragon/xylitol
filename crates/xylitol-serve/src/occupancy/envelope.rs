use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use xylitol_protocol::codec::{self, Inbound, Rejected};
use xylitol_protocol::{ErrorObject, Message, Notification, Outcome, Request, RequestId, Response};

use crate::occupancy::host::HostState;
use crate::occupancy::rpc_module::dispatch_request;
use crate::wire::Reply;

pub async fn dispatch_text(host: &Arc<HostState>, body: &[u8], mux_id: Option<u64>) -> Reply {
    respond_to_body(body, move |req| async move {
        dispatch_request(host, &req.id, &req.method, &req.params, mux_id).await
    })
    .await
}

pub async fn respond_to_body<F, Fut>(body: &[u8], mut dispatch: F) -> Reply
where
    F: FnMut(Request) -> Fut,
    Fut: Future<Output = Response>,
{
    match codec::parse_bytes(body) {
        Err(rejected) => Reply::Invalid(rejection_value(rejected)),
        Ok(Inbound::Single(Err(rejected))) => Reply::Invalid(rejection_value(rejected)),
        Ok(Inbound::Single(Ok(Message::Request(req)))) => {
            Reply::Answer(codec::response_value(&dispatch(req).await))
        }
        Ok(Inbound::Single(Ok(Message::Notification(_)))) => Reply::Silent,
        Ok(Inbound::Single(Ok(Message::Response(resp)))) => Reply::Invalid(not_a_request(resp.id)),
        Ok(Inbound::Batch(items)) => {
            let mut entries = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Err(rejected) => entries.push(rejection_value(rejected)),
                    Ok(Message::Request(req)) => {
                        entries.push(codec::response_value(&dispatch(req).await));
                    }
                    Ok(Message::Response(resp)) => entries.push(not_a_request(resp.id)),
                    Ok(Message::Notification(_)) => {}
                }
            }
            if entries.is_empty() {
                Reply::Silent
            } else {
                Reply::Answer(Value::Array(entries))
            }
        }
    }
}

pub fn downlink_text(note: &Notification) -> String {
    codec::notification_value(&note.method, &note.params).to_string()
}

fn rejection_value(rejected: Rejected) -> Value {
    codec::response_value(&Response {
        id: rejected.id,
        outcome: Outcome::Error(rejected.error),
    })
}

fn not_a_request(id: Option<RequestId>) -> Value {
    codec::response_value(&Response {
        id,
        outcome: Outcome::Error(ErrorObject::invalid_request(
            "a JSON-RPC response is not a request",
        )),
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use salvo::http::StatusCode;
    use serde_json::json;

    use super::*;

    async fn echo(req: Request) -> Response {
        Response {
            id: Some(req.id),
            outcome: Outcome::Result(json!({ "method": req.method })),
        }
    }

    async fn run(body: &[u8]) -> (Reply, usize) {
        let calls = RefCell::new(0);
        let reply = respond_to_body(body, |req| {
            *calls.borrow_mut() += 1;
            echo(req)
        })
        .await;
        (reply, calls.into_inner())
    }

    fn body_json(reply: &Reply) -> Value {
        serde_json::from_str(&reply.body().expect("body")).expect("json body")
    }

    #[tokio::test]
    async fn batch_answers_requests_in_order_and_skips_notifications() {
        let seen = RefCell::new(Vec::new());
        let body = br#"[
            {"jsonrpc":"2.0","id":"a","method":"first"},
            {"jsonrpc":"2.0","method":"note"},
            {"jsonrpc":"2.0","id":2,"method":"second"}
        ]"#;
        let reply = respond_to_body(body, |req| {
            seen.borrow_mut().push(req.method.clone());
            echo(req)
        })
        .await;
        assert_eq!(
            *seen.borrow(),
            vec!["first".to_string(), "second".to_string()]
        );
        assert_eq!(
            body_json(&reply),
            json!([
                {"jsonrpc": "2.0", "id": "a", "result": {"method": "first"}},
                {"jsonrpc": "2.0", "id": 2, "result": {"method": "second"}}
            ])
        );
        assert_eq!(reply.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn batch_response_object_is_an_invalid_entry_and_never_dispatched() {
        let (reply, calls) =
            run(br#"[{"jsonrpc":"2.0","id":7,"result":{}},{"jsonrpc":"2.0","id":8,"method":"m"}]"#)
                .await;
        assert_eq!(calls, 1);
        assert_eq!(reply.status(), StatusCode::OK);
        let entries = body_json(&reply);
        assert_eq!(entries[0]["id"], 7);
        assert_eq!(entries[0]["error"]["code"], -32600);
        assert_eq!(entries[1]["id"], 8);
        assert_eq!(entries[1]["result"]["method"], "m");
    }

    #[tokio::test]
    async fn single_notification_is_silent_and_never_executed() {
        let (reply, calls) = run(br#"{"jsonrpc":"2.0","method":"host.describe"}"#).await;
        assert_eq!(reply, Reply::Silent);
        assert_eq!(reply.status(), StatusCode::OK);
        assert_eq!(calls, 0);
    }

    #[tokio::test]
    async fn notification_only_batch_is_silent() {
        let (reply, calls) =
            run(br#"[{"jsonrpc":"2.0","method":"a"},{"jsonrpc":"2.0","method":"b"}]"#).await;
        assert_eq!(reply, Reply::Silent);
        assert_eq!(calls, 0);
    }

    #[tokio::test]
    async fn empty_batch_is_invalid_request_400() {
        let (reply, calls) = run(b"[]").await;
        assert_eq!(calls, 0);
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(&reply)["error"]["code"], -32600);
    }

    #[tokio::test]
    async fn batch_error_entry_keeps_its_id_and_the_batch_continues() {
        let (reply, calls) = run(
            br#"[{"jsonrpc":"1.0","id":"x","method":"m"},{"jsonrpc":"2.0","id":"y","method":"m"}]"#,
        )
        .await;
        assert_eq!(calls, 1);
        let entries = body_json(&reply);
        assert_eq!(entries[0]["id"], "x");
        assert_eq!(entries[0]["error"]["code"], -32600);
        assert_eq!(entries[1]["id"], "y");
        assert_eq!(entries[1]["result"]["method"], "m");
    }

    #[tokio::test]
    async fn truncated_json_is_parse_error_400_with_null_id() {
        let (reply, calls) = run(b"{").await;
        assert_eq!(calls, 0);
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        let v = body_json(&reply);
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn non_utf8_body_is_parse_error_400() {
        let (reply, _) = run(&[0xff, 0xfe]).await;
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(&reply)["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn scalar_body_is_invalid_request_400() {
        let (reply, calls) = run(b"3").await;
        assert_eq!(calls, 0);
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(&reply)["error"]["code"], -32600);
    }

    #[tokio::test]
    async fn single_response_is_rejected_with_its_id_echoed() {
        let (reply, calls) = run(br#"{"jsonrpc":"2.0","id":"r9","result":{}}"#).await;
        assert_eq!(calls, 0);
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        let v = body_json(&reply);
        assert_eq!(v["id"], "r9");
        assert_eq!(v["error"]["code"], -32600);
    }

    #[tokio::test]
    async fn single_request_echoes_an_integer_id() {
        let (reply, calls) = run(br#"{"jsonrpc":"2.0","id":41,"method":"host.describe"}"#).await;
        assert_eq!(calls, 1);
        assert_eq!(reply.status(), StatusCode::OK);
        let v = body_json(&reply);
        assert!(v["id"].is_number(), "{v}");
        assert_eq!(v["id"], 41);
        assert_eq!(v["result"]["method"], "host.describe");
    }

    #[test]
    fn downlink_text_is_a_notification_without_id() {
        let note = Notification {
            method: "session/subscribed".into(),
            params: json!({"session_id": "s", "seq": 0, "inflight": false}),
        };
        let v: Value = serde_json::from_str(&downlink_text(&note)).expect("json");
        assert_eq!(
            v,
            json!({
                "jsonrpc": "2.0",
                "method": "session/subscribed",
                "params": {"session_id": "s", "seq": 0, "inflight": false}
            })
        );
        assert!(v.get("id").is_none());
    }
}
