//! wire v3 client-side codec helpers (c2834 tasks 3.1/3.3)。
//!
//! 纯编码 / 解码层,无传输:给 [`super::HttpWsClient`] 用的 fory v3 上行帧
//! 构造与下行帧 → `RpcMessage` 还原。下行还原以 JSON 路径的 reader 产出为
//! 准(`session/event` 等 payload 与 `codec::decode_str` 的通知形状同构,
//! 下游消费零改动)。产品默认开；[`env_enabled`] 只解析显式 env（测试注入）。

use serde_json::Value;

use crate::protocol::RpcMessage;
use crate::protocol::RpcResult;
use crate::protocol::wire::v3::mapping as v3_mapping;
use crate::protocol::wire::v3::{
    self, ClientRequest, Describe, Frame, Notification, Raw, Request, ResponsePayload,
    ServerNotification, ServerResponse,
};
use crate::protocol::wire::{
    ApprovalRequestedPayload, HostDescribeValue, QuestionRequestedPayload, SessionEventPayload,
    SessionResourcesPayload, SessionResyncRequiredPayload, SessionSubscribedPayload,
    WIRE_FORMAT_FORY_V3, registry,
};

/// Test injection env. Unset is treated as on by the attach client.
pub(super) const WIRE_V3_ENV: &str = "XYLITOL_WIRE_V3";

/// 读进程 env 决定 v3 是否开启。
///
/// 未设 `XYLITOL_WIRE_V3` 时产品默认开；显式 env 仍走本解析。
pub(super) fn env_enabled() -> bool {
    flag_truthy(std::env::var_os(WIRE_V3_ENV).as_deref())
}

/// 纯解析(测试不碰进程 env):`"1"` / `"true"`(大小写不敏感、容忍空白)为真。
fn flag_truthy(raw: Option<&std::ffi::OsStr>) -> bool {
    let Some(raw) = raw else { return false };
    let normalized = raw.to_string_lossy().trim().to_ascii_lowercase();
    normalized == "1" || normalized == "true"
}

/// 信封 rpcId(字符串)→ v3 数值 `rpc_id`。
///
/// 数字形态直通;其余(`unary` 生成的 UUID 等)经 `DefaultHasher::new()`
/// (零 key SipHash,跨调用 / 跨进程稳定)做稳定哈希映射——幂等语义
/// (c2460)要求同一 rpcId 字符串复用同一 `rpc_id`,host 才能对重放取
/// 首次结果;64 位空间下不同非数字 id 并发碰撞概率 ~2⁻⁶⁴,实验轨道接受。
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

/// 产品方法名 → `Method` 判别值(`method_name` 正表的客户端侧反查)。
fn method_discriminant(method: &str) -> Option<v3::Method> {
    v3::METHOD_NAMES
        .iter()
        .find(|(_, name)| *name == method)
        .map(|(m, _)| m.clone())
}

/// unary(method, params)→ v3 上行 `ClientRequest`。
///
/// - `host.describe` → [`Request::Describe`](承载格式能力协商);
/// - 其余全部方法 → [`Request::Raw`],payload JSON 原文过线(c2842)。
///
/// v3 上行是**透明信封**:除 describe 外一律 RAW 原文过线,与 JSON 轨
/// (params 透传)逐字节同语义。原因:remote driver 的 `unary` 经
/// `with_session` 给每个命令注入 `session_id`/`cwd` 及 run 上下文字段
/// (model_id / thinking_level),typed `Command` schema 只载各命令业务字段
/// 而不含这些注入字段——压缩进 typed 变体会把命令从已订阅会话上脱锚,
/// 服务端回落 fallback session(无订阅者):run 事件丢失、TUI 挂死;
/// set_model/get_state/steer 等打错槽(writer_conflict / 状态侧写)。
/// JSON 轨 params 透传一直正确,RAW 使 v3 恢复「纯编码层、语义不变」契约。
/// 服务端仍保留 typed `Command` 解码(v3_to_command)以兼容旧端 / Flutter
/// POC 等仍发 typed 帧的对端。
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

