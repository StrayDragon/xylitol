//! wire v3(fory 二进制)上行处理 — c2834 tasks 2.1–2.4(spec r1902/r1904)。
//!
//! v3 是**纯编码层**:上行帧解码后转为 JSON-RPC 2.0 文本喂既有
//! `rpc_module::dispatch_raw`，方法表 / 幂等 / 写者租约 / 审批执行面零分叉
//! (对拍纪律 spec r1908:两路径由同一 dispatch 与事件源支撑)。
//! 下行 v3 事件通知由 mux 侧编码(见 `http.rs` binary 通道);应答侧
//! `host.describe` 与 task 2.5b 已接的三类(会话条目 / 会话树 / travel)走具名
//! union,其余方法以 `RawOk`(JSON 原文)承载。

use std::sync::Arc;

use crate::app::server::host::HostState;
use crate::app::server::rpc_module::{self};
use crate::protocol::wire::v3::{
    ClientRequest, DescribeResult, Frame, Request, ResponsePayload, RpcError, ServerResponse,
    method_name,
};

/// v3 应答帧的 HTTP content-type(WS 场景即 binary 帧本身,无 content-type)。
pub(crate) const CONTENT_TYPE: &str = "application/x-fory-v3";

/// v3 帧识别:xlang 编码以 `0x01` 头字节开始;JSON 文本永不以该字节开头。
pub(crate) fn looks_like_fory_v3(bytes: &[u8]) -> bool {
    bytes.first() == Some(&0x01)
}

/// 上行帧 → JSON-RPC 请求文本与目标方法名。
///
/// `Request::Command` 的 method 名直接取 serde type tag(`steer` / `queue_stats`
/// 等与 registry SSOT 名一致,由 tag 漂移测试锁定);`Request::Raw` 携带
/// registry 名单内的 RAW 方法(`arm_tool_freeze` / `persist_trust`)。
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
        Request::Describe(_) => crate::protocol::wire::registry::METHOD_HOST_DESCRIBE.to_string(),
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

/// 生成物 `Command` 各变体载荷是具名 struct;serde 形状需要 `&wire::Command`。
/// 由 `mapping::v3_to_command` 提供领域转换(未映射变体在此即为失败)。
fn v3_command_ptr(
    cmd: &crate::protocol::wire::v3::Command,
) -> crate::protocol::wire::command::Command {
    crate::protocol::wire::v3::mapping::v3_to_command(cmd)
        .unwrap_or_else(|| panic!("v3 Command variant unmapped: {cmd:?}"))
}

