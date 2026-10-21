use std::sync::Arc;

use crate::occupancy::host::HostState;
use crate::occupancy::rpc_module::{self};
use xylitol_protocol::v3::{
    ClientRequest, DescribeResult, Frame, Request, ResponsePayload, RpcError, ServerResponse,
    method_name,
};

fn to_jsonrpc_request(req: &ClientRequest) -> Result<(String, String), String> {
    let method = match &req.request {
        Request::Command(cmd) => {
            let value = serde_json::to_value(v3_command_ptr(cmd))
                .map_err(|e| format!("command encode: {e}"))?;
            let tag = value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .ok_or("command tag missing")?
                .to_string();
            return Ok((
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req.rpc_id,
                    "method": tag,
                    "params": value,
                })
                .to_string(),
                tag,
            ));
        }
        Request::Describe(_) => xylitol_protocol::registry::METHOD_HOST_DESCRIBE.to_string(),
        Request::Raw(raw) => method_name(&raw.method).to_string(),
        Request::Unknown(_) => return Err("unknown request variant".into()),
    };
    let params = match &req.request {
        Request::Raw(raw) => {
            serde_json::from_str(&raw.json).unwrap_or_else(|_| serde_json::json!({}))
        }
        _ => serde_json::json!({}),
    };
    Ok((
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": req.rpc_id,
            "method": method,
            "params": params,
        })
        .to_string(),
        method,
    ))
}

fn v3_command_ptr(cmd: &xylitol_protocol::v3::Command) -> xylitol_protocol::command::Command {
    crate::v3_mapping::v3_to_command(cmd)
        .unwrap_or_else(|| panic!("v3 Command variant unmapped: {cmd:?}"))
}

fn response_from_raw(
    rpc_id: u64,
    method: &str,
    v: &serde_json::Value,
    token: Option<String>,
) -> Result<ServerResponse, String> {
    if let Some(err) = v.get("error") {
        let code = err
            .pointer("/data/code")
            .and_then(serde_json::Value::as_str)
            .or_else(|| err.get("message").and_then(serde_json::Value::as_str))
            .unwrap_or("internal_error")
            .to_string();
        let details = err
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        return Ok(ServerResponse {
            rpc_id,
            ok: false,
            error: Some(RpcError { code, details }),
            payload: None,
            writer_token: token,
        });
    }
    let result = v.get("result").cloned().unwrap_or(serde_json::Value::Null);
    let payload = if method == xylitol_protocol::registry::METHOD_HOST_DESCRIBE {
        let describe: xylitol_protocol::HostDescribeValue =
            serde_json::from_value(result.clone()).map_err(|e| format!("describe decode: {e}"))?;
        ResponsePayload::DescribeResult(DescribeResult {
            protocol: describe.protocol,
            formats: describe.formats.iter().map(|s| s.to_string()).collect(),
        })
    } else if let Some(typed) = typed_payload(method, &result) {
        typed
    } else {
        ResponsePayload::RawOk(xylitol_protocol::v3::RawOk {
            json: result.to_string(),
        })
    };
    Ok(ServerResponse {
        rpc_id,
        ok: true,
        error: None,
        payload: Some(payload),
        writer_token: token,
    })
}

fn typed_payload(method: &str, result: &serde_json::Value) -> Option<ResponsePayload> {
    use crate::v3_mapping;
    use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeTravel};
    use xylitol_protocol::registry;

    match method {
        registry::METHOD_GET_MESSAGES | registry::METHOD_LOAD_SESSION_ENTRIES => {
            let entries: Vec<SessionEntry> =
                serde_json::from_value(result.get("entries")?.clone()).ok()?;
            Some(ResponsePayload::MessagesResult(
                v3_mapping::messages_result_to_v3(&entries),
            ))
        }
        registry::METHOD_TRAVEL_SESSION_TREE => {
            let travel: SessionTreeTravel = serde_json::from_value(result.clone()).ok()?;
            Some(ResponsePayload::TravelResult(
                v3_mapping::travel_result_to_v3(&travel),
            ))
        }
        _ => None,
    }
}

pub async fn handle_uplink(host: &Arc<HostState>, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let frame = Frame::from_bytes(bytes).map_err(|e| format!("v3 decode: {e}"))?;
    let Frame::ClientRequest(req) = frame else {
        return Err("expected ClientRequest frame".into());
    };
    let (text, method) = to_jsonrpc_request(&req)?;
    let (value, _) = rpc_module::dispatch_raw(host, &text)
        .await
        .ok_or_else(|| "illegal envelope".to_string())?;
    let resp = response_from_raw(req.rpc_id, &method, &value, None)?;
    Frame::ServerResponse(resp)
        .to_bytes()
        .map_err(|e| format!("v3 encode: {e}"))
}

