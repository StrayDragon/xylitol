//! c2846 探针 v2：精确复刻 remote driver 的 unary_cmd（serde 序列化 Command），
//! 打 live serve 观察 session_tree 真实应答。用法：
//! `cargo run --example lab_probe_session_tree -- [session_id]`（不带 id 走全局活动会话）
use xylitol::app::core::host_client::{HostClient, HttpWsClient};
use xylitol::protocol::Command;
use xylitol::protocol::session::SessionTreeKind;

#[tokio::main]
async fn main() {
    let sid = std::env::args().nth(1).unwrap_or_default();
    let url = "http://127.0.0.1:18790";
    let client = HttpWsClient::new(url).with_wire_v3(true);

    for kind in [
        SessionTreeKind::MessageHistory,
        SessionTreeKind::FileBrowser,
    ] {
        let mut cmd = Command::SessionTree { kind: kind.clone() };
        if !sid.is_empty() {
            cmd = Command::SessionTree { kind: kind.clone() };
        }
        let payload = serde_json::to_value(&cmd).expect("serialize command");
        let mut payload = payload;
        if payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(str::is_empty)
            .unwrap_or(true)
            && !sid.is_empty()
        {
            payload["session_id"] = serde_json::json!(sid);
        }
        if payload.get("cwd").is_none() {
            if let Ok(c) = std::env::current_dir() {
                payload["cwd"] = serde_json::json!(c.to_string_lossy());
            }
        }
        eprintln!("[probe] kind={kind:?} payload={payload}");
        let r = client
            .unary("session_tree", payload)
            .await
            .expect("unary session_tree");
        eprintln!("[probe] ok={} err={:?}", r.ok, r.error.map(|e| e.details));
        let tree_len = r
            .value
            .as_ref()
            .and_then(|v| v.get("tree"))
            .and_then(|v| v.as_array())
            .map(|a| a.len());
        eprintln!("[probe] tree[].len={tree_len:?}");
        if let Some(v) = &r.value {
            let s = serde_json::to_string(v).unwrap_or_default();
            eprintln!("[probe] value head: {}", &s[..s.len().min(200)]);
        }
    }
}
