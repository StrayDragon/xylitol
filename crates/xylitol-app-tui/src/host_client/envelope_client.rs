use serde_json::Value;

use crate::v3_mapping;
use crate::wire_event::ToWireEvent;
use xylitol_protocol::RpcMessage;
use xylitol_protocol::RpcResult;
use xylitol_protocol::v3::{
    self, ClientRequest, Describe, Frame, Notification, Raw, Request, ResponsePayload,
    ServerNotification, ServerResponse,
};
use xylitol_protocol::{
    ApprovalRequestedPayload, HostDescribeValue, QuestionRequestedPayload, SessionEventPayload,
    SessionResourcesPayload, SessionResyncRequiredPayload, SessionSubscribedPayload,
    WIRE_FORMAT_FORY_V3, registry,
};

pub const ENVELOPE_FORMAT_ENV: &str = "XYLITOL_ENVELOPE_V3";

pub(super) fn env_enabled() -> bool {
    flag_truthy(std::env::var_os(ENVELOPE_FORMAT_ENV).as_deref())
}

fn flag_truthy(raw: Option<&std::ffi::OsStr>) -> bool {
    let Some(raw) = raw else { return false };
    let normalized = raw.to_string_lossy().trim().to_ascii_lowercase();
    normalized == "1" || normalized == "true"
}

pub fn stable_rpc_id(rpc_id: &str) -> u64 {
    match rpc_id.parse::<u64>() {
        Ok(n) => n,
        Err(_) => {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            rpc_id.hash(&mut hasher);
            hasher.finish()
        }
    }
}

fn method_discriminant(method: &str) -> Option<v3::Method> {
    v3::METHOD_NAMES
        .iter()
        .find(|(_, name)| *name == method)
        .map(|(m, _)| m.clone())
}

fn build_request(method: &str, payload: &Value) -> Result<Request, String> {
    if method == registry::METHOD_HOST_DESCRIBE {
        return Ok(Request::Describe(Describe {}));
    }
    let discriminant = method_discriminant(method)
        .ok_or_else(|| format!("RAW method {method} has no v3 Method discriminant"))?;
    Ok(Request::Raw(Raw {
        method: discriminant,
        json: payload.to_string(),
    }))
}

pub fn client_request_bytes(
    rpc_id: &str,
    method: &str,
    payload: &Value,
) -> Result<Vec<u8>, String> {
    let frame = Frame::ClientRequest(ClientRequest {
        rpc_id: stable_rpc_id(rpc_id),
        request: build_request(method, payload)?,
        writer_token: None,
    });
    frame.to_bytes().map_err(|e| format!("v3 encode: {e}"))
}

pub(super) fn describe_request_bytes() -> Result<Vec<u8>, String> {
    let frame = Frame::ClientRequest(ClientRequest {
        rpc_id: 0,
        request: Request::Describe(Describe {}),
        writer_token: None,
    });
    frame.to_bytes().map_err(|e| format!("v3 encode: {e}"))
}

pub(super) fn formats_advertise_envelope(result: &RpcResult) -> bool {
    let Ok(describe) =
        serde_json::from_value::<HostDescribeValue>(result.value.clone().unwrap_or(Value::Null))
    else {
        return false;
    };
    describe.formats.iter().any(|f| f == WIRE_FORMAT_FORY_V3)
}

pub(super) enum Downlink {
    Response { rpc_id: u64, result: RpcResult },
    Notification(RpcMessage),
}

pub(super) fn decode_downlink(
    bytes: &[u8],
    session_id: &mut String,
) -> Result<Option<Downlink>, String> {
    let frame = Frame::from_bytes(bytes).map_err(|e| format!("v3 decode: {e}"))?;
    Ok(match frame {
        Frame::ServerResponse(resp) => Some(Downlink::Response {
            rpc_id: resp.rpc_id,
            result: server_response_to_result(&resp),
        }),
        Frame::ServerNotification(note) => {
            notification_to_message(&note, session_id).map(Downlink::Notification)
        }
        Frame::ClientRequest(_) | Frame::Unknown(_) => None,
    })
}

