//! 产品 JSON-RPC 2.0 分发（手写载体；方法表 SSOT 见 [`crate::protocol::wire::registry`]）。
//!
//! Salvo 拥有监听器与 HTTP 周边，[`handle_unary`] 是唯一执行面；本模块只做
//! 「JSON 文本 ↔ 方法表」这一层：解析 → 命中 → 应答信封。幂等键取信封 `id`，
//! 会话写占用是内核锁，不经协议令牌；MUST NOT 把 writerToken 放进 `params`。
//!
//! 等价判据（design D2，逐条有单测）：① `id` 逐字回显（保留原 JSON 类型）
//! ② 未登记方法 `-32601` + `data.code = unregistered_method` ③ 非法信封判否
//! （含 JSON-RPC 数组 batch，r1928）④ 请求体 4 MiB 上限。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::app::server::host::{HostState, handle_unary};
use crate::protocol::wire::codec;
use crate::protocol::wire::envelope::RpcResult;
use crate::protocol::wire::registry;

/// 请求体上限（信封解析前的硬闸）。
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// 反向 RPC 应答方法：不在 registry 方法表内，直接落网关待答表。
const REVERSE_METHODS: [&str; 2] = ["approve_tool", "answer_question"];

/// 幂等准入用的 id 文本形态：字符串原样、数字取十进制；无 id（通知）为 `None`。
fn id_key(id: &Value) -> Option<String> {
    match id {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// 一次 JSON-RPC 2.0 unary 往返（POST body 与 WS 文本帧共用）。
///
/// 返回 `None` = 非法信封（调用方按 400 处理，语义同 r1696）；
/// `Some((应答, ()))` 里应答是完整 JSON-RPC 对象（`result` 或 `error`）。
/// 第二项恒为 `None`：占用改内核锁后不再 mint 协议令牌。
/// 未登记方法不判非法，而是给 `-32601` 应答（合法信封 + 产品码）。
pub(crate) async fn dispatch_raw(
    host: &Arc<HostState>,
    request: &str,
) -> Option<(Value, Option<String>)> {
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
        // 反向应答：`call_id` 优先，缺位回落本次 id（与网关待答表同键）。
        let call_id = payload
            .get("call_id")
            .or_else(|| payload.get("id"))
            .and_then(Value::as_str)
            .or_else(|| id.as_str())
            .unwrap_or_default()
            .to_string();
        let answered = host.respond(&call_id, payload).await;
        // 锁定语义（protocol-app `approve-tool-is-product-unary`）：反向应答 MUST 有
        // result，未命中待答表也是已受理的空结果（不拆第二套错误模型）。
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 一次 unary 的最小夹具：每次新建 host，租约状态互不干扰。
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

    /// 等价判据 ①：id 逐字回显——数字不进字符串，字符串不进数字。
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

    /// 等价判据 ③：非法信封判否（缺 method / 载体版本不符 / 非 JSON / 数组 batch）。
    #[tokio::test]
    async fn illegal_envelopes_are_rejected() {
        let host = host();
        for raw in [
            r#"{"jsonrpc":"2.0","id":"1"}"#,
            r#"{"jsonrpc":"1.0","id":"1","method":"host.describe"}"#,
            "not json",
            r#"{"type":"client-request","rpcId":"1","method":"host.describe"}"#,
            // r1928: JSON-RPC 2.0 batch 数组——幂等键/写者租约语义未定义，MUST 判非法信封。
            r#"[{"jsonrpc":"2.0","id":"1","method":"host.describe"},{"jsonrpc":"2.0","id":"2","method":"host.describe"}]"#,
        ] {
            assert!(
                dispatch_raw(&host, raw).await.is_none(),
                "非法信封 MUST 判否: {raw}"
            );
        }
    }

    /// 等价判据 ④：超过 4 MiB 的请求体判否（不进 dispatch）。
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

    /// 无 params 的请求按空对象处理（幂等键与写者准入都看得到 id）。
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

    /// Same-process writer methods share occupancy; responses carry no token.
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

    /// 反向 RPC 应答：与 JSON 轨同一形态（result 为 `{}`，不占写者租约）。
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