/// JSON-RPC 应答文本 → v3 `ServerResponse` 帧。
///
/// `host.describe` 特判为 `DescribeResult`(承载 wire 格式能力,spec r1911);
/// 其余成功应答第一版为 `RawOk`(result JSON 原文)。产品错误码取
/// `error.data.code`(缺失回落信封 message),对齐 `polish_rpc_json` 语义。
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
    let payload = if method == crate::protocol::wire::registry::METHOD_HOST_DESCRIBE {
        let describe: crate::protocol::wire::HostDescribeValue =
            serde_json::from_value(result.clone()).map_err(|e| format!("describe decode: {e}"))?;
        ResponsePayload::DescribeResult(DescribeResult {
            protocol: describe.protocol,
            formats: describe.formats.iter().map(|s| s.to_string()).collect(),
        })
    } else if let Some(typed) = typed_payload(method, &result) {
        typed
    } else {
        ResponsePayload::RawOk(crate::protocol::wire::v3::RawOk {
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

/// 强 schema 应答（c2834 task 2.5b）：会话条目 / 会话树 / travel 三类载荷走
/// 具名 union,拿到完整体积收益;其余方法（含未命中与解码降级）走 `RawOk`。
///
/// 降级是逐方法的：某方法内任一 unknown 变体就整份退回 `RawOk`，不拼半份
/// 强 schema（对拍不断链，r1908）。
fn typed_payload(method: &str, result: &serde_json::Value) -> Option<ResponsePayload> {
    use crate::protocol::session::{SessionEntry, SessionTreeTravel};
    use crate::protocol::wire::registry;
    use crate::protocol::wire::v3::mapping;

    match method {
        registry::METHOD_GET_MESSAGES | registry::METHOD_LOAD_SESSION_ENTRIES => {
            let entries: Vec<SessionEntry> =
                serde_json::from_value(result.get("entries")?.clone()).ok()?;
            Some(ResponsePayload::MessagesResult(
                mapping::messages_result_to_v3(&entries),
            ))
        }
        // c2845: session_tree 刻意不走强 schema `TreeResult`。
        //
        // `SessionTreeNode.children` 在生成的 binary codec 中按树深逐层递归
        // （debug 帧巨大）：历史上 ~185 层深树即可让 tokio worker 默认栈
        // 溢出 abort（serve `/session-tree`）。该方法改走 `RawOk`——载荷与
        // JSON 轨逐字面同构（`{"tree": [...]}`），由客户端 serde 解析（小帧、
        // 深度安全），把「按深度递归」从传输路径整体移除。强 schema
        // `TreeResult` / `v3_to_tree_nodes` 保留供兼容解码与旧端对拍。
        registry::METHOD_TRAVEL_SESSION_TREE => {
            let travel: SessionTreeTravel = serde_json::from_value(result.clone()).ok()?;
            Some(ResponsePayload::TravelResult(mapping::travel_result_to_v3(
                &travel,
            )))
        }
        _ => None,
    }
}

/// 处理一条 v3 上行帧,产出 v3 应答帧字节与本次应答携带的写者租约
/// (POST 与 WS binary 共用)。
///
/// 帧内 `writer_token` 优先;WS 场景由调用方传入**连接本地租约**
/// (`header_writer` 形参,r1793 语义:同连接内后续 unary 用连接本地租约,
/// 应答携带新 mint 的 token 时调用方须更新连接本地状态)。非
/// `ClientRequest` 帧与未知变体按 envelope 错误处理(调用方决定 400 或
/// 断连)。
pub(crate) async fn handle_uplink(
    host: &Arc<HostState>,
    bytes: &[u8],
    conn_writer: Option<String>,
) -> Result<(Vec<u8>, Option<String>), String> {
    let frame = Frame::from_bytes(bytes).map_err(|e| format!("v3 decode: {e}"))?;
    let Frame::ClientRequest(req) = frame else {
        return Err("expected ClientRequest frame".into());
    };
    let (text, method) = to_jsonrpc_request(&req)?;
    let writer = req.writer_token.clone().or(conn_writer);
    let (value, token) = rpc_module::dispatch_raw(host, &text, writer)
        .await
        .ok_or_else(|| "illegal envelope".to_string())?;
    let resp = response_from_raw(req.rpc_id, &method, &value, token.clone())?;
    let bytes = Frame::ServerResponse(resp)
        .to_bytes()
        .map_err(|e| format!("v3 encode: {e}"))?;
    Ok((bytes, token))
}

/// 下行:JSON-RPC notification(method + params)→ v3 `ServerNotification` 帧。
///
/// mux 的下行源是 `RpcMessage::ServerRequest`(method + serde payload);v3
/// 模式连接消费本函数产出的 binary 帧(spec r1905:seq 语义、resources 不
/// 消耗 seq、approval/question 形态平移)。事件走 wire::Event → XyEvent →
/// v3 两跳映射(两段都已存在,对拍点在领域对象层)。
pub(crate) fn downlink_frame(
    method: &str,
    payload: serde_json::Value,
    fallback_seq: u64,
) -> Result<Vec<u8>, String> {
    use crate::protocol::wire::v3::{
        ApprovalRequested, Notification, QuestionRequested, Resources, ResyncRequired,
        ServerNotification, Subscribed,
    };
    let (seq, notification) = match method {
        "session/event" => {
            let ev: crate::protocol::wire::SessionEventPayload =
                serde_json::from_value(payload).map_err(|e| format!("event payload: {e}"))?;
            let xy: crate::protocol::lifecycle::XyEvent = (&ev.event)
                .try_into()
                .map_err(|_| "wire event -> XyEvent".to_string())?;
            let v3_ev = crate::protocol::wire::v3::mapping::xy_event_to_v3(&xy)
                .ok_or("XyEvent variant has no v3 projection")?;
            (ev.seq, Notification::Event(v3_ev))
        }
        "session/subscribed" => {
            let p: crate::protocol::wire::SessionSubscribedPayload =
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
            let p: crate::protocol::wire::SessionResyncRequiredPayload =
                serde_json::from_value(payload).map_err(|e| format!("resync payload: {e}"))?;
            (
                fallback_seq,
                Notification::ResyncRequired(ResyncRequired {
                    session_id: p.session_id,
                }),
            )
        }
        "session/resources" => {
            let p: crate::protocol::wire::SessionResourcesPayload =
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
            let p: crate::protocol::wire::ApprovalRequestedPayload =
                serde_json::from_value(payload).map_err(|e| format!("approval payload: {e}"))?;
            (
                fallback_seq,
                Notification::ApprovalRequested(ApprovalRequested { call_id: p.call_id }),
            )
        }
        "question/requested" => {
            let p: crate::protocol::wire::QuestionRequestedPayload =
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
                0, // bash 直播不消耗 journal seq(r1703/c2760)
                Notification::BashOutput(crate::protocol::wire::v3::BashOutput {
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

    /// JSON-RPC 应答形状的测试夹具（与 dispatch 产出的 Value 同构）。
    fn json_of(raw: &str) -> serde_json::Value {
        serde_json::from_str(raw).expect("fixture json")
    }

    fn steer_frame(rpc_id: u64) -> Vec<u8> {
        Frame::ClientRequest(ClientRequest {
            rpc_id,
            request: Request::Command(crate::protocol::wire::v3::Command::Steer(
                crate::protocol::wire::v3::Steer {
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
        assert!(looks_like_fory_v3(&[0x01, 0xff, 0x22]));
        assert!(!looks_like_fory_v3(b"{\"jsonrpc\""));
        assert!(!looks_like_fory_v3(b""));
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
            request: Request::Raw(crate::protocol::wire::v3::Raw {
                method: crate::protocol::wire::v3::Method::ArmToolFreeze,
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
        // task 2.5b:会话条目 / 会话树 / travel 走具名 union，其余仍走 RawOk。
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
                let back = crate::protocol::wire::v3::mapping::v3_to_entries(&m).unwrap();
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

        // c2845: `session_tree` 刻意走 RawOk（深度安全）——所返回载荷与 JSON 轨
        // 逐字面同构，MUST NOT 再用递归强 schema TreeResult（深树会爆 worker 栈）。
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
                // 兼容解码面：强 schema 仍可对旧端/旧帧往返（v3_to_tree_nodes）。
                let typed = crate::protocol::wire::v3::mapping::tree_result_to_v3(
                    &serde_json::from_value::<Vec<
                        crate::protocol::session::SessionTreeNode,
                    >>(value.get("tree").cloned().unwrap())
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

        // 未接强 schema 的方法保持 RawOk(JSON 原文)。
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
            &json_of(r#"{"jsonrpc":"2.0","id":1,"result":{"protocol":2,"formats":["jsonrpc","fory-v3"]}}"#),
            None,
        )
        .unwrap();
        match resp.payload.unwrap() {
            ResponsePayload::DescribeResult(d) => {
                assert_eq!(d.protocol, 2);
                assert_eq!(
                    d.formats,
                    vec!["jsonrpc".to_string(), "fory-v3".to_string()]
                );
            }
            _ => panic!("expected DescribeResult"),
        }
    }
}