pub fn server_response_to_result(resp: &ServerResponse) -> RpcResult {
    if resp.ok {
        RpcResult::ok_value(payload_value(resp.payload.as_ref()))
    } else {
        let err = resp.error.clone().unwrap_or_default();
        RpcResult::error(err.code, err.details)
    }
}

fn parse_raw_json(json: &str) -> Value {
    use serde::Deserialize as _;
    if xylitol_protocol::codec::json_nesting_exceeds_limit(json) {
        return Value::Null;
    }
    let mut de = serde_json::Deserializer::from_str(json);
    de.disable_recursion_limit();
    Value::deserialize(&mut de).unwrap_or(Value::Null)
}

fn payload_value(payload: Option<&ResponsePayload>) -> Value {
    match payload {
        Some(ResponsePayload::DescribeResult(d)) => serde_json::to_value(HostDescribeValue {
            protocol: d.protocol,
            formats: d.formats.clone(),
        })
        .unwrap_or(Value::Null),
        Some(ResponsePayload::MessagesResult(m)) => match v3_mapping::v3_to_entries(m) {
            Some(entries) => {
                serde_json::json!({ "entries": serde_json::to_value(entries).unwrap_or(Value::Null) })
            }
            None => Value::Null,
        },
        Some(ResponsePayload::TreeResult(t)) => match v3_mapping::v3_to_tree_nodes(t) {
            Some(nodes) => {
                serde_json::json!({ "tree": serde_json::to_value(nodes).unwrap_or(Value::Null) })
            }
            None => Value::Null,
        },
        Some(ResponsePayload::TravelResult(t)) => {
            serde_json::to_value(v3_mapping::v3_to_travel_result(t)).unwrap_or(Value::Null)
        }
        Some(ResponsePayload::RawOk(raw)) => parse_raw_json(&raw.json),
        Some(ResponsePayload::SubscribeResult(s)) => {
            serde_json::to_value(SessionSubscribedPayload {
                session_id: s.session_id.clone(),
                seq: s.seq,
            })
            .unwrap_or(Value::Null)
        }
        _ => Value::Null,
    }
}