/// v3 上行帧字节(WS binary 帧)。
pub fn client_request_bytes(
    rpc_id: &str,
    method: &str,
    payload: &Value,
    writer_token: Option<String>,
) -> Result<Vec<u8>, String> {
    let frame = Frame::ClientRequest(ClientRequest {
        rpc_id: stable_rpc_id(rpc_id),
        request: build_request(method, payload)?,
        writer_token,
    });
    frame.to_bytes().map_err(|e| format!("v3 encode: {e}"))
}

/// v3 协商用的 describe 帧字节(rpc_id 固定 0:协商发生在连接私有期,
/// pending 表此刻无并发写者,专属 id 不与业务请求碰撞)。
pub(super) fn describe_request_bytes() -> Result<Vec<u8>, String> {
    let frame = Frame::ClientRequest(ClientRequest {
        rpc_id: 0,
        request: Request::Describe(Describe {}),
        writer_token: None,
    });
    frame.to_bytes().map_err(|e| format!("v3 encode: {e}"))
}

/// describe 应答是否宣告 `fory-v3`(未宣告 / 旧 host 无 formats = 否)。
pub(super) fn formats_advertise_v3(result: &RpcResult) -> bool {
    let Ok(describe) =
        serde_json::from_value::<HostDescribeValue>(result.value.clone().unwrap_or(Value::Null))
    else {
        return false;
    };
    describe.formats.iter().any(|f| f == WIRE_FORMAT_FORY_V3)
}

/// reader 侧一条 binary 帧的解析产物。
pub(super) enum Downlink {
    /// unary 应答:按 `rpc_id`(数字形态)回填 pending。
    Response { rpc_id: u64, result: RpcResult },
    /// 下行通知:与 JSON 路径 reader 产出同构的 `ServerRequest`。
    Notification(RpcMessage),
}

/// WS binary 帧 → [`Downlink`]。
///
/// `session_id` 是 Event payload 的会话归属:v3 `ServerNotification` 不携带
/// session_id,由调用方在 `Subscribed` 帧上跟踪(reader loop 本地状态,
/// 首个 Subscribed 之前为空串——事件实际总在 subscribe 之后流动)。
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
        // 服务端不会上行 ClientRequest;schema-ahead 帧降级为忽略。
        Frame::ClientRequest(_) | Frame::Unknown(_) => None,
    })
}

/// v3 `ServerResponse` → `RpcResult`(对齐 JSON 路径的应答语义)。
pub fn server_response_to_result(resp: &ServerResponse) -> RpcResult {
    let mut result = if resp.ok {
        RpcResult::ok_value(payload_value(resp.payload.as_ref()))
    } else {
        let err = resp.error.clone().unwrap_or_default();
        RpcResult::error(err.code, err.details)
    };
    result.writer_token = resp.writer_token.clone();
    result
}

/// 成功载荷 → JSON value(`RawOk` 原文解析;describe 还原为
/// `HostDescribeValue` 形状——与 JSON 轨 describe result 同构)。
///
/// c2846: 深度安全地解析 RawOk 原文。
///
/// `session_tree` 深树 JSON 可超过 serde_json 默认 **128 层**递归解析上限
/// （实机深会话 188 层），默认 `from_str` 会 `recursion limit exceeded` 并被
/// 调用方静默降级为 `Null`（下游再报 `invalid type: null`）。服务端序列化
/// 无此上限（仅在反序列化侧），故对 RAW 原文禁用默认递归上限，但 c2851/r1927
/// 加显式深度闸（与 codec 共用 `JSON_DEPTH_LIMIT`）——超限仍按 r1907 退化
/// `Null`，不拼半份形状、不进无界递归下降。
fn parse_raw_json(json: &str) -> Value {
    use serde::Deserialize as _;
    if crate::protocol::wire::codec::json_nesting_exceeds_limit(json) {
        return Value::Null;
    }
    let mut de = serde_json::Deserializer::from_str(json);
    de.disable_recursion_limit();
    Value::deserialize(&mut de).unwrap_or(Value::Null)
}