pub fn downlink_frame(
    method: &str,
    payload: serde_json::Value,
    fallback_seq: u64,
) -> Result<Vec<u8>, String> {
    use xylitol_protocol::v3::{
        ApprovalRequested, Notification, QuestionRequested, Resources, ResyncRequired,
        ServerNotification, Subscribed,
    };
    let (seq, notification) = match method {
        "session/event" => {
            let ev: xylitol_protocol::SessionEventPayload =
                serde_json::from_value(payload).map_err(|e| format!("event payload: {e}"))?;
            let xy = crate::FromWireEvent::from_wire_event(&ev.event)
                .map_err(|_| "wire event -> XyEvent".to_string())?;
            let v3_ev = crate::v3_mapping::xy_event_to_v3(&xy)
                .ok_or("XyEvent variant has no v3 projection")?;
            (ev.seq, Notification::Event(v3_ev))
        }
        "session/subscribed" => {
            let p: xylitol_protocol::SessionSubscribedPayload =
                serde_json::from_value(payload).map_err(|e| format!("subscribed payload: {e}"))?;
            (
                p.seq,
                Notification::Subscribed(Subscribed {
                    session_id: p.session_id,
                    seq: p.seq,
                }),
            )
        }
        "session/resync_required" => {
            let p: xylitol_protocol::SessionResyncRequiredPayload =
                serde_json::from_value(payload).map_err(|e| format!("resync payload: {e}"))?;
            (
                fallback_seq,
                Notification::ResyncRequired(ResyncRequired {
                    session_id: p.session_id,
                }),
            )
        }
        "session/resources" => {
            let p: xylitol_protocol::SessionResourcesPayload =
                serde_json::from_value(payload).map_err(|e| format!("resources payload: {e}"))?;
            (
                fallback_seq,
                Notification::Resources(Resources {
                    session_id: p.session_id,
                    snapshot_json: p.snapshot.to_string(),
                }),
            )
        }
        "approval/requested" => {
            let p: xylitol_protocol::ApprovalRequestedPayload =
                serde_json::from_value(payload).map_err(|e| format!("approval payload: {e}"))?;
            (
                fallback_seq,
                Notification::ApprovalRequested(ApprovalRequested { call_id: p.call_id }),
            )
        }
        "question/requested" => {
            let p: xylitol_protocol::QuestionRequestedPayload =
                serde_json::from_value(payload).map_err(|e| format!("question payload: {e}"))?;
            (
                fallback_seq,
                Notification::QuestionRequested(QuestionRequested { call_id: p.call_id }),
            )
        }
        "session/bash_output" => {
            let session_id = payload
                .get("session_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let bash_id = payload
                .get("bash_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let seq = payload
                .get("seq")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let data = payload
                .get("data")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            (
                0,
                Notification::BashOutput(xylitol_protocol::v3::BashOutput {
                    session_id,
                    bash_id,
                    seq,
                    data,
                }),
            )
        }
        _ => return Err(format!("no v3 downlink mapping for method {method}")),
    };
    Frame::ServerNotification(ServerNotification { seq, notification })
        .to_bytes()
        .map_err(|e| format!("v3 downlink encode: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_of(raw: &str) -> serde_json::Value {
        serde_json::from_str(raw).expect("fixture json")
    }

    fn steer_frame(rpc_id: u64) -> Vec<u8> {
        Frame::ClientRequest(ClientRequest {
            rpc_id,
            request: Request::Command(xylitol_protocol::v3::Command::Steer(
                xylitol_protocol::v3::Steer {
                    message: "hi".into(),
                },
            )),
            writer_token: None,
        })
        .to_bytes()
        .unwrap()
    }

    #[test]
    fn frame_detection() {
        assert!(crate::looks_like_fory_envelope(&[0x01, 0xff, 0x22]));
        assert!(!crate::looks_like_fory_envelope(b"{\"jsonrpc\""));
        assert!(!crate::looks_like_fory_envelope(b""));
    }

    #[test]
    fn jsonrpc_request_shape_from_command() {
        let frame = Frame::from_bytes(&steer_frame(7)).unwrap();
        let Frame::ClientRequest(req) = frame else {
            unreachable!()
        };
        let (text, method) = to_jsonrpc_request(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(method, "steer");
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 7);
        assert_eq!(v["method"], "steer");
        assert_eq!(v["params"]["type"], "steer");
        assert_eq!(v["params"]["message"], "hi");
    }

    #[test]
    fn jsonrpc_request_shape_from_raw() {
        let frame = Frame::ClientRequest(ClientRequest {
            rpc_id: 3,
            request: Request::Raw(xylitol_protocol::v3::Raw {
                method: xylitol_protocol::v3::Method::ArmToolFreeze,
                json: r#"{"frozen":true}"#.into(),
            }),
            writer_token: None,
        });
        let Frame::ClientRequest(req) = frame else {
            unreachable!()
        };
        let (text, method) = to_jsonrpc_request(&req).unwrap();
        assert_eq!(method, "arm_tool_freeze");
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["params"]["frozen"], true);
    }

    #[test]
    fn typed_payloads_cover_session_results() {
        let entries_json = r#"{"entries":[{"type":"message","id":"e1","parentId":null,"timestamp":7,"message":{"role":"user","content":"hi"}}]}"#;
        let resp = response_from_raw(
            1,
            "get_messages",
            &json_of(&format!(
                r#"{{"jsonrpc":"2.0","id":1,"result":{entries_json}}}"#
            )),
            None,
        )
        .unwrap();
        let bytes = Frame::ServerResponse(resp).to_bytes().unwrap();
        let Frame::ServerResponse(decoded) = Frame::from_bytes(&bytes).unwrap() else {
            unreachable!()
        };
        match decoded.payload.unwrap() {
            ResponsePayload::MessagesResult(m) => {
                assert_eq!(m.entries.len(), 1, "一条条目不应丢");
                let back = crate::v3_mapping::v3_to_entries(&m).unwrap();
                assert_eq!(
                    serde_json::to_value(&back).unwrap(),
                    serde_json::from_str::<serde_json::Value>(entries_json)
                        .unwrap()
                        .get("entries")
                        .cloned()
                        .unwrap(),
                    "v3 往返后的条目 JSON 与 JSON 轨逐字节同构"
                );
            }
            other => panic!("expected MessagesResult, got {other:?}"),
        }

        let tree = response_from_raw(
            2,
            "session_tree",
            &json_of(r#"{"jsonrpc":"2.0","id":2,"result":{"tree":[{"entry":{"type":"message","id":"e1","parentId":null,"timestamp":7,"message":{"role":"user","content":"hi"}},"children":[],"label":null}]}}"#),
            None,
        )
        .unwrap();
        match tree.payload.unwrap() {
            ResponsePayload::RawOk(raw) => {
                let value: serde_json::Value =
                    serde_json::from_str(&raw.json).expect("raw json parses");
                let nodes = value
                    .get("tree")
                    .and_then(serde_json::Value::as_array)
                    .expect("tree array");
                assert_eq!(nodes.len(), 1);
                let typed = crate::v3_mapping::tree_result_to_v3(
                    &serde_json::from_value::<Vec<xylitol_plugin_sdk::session::SessionTreeNode>>(
                        value.get("tree").cloned().unwrap(),
                    )
                    .unwrap(),
                );
                assert_eq!(typed.nodes.len(), 1);
                assert_eq!(typed.nodes[0].children.len(), 0);
            }
            other => panic!("session_tree 应答 MUST 走 RawOk，实际 {other:?}"),
        }

        let travel = response_from_raw(
            3,
            "travel_session_tree",
            &json_of(r#"{"jsonrpc":"2.0","id":3,"result":{"kind":"message_history","selected_id":"e1","leaf_id":null}}"#),
            None,
        )
        .unwrap();
        match travel.payload.unwrap() {
            ResponsePayload::TravelResult(t) => {
                assert_eq!(t.selected_id, "e1");
                assert!(t.editor_text.is_none(), "缺位字段保持 None");
            }
            other => panic!("expected TravelResult, got {other:?}"),
        }

        let raw = response_from_raw(
            4,
            "get_state",
            &json_of(r#"{"jsonrpc":"2.0","id":4,"result":{"seq":1}}"#),
            None,
        )
        .unwrap();
        match raw.payload.unwrap() {
            ResponsePayload::RawOk(raw) => assert_eq!(raw.json, r#"{"seq":1}"#),
            other => panic!("expected RawOk, got {other:?}"),
        }
    }

    #[test]
    fn response_result_and_error_shapes() {
        let ok = response_from_raw(
            5,
            "get_state",
            &json_of(r#"{"jsonrpc":"2.0","id":5,"result":{"seq":1}}"#),
            None,
        )
        .unwrap();
        assert!(ok.ok);
        match ok.payload.unwrap() {
            ResponsePayload::RawOk(raw) => assert_eq!(raw.json, r#"{"seq":1}"#),
            _ => panic!("expected RawOk"),
        }

        let err = response_from_raw(
            6,
            "steer",
            &json_of(r#"{"jsonrpc":"2.0","id":6,"error":{"code":-32601,"message":"m","data":{"code":"unregistered_method"}}}"#),
            None,
        )
        .unwrap();
        assert!(!err.ok);
        let e = err.error.unwrap();
        assert_eq!(e.code, "unregistered_method");
    }

    #[test]
    fn response_describe_carries_formats() {
        let resp = response_from_raw(
            1,
            "host.describe",
            &json_of(r#"{"jsonrpc":"2.0","id":1,"result":{"protocol":2,"formats":["fory-v3"]}}"#),
            None,
        )
        .unwrap();
        match resp.payload.unwrap() {
            ResponsePayload::DescribeResult(d) => {
                assert_eq!(d.protocol, 2);
                assert_eq!(d.formats, vec!["fory-v3".to_string()]);
            }
            _ => panic!("expected DescribeResult"),
        }
    }
}