pub(super) fn notification_to_message(
    note: &ServerNotification,
    session_id: &mut String,
) -> Option<RpcMessage> {
    let mapped = match &note.notification {
        Notification::Event(ev) => {
            let xy = v3_mapping::v3_event_to_xy(ev)?;
            let wire_event = xy.to_wire_event()?;
            let payload = serde_json::to_value(SessionEventPayload {
                session_id: session_id.clone(),
                seq: note.seq,
                event: wire_event,
            })
            .ok()?;
            ("session/event", payload)
        }
        Notification::Subscribed(s) => {
            *session_id = s.session_id.clone();
            let payload = serde_json::to_value(SessionSubscribedPayload {
                session_id: s.session_id.clone(),
                seq: s.seq,
            })
            .ok()?;
            ("session/subscribed", payload)
        }
        Notification::ResyncRequired(r) => {
            let payload = serde_json::to_value(SessionResyncRequiredPayload {
                session_id: r.session_id.clone(),
            })
            .ok()?;
            ("session/resync_required", payload)
        }
        Notification::Resources(r) => {
            let payload = serde_json::to_value(SessionResourcesPayload {
                session_id: r.session_id.clone(),
                snapshot: serde_json::from_str(&r.snapshot_json).unwrap_or(Value::Null),
            })
            .ok()?;
            ("session/resources", payload)
        }
        Notification::ApprovalRequested(a) => {
            let payload = serde_json::to_value(ApprovalRequestedPayload {
                call_id: a.call_id.clone(),
            })
            .ok()?;
            ("approval/requested", payload)
        }
        Notification::QuestionRequested(q) => {
            let payload = serde_json::to_value(QuestionRequestedPayload {
                call_id: q.call_id.clone(),
            })
            .ok()?;
            ("question/requested", payload)
        }
        Notification::BashOutput(b) => {
            let payload = serde_json::json!({
                "session_id": b.session_id,
                "bash_id": b.bash_id,
                "seq": b.seq,
                "data": b.data,
            });
            ("session/bash_output", payload)
        }
        Notification::Unknown(_) => return None,
    };
    let (method, payload) = mapped;
    Some(RpcMessage::ServerRequest {
        rpc_id: String::new(),
        method: method.to_string(),
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use xylitol_protocol::v3::{
        ApprovalRequested, DescribeResult, Event as V3Event, ResyncRequired, ServerNotification,
        Subscribed, TextDelta as V3TextDelta, ToolStart as V3ToolStart,
    };

    fn decode_frame(bytes: &[u8]) -> Frame {
        Frame::from_bytes(bytes).expect("roundtrip decode")
    }

    #[test]
    fn env_flag_parsing() {
        let flag = |v: &str| flag_truthy(Some(std::ffi::OsStr::new(v)));
        assert!(!flag_truthy(None));
        assert!(!flag("0"));
        assert!(!flag("off"));
        assert!(!flag(""));
        assert!(flag("1"));
        assert!(flag("true"));
        assert!(flag("TRUE"));
        assert!(flag(" true "));
        assert_eq!(ENVELOPE_FORMAT_ENV, "XYLITOL_ENVELOPE_V3");
        assert_ne!(ENVELOPE_FORMAT_ENV, "XYLITOL_WIRE_V3");
    }

    #[test]
    fn stable_rpc_id_numeric_and_hashed() {
        assert_eq!(stable_rpc_id("42"), 42);
        assert_eq!(stable_rpc_id("0"), 0);
        let a = stable_rpc_id("0198f4b5-6d3e-7f2a-9c1b-4d5e6f708192");
        let b = stable_rpc_id("0198f4b5-6d3e-7f2a-9c1b-4d5e6f708192");
        assert_eq!(a, b, "same rpcId string must map to the same u64");
        let c = stable_rpc_id("other-id");
        assert_ne!(a, c);
    }

    #[test]
    fn request_family_shapes() {
        let bytes = client_request_bytes("7", "host.describe", &json!({})).unwrap();
        let Frame::ClientRequest(req) = decode_frame(&bytes) else {
            panic!("expected ClientRequest")
        };
        assert_eq!(req.rpc_id, 7);
        assert!(matches!(req.request, Request::Describe(_)));

        let bytes =
            client_request_bytes("8", "persist_trust", &json!({"mode": "project"})).unwrap();
        let Frame::ClientRequest(req) = decode_frame(&bytes) else {
            panic!("expected ClientRequest")
        };
        match &req.request {
            Request::Raw(raw) => {
                assert_eq!(raw.method, v3::Method::PersistTrust);
                assert_eq!(raw.json, r#"{"mode":"project"}"#);
            }
            other => panic!("expected Raw, got {other:?}"),
        }

        let bytes = client_request_bytes(
            "9",
            "subscribe",
            &json!({"session_id": "s1", "last_seq": 3, "cwd": "/tmp"}),
        )
        .unwrap();
        let Frame::ClientRequest(req) = decode_frame(&bytes) else {
            panic!("expected ClientRequest")
        };
        match &req.request {
            Request::Raw(raw) => {
                assert_eq!(raw.method, v3::Method::Subscribe);
                let p: serde_json::Value = serde_json::from_str(&raw.json).unwrap();
                assert_eq!(p["session_id"], "s1");
                assert_eq!(p["last_seq"], 3);
                assert_eq!(p["cwd"], "/tmp");
            }
            other => panic!("expected RAW subscribe, got {other:?}"),
        }
        assert!(req.writer_token.is_none(), "MUST NOT stamp writer_token");

        let bytes = client_request_bytes(
            "10",
            "prompt",
            &json!({"message": "hi", "session_id": "sess-1", "cwd": "/tmp", "model_id": "m1"}),
        )
        .unwrap();
        let Frame::ClientRequest(req) = decode_frame(&bytes) else {
            panic!("expected ClientRequest")
        };
        match &req.request {
            Request::Raw(raw) => {
                assert_eq!(raw.method, v3::Method::Prompt);
                let p: serde_json::Value = serde_json::from_str(&raw.json).unwrap();
                assert_eq!(p["session_id"], "sess-1");
                assert_eq!(p["cwd"], "/tmp");
                assert_eq!(p["message"], "hi");
            }
            other => panic!("expected RAW prompt, got {other:?}"),
        }

        assert!(client_request_bytes("1", "no_such_method", &json!({})).is_err());
    }

    #[test]
    fn non_registry_command_maps() {
        let bytes = client_request_bytes(
            "5",
            "approve_tool",
            &json!({"call_id": "c1", "approved": true}),
        )
        .unwrap();
        let Frame::ClientRequest(req) = decode_frame(&bytes) else {
            panic!("expected ClientRequest")
        };
        match &req.request {
            Request::Raw(raw) => {
                assert_eq!(raw.method, v3::Method::ApproveTool);
                let p: serde_json::Value = serde_json::from_str(&raw.json).unwrap();
                assert_eq!(p["call_id"], "c1");
                assert_eq!(p["approved"], true);
            }
            other => panic!("expected RAW approve_tool, got {other:?}"),
        }
    }

    #[test]
    fn server_response_to_result_shapes() {
        let ok = ServerResponse {
            rpc_id: 5,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::RawOk(v3::RawOk {
                json: r#"{"seq":1}"#.into(),
            })),
            writer_token: Some("lease-9".into()),
        };
        let result = server_response_to_result(&ok);
        assert!(result.ok);
        assert_eq!(result.value, Some(json!({"seq": 1})));
        assert!(
            result.writer_token.is_none(),
            "wire writer_token MUST NOT become occupancy"
        );

        let describe = ServerResponse {
            rpc_id: 1,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::DescribeResult(DescribeResult {
                protocol: 2,
                formats: vec!["jsonrpc".into(), "fory-v3".into()],
            })),
            writer_token: None,
        };
        let result = server_response_to_result(&describe);
        assert_eq!(
            result.value,
            Some(json!({"protocol": 2, "formats": ["jsonrpc", "fory-v3"]})),
            "describe value keeps the JSON-track HostDescribeValue shape"
        );
        assert!(formats_advertise_envelope(&result));

        let err = ServerResponse {
            rpc_id: 6,
            ok: false,
            error: Some(v3::RpcError {
                code: "session_write_held".into(),
                details: "session s is being written by another process".into(),
            }),
            payload: None,
            writer_token: None,
        };
        let result = server_response_to_result(&err);
        assert!(!result.ok);
        assert_eq!(
            result.error.map(|e| (e.code, e.details)),
            Some((
                "session_write_held".into(),
                "session s is being written by another process".into(),
            ))
        );
    }

    #[test]
    fn raw_ok_parses_beyond_default_recursion_limit() {
        let mut leaf = r#"{"entry":"e0","children":[]}"#.to_string();
        for i in 1..150u32 {
            leaf = format!(r#"{{"entry":"e{i}","children":[{leaf}]}}"#);
        }
        let deep = format!(r#"{{"tree":[{leaf}]}}"#);
        assert!(deep.len() > 1024, "fixture 应显著深过默认上限");

        let resp = ServerResponse {
            rpc_id: 9,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::RawOk(v3::RawOk { json: deep })),
            writer_token: None,
        };
        let result = server_response_to_result(&resp);
        assert!(result.ok, "深树 RawOk MUST 解析成功: {result:?}");
        let value = result.value.expect("深树 MUST 不为 Null");
        assert!(
            value.get("tree").is_some(),
            "tree 键 MUST 存在，不得降级 Null: {value:?}"
        );
    }

    #[test]
    fn raw_ok_over_json_depth_limit_degrades_to_null() {
        use xylitol_protocol::codec::JSON_DEPTH_LIMIT;
        let depth = JSON_DEPTH_LIMIT as usize + 1;
        let too_deep = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        let resp = ServerResponse {
            rpc_id: 10,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::RawOk(v3::RawOk { json: too_deep })),
            writer_token: None,
        };
        let result = server_response_to_result(&resp);
        assert!(result.ok, "超限仍是 ok 应答，只是载荷退化: {result:?}");
        assert_eq!(
            result.value,
            Some(Value::Null),
            "超限 RawOk MUST 退化 Null（r1907）"
        );
    }

    #[test]
    fn raw_ok_parses_two_hundred_nested_arrays() {
        let deep = format!("{}0{}", "[".repeat(200), "]".repeat(200));
        let resp = ServerResponse {
            rpc_id: 11,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::RawOk(v3::RawOk { json: deep })),
            writer_token: None,
        };
        let result = server_response_to_result(&resp);
        let value = result.value.expect("~200 层 RawOk MUST 不为 Null");
        assert!(value.is_array(), "200 层数组 MUST 解析为 Array: {value:?}");
    }

    #[test]
    fn negotiate_gate_rejects_without_fory_v3() {
        let jsonrpc_only = RpcResult::ok_value(json!({"protocol": 2, "formats": ["jsonrpc"]}));
        assert!(!formats_advertise_envelope(&jsonrpc_only));
        let old_host = RpcResult::ok_value(json!({"protocol": 2}));
        assert!(!formats_advertise_envelope(&old_host));
        let dual = RpcResult::ok_value(json!({"protocol": 2, "formats": ["jsonrpc", "fory-v3"]}));
        assert!(formats_advertise_envelope(&dual));
    }

    #[test]
    fn typed_payloads_restore_json_track_shape() {
        use xylitol_plugin_sdk::session::{
            EntryBase, MessageEntry, SessionEntry, SessionHeader, SessionTreeKind, SessionTreeNode,
            SessionTreeTravel,
        };

        let header = SessionEntry::Header(SessionHeader {
            entry_type: String::new(),
            version: 7,
            id: "s1".into(),
            timestamp: 1_700_000_000,
            cwd: "/w".into(),
            parent_session: Some("s0".into()),
            fork_at_entry_id: None,
        });
        let message = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: String::new(),
                id: "e1".into(),
                parent_id: Some("e0".into()),
                timestamp: 1_700_000_001,
            },
            message: json!({"role": "user", "content": "hi"}),
        });
        let entries = vec![header.clone(), message.clone()];
        let ok_entries = ServerResponse {
            rpc_id: 2,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::MessagesResult(
                v3_mapping::messages_result_to_v3(&entries),
            )),
            writer_token: None,
        };
        assert_eq!(
            server_response_to_result(&ok_entries).value,
            Some(json!({ "entries": serde_json::to_value(&entries).unwrap() })),
            "get_messages / load_session_entries 的包裹键必须是 entries"
        );

        let nodes = vec![SessionTreeNode {
            entry: message,
            children: Vec::new(),
            label: Some("checkpoint".into()),
        }];
        let ok_tree = ServerResponse {
            rpc_id: 3,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::TreeResult(v3_mapping::tree_result_to_v3(
                &nodes,
            ))),
            writer_token: None,
        };
        assert_eq!(
            server_response_to_result(&ok_tree).value,
            Some(json!({ "tree": serde_json::to_value(&nodes).unwrap() })),
            "session_tree 的包裹键必须是 tree"
        );

        let travel = SessionTreeTravel {
            kind: SessionTreeKind::MessageHistory,
            selected_id: "e1".into(),
            leaf_id: None,
            editor_text: Some("prefill".into()),
        };
        let ok_travel = ServerResponse {
            rpc_id: 4,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::TravelResult(
                v3_mapping::travel_result_to_v3(&travel),
            )),
            writer_token: None,
        };
        assert_eq!(
            server_response_to_result(&ok_travel).value,
            Some(serde_json::to_value(&travel).unwrap()),
            "travel 与 JSON 轨一样是平对象"
        );
    }

    #[test]
    fn notification_event_and_subscribed_shapes() {
        let mut session_id = String::new();
        let note = ServerNotification {
            seq: 11,
            notification: Notification::Event(V3Event::ToolStart(V3ToolStart {
                id: "t1".into(),
                name: "grep".into(),
                args_json: r#"{"pattern":"TODO"}"#.into(),
            })),
        };
        let Some(RpcMessage::ServerRequest {
            rpc_id,
            method,
            payload,
        }) = notification_to_message(&note, &mut session_id)
        else {
            panic!("event must map")
        };
        assert_eq!(rpc_id, "");
        assert_eq!(method, "session/event");
        assert_eq!(payload["seq"], 11);
        assert_eq!(payload["session_id"], "", "no Subscribed seen yet");
        assert_eq!(payload["event"]["type"], "tool_start");
        assert_eq!(payload["event"]["id"], "t1");
        assert_eq!(payload["event"]["name"], "grep");
        assert_eq!(payload["event"]["args"]["pattern"], "TODO");

        let subscribed = ServerNotification {
            seq: 12,
            notification: Notification::Subscribed(Subscribed {
                session_id: "s-7".into(),
                seq: 12,
            }),
        };
        let Some(RpcMessage::ServerRequest {
            method, payload, ..
        }) = notification_to_message(&subscribed, &mut session_id)
        else {
            panic!("subscribed must map")
        };
        assert_eq!(method, "session/subscribed");
        assert_eq!(payload, json!({"session_id": "s-7", "seq": 12}));

        let note = ServerNotification {
            seq: 13,
            notification: Notification::Event(V3Event::TextDelta(V3TextDelta { text: "x".into() })),
        };
        let Some(RpcMessage::ServerRequest { payload, .. }) =
            notification_to_message(&note, &mut session_id)
        else {
            panic!("event must map")
        };
        assert_eq!(payload["session_id"], "s-7");
        assert_eq!(payload["event"]["type"], "text_delta");
    }

    #[test]
    fn notification_other_variants_and_unknown() {
        let mut session_id = String::new();
        let cases: Vec<(Notification, &str, Value)> = vec![
            (
                Notification::ResyncRequired(ResyncRequired {
                    session_id: "s1".into(),
                }),
                "session/resync_required",
                json!({"session_id": "s1"}),
            ),
            (
                Notification::Resources(v3::Resources {
                    session_id: "s1".into(),
                    snapshot_json: r#"{"skills":[]}"#.into(),
                }),
                "session/resources",
                json!({"session_id": "s1", "snapshot": {"skills": []}}),
            ),
            (
                Notification::ApprovalRequested(ApprovalRequested {
                    call_id: "c9".into(),
                }),
                "approval/requested",
                json!({"call_id": "c9"}),
            ),
            (
                Notification::QuestionRequested(v3::QuestionRequested {
                    call_id: "q1".into(),
                }),
                "question/requested",
                json!({"call_id": "q1"}),
            ),
        ];
        for (notification, method, payload) in cases {
            let note = ServerNotification {
                seq: 1,
                notification,
            };
            let Some(RpcMessage::ServerRequest {
                method: got_method,
                payload: got_payload,
                ..
            }) = notification_to_message(&note, &mut session_id)
            else {
                panic!("{method} must map")
            };
            assert_eq!(got_method, method);
            assert_eq!(got_payload, payload);
        }
    }

    #[test]
    fn decode_downlink_ignores_non_server_frames() {
        let mut session_id = String::new();
        let bytes = client_request_bytes("1", "host.describe", &json!({})).unwrap();
        assert!(
            decode_downlink(&bytes, &mut session_id)
                .expect("decode")
                .is_none()
        );
    }
}