/// 强 schema 应答(task 2.5b)以领域条目为中间物回到 serde,因此与
/// `host::outcome_to_value` 的 JSON 形状逐字节同构(同一组 Serialize)。
/// unknown 变体(r1907)使整份载荷降级 `Null`,不拼半份形状。
fn payload_value(payload: Option<&ResponsePayload>) -> Value {
    match payload {
        Some(ResponsePayload::DescribeResult(d)) => serde_json::to_value(HostDescribeValue {
            protocol: d.protocol,
            formats: d.formats.clone(),
        })
        .unwrap_or(Value::Null),
        // 会话条目与树:强 schema union 还原为 `{"entries": […]}` / `{"tree": […]}`。
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
        // travel 结果是平对象(闭集枚举),与 JSON 轨 `to_value(travel)` 一致。
        Some(ResponsePayload::TravelResult(t)) => {
            serde_json::to_value(v3_mapping::v3_to_travel_result(t)).unwrap_or(Value::Null)
        }
        // 其余未接强 schema 的方法一律 RawOk(JSON 原文)。
        Some(ResponsePayload::RawOk(raw)) => parse_raw_json(&raw.json),
        // 防御:SubscribeResult 与 JSON 轨 subscribe result 同构，不断链。
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

/// v3 `ServerNotification` → JSON 路径同构的 `RpcMessage::ServerRequest`
/// (rpc_id 空、method / payload 与 `codec::decode_str` 的通知产物一致)。
/// 不可映射(Union Unknown、降级事件)返回 `None`,调用方跳过该帧。
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
        // schema-ahead union member(更新对端):降级跳过,不断链。
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
    use crate::protocol::wire::v3::{
        ApprovalRequested, DescribeResult, Event as V3Event, ResyncRequired, ServerNotification,
        Subscribed, TextDelta as V3TextDelta, ToolStart as V3ToolStart,
    };
    use serde_json::{Value, json};

    fn decode_frame(bytes: &[u8]) -> Frame {
        Frame::from_bytes(bytes).expect("roundtrip decode")
    }

    /// task 3.3:开关纯解析——未设 / 乱值为关,仅 "1"/"true" 开(库默认;
    /// 产品面切换在 TUI attach 入口,见 task 5.1 备注)。
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
    }

    /// task 3.1 (b):rpc_id 数字形态直通、非数字形态稳定映射
    /// (同一 rpcId 两次映射一致 = 幂等语义可复用)。
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

    /// task 3.1 (b):describe / RAW / Command 三族请求形状。
    #[test]
    fn request_family_shapes() {
        let bytes = client_request_bytes("7", "host.describe", &json!({}), None).unwrap();
        let Frame::ClientRequest(req) = decode_frame(&bytes) else {
            panic!("expected ClientRequest")
        };
        assert_eq!(req.rpc_id, 7);
        assert!(matches!(req.request, Request::Describe(_)));

        let bytes =
            client_request_bytes("8", "persist_trust", &json!({"mode": "project"}), None).unwrap();
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

        // 手搓载荷(无 type tag)经 tag 注入仍可构帧;subscribe 亦走 RAW(c2842),
        // 注入的 cwd 必须保留(typed Subscribe 无 cwd 会让服务端在错误 workspace
        // 装配写者)。
        let bytes = client_request_bytes(
            "9",
            "subscribe",
            &json!({"session_id": "s1", "last_seq": 3, "cwd": "/tmp"}),
            Some("writer-1".into()),
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
        assert_eq!(req.writer_token.as_deref(), Some("writer-1"));

        // 回归锁：prompt 必须走 RAW 原文(c2842 起全部命令走 RAW,字段全程保真;
        // 早先 typed Command::Prompt 只载 message 会把 run 从已订阅 session
        // 脱锚到 fallback,事件 broadcast 丢失、TUI 挂死)。
        let bytes = client_request_bytes(
            "10",
            "prompt",
            &json!({"message": "hi", "session_id": "sess-1", "cwd": "/tmp", "model_id": "m1"}),
            None,
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

        // 非 registry、非白名单方法无 v3 映射(显式失败,不静默错发)。
        assert!(client_request_bytes("1", "no_such_method", &json!({}), None).is_err());
    }

    /// task 3.1 (b):approve_tool(非 registry 命令)亦走 RAW 原文过线。
    #[test]
    fn non_registry_command_maps() {
        let bytes = client_request_bytes(
            "5",
            "approve_tool",
            &json!({"call_id": "c1", "approved": true}),
            None,
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

    /// task 3.1 (b):ServerResponse → RpcResult(ok / error / writer_token /
    /// RawOk 与 DescribeResult 的 value 形状)。
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
        assert_eq!(result.writer_token.as_deref(), Some("lease-9"));

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
        assert!(formats_advertise_v3(&result));

        let err = ServerResponse {
            rpc_id: 6,
            ok: false,
            error: Some(v3::RpcError {
                code: "writer_conflict".into(),
                details: "stale lease".into(),
            }),
            payload: None,
            writer_token: None,
        };
        let result = server_response_to_result(&err);
        assert!(!result.ok);
        assert_eq!(
            result.error.map(|e| (e.code, e.details)),
            Some(("writer_conflict".into(), "stale lease".into()))
        );
    }

    /// c2846: RawOk 原文解析禁用递归上限——深树 JSON（实机 188 层 > 默认 128）
    /// 必须还原为完整 Value，不得被静默降级 Null（旧行为让下游报
    /// `invalid type: null, expected a sequence`）。
    #[test]
    fn raw_ok_parses_beyond_default_recursion_limit() {
        // 构造 150 层（>128）嵌套 JSON，纯字符串拼接避免夹具自身触发默认上限。
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

    /// c2851/r1927: 超过共享深度上限的 RawOk 按 r1907 退化 Null，不进无界递归。
    #[test]
    fn raw_ok_over_json_depth_limit_degrades_to_null() {
        use crate::protocol::wire::codec::JSON_DEPTH_LIMIT;
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

    /// 协商门:旧 host(formats 缺失)与未宣告 fory-v3 都必须判否。
    #[test]
    fn negotiate_gate_rejects_without_fory_v3() {
        let jsonrpc_only = RpcResult::ok_value(json!({"protocol": 2, "formats": ["jsonrpc"]}));
        assert!(!formats_advertise_v3(&jsonrpc_only));
        let old_host = RpcResult::ok_value(json!({"protocol": 2}));
        assert!(!formats_advertise_v3(&old_host));
        let dual = RpcResult::ok_value(json!({"protocol": 2, "formats": ["jsonrpc", "fory-v3"]}));
        assert!(formats_advertise_v3(&dual));
    }

    /// task 2.5b：具名应答 union 还原为与 JSON 轨 `outcome_to_value` 同构的形状
    /// (条目/树多一层包裹键,travel 是平对象)。
    #[test]
    fn typed_payloads_restore_json_track_shape() {
        use crate::protocol::session::{
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

    /// task 3.1 (b):ServerNotification(Event / Subscribed)→ 与 JSON 路径
    /// reader 同构的 `RpcMessage::ServerRequest` 形状。
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

        // Subscribed 之后的事件携带上跟踪到的 session_id。
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

    /// 其余通知变体同样落到 JSON 路径形状;Unknown 降级跳过。
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

        let unknown = ServerNotification {
            seq: 2,
            notification: Notification::Unknown(fory::UnknownCase::new(99, ())),
        };
        assert!(notification_to_message(&unknown, &mut session_id).is_none());
    }

    /// decode_downlink 对 ClientRequest / Unknown 上行帧忽略(None)。
    #[test]
    fn decode_downlink_ignores_non_server_frames() {
        let mut session_id = String::new();
        let bytes = client_request_bytes("1", "host.describe", &json!({}), None).unwrap();
        assert!(
            decode_downlink(&bytes, &mut session_id)
                .expect("decode")
                .is_none()
        );
    }
}
