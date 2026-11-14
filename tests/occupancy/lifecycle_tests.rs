use std::time::Duration;

use tokio_tungstenite::connect_async;
use xylitol_kernel::occupancy::host_state_for_test;
use xylitol_kernel::occupancy::runtime::{ServerConfig, serve_registered};
use xylitol_serve::ServeLifecycle;

fn temp_reg() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("serve.json");
    (dir, path)
}

#[tokio::test]
async fn last_client_exits_after_idle_and_removes_registration() {
    let host = host_state_for_test().expect("host");
    let (_dir, path) = temp_reg();
    let (running, port) = serve_registered(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: Some(path.clone()),
            lifecycle: ServeLifecycle::LastClient,
            idle_timeout: Duration::from_millis(300),
            ws_keepalive: Some(Duration::from_secs(20)),
        },
        host,
        path.clone(),
    )
    .await
    .expect("serve");
    assert!(path.exists());
    drop(open_ws(port).await);
    tokio::time::timeout(Duration::from_millis(1500), running.last_client_idle())
        .await
        .expect("idle");
    running.shutdown();
    drop(running);
    assert!(!path.exists(), "registration must be removed");
}

#[tokio::test]
async fn last_client_reconnect_within_grace_stays_up() {
    let host = host_state_for_test().expect("host");
    let (_dir, path) = temp_reg();
    let (running, port) = serve_registered(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: Some(path.clone()),
            lifecycle: ServeLifecycle::LastClient,
            idle_timeout: Duration::from_millis(250),
            ws_keepalive: Some(Duration::from_secs(20)),
        },
        host,
        path.clone(),
    )
    .await
    .expect("serve");
    drop(open_ws(port).await);
    tokio::time::sleep(Duration::from_millis(40)).await;
    let ws = open_ws(port).await;
    let idle = tokio::time::timeout(Duration::from_millis(500), running.last_client_idle()).await;
    assert!(idle.is_err(), "reconnect must cancel idle");
    drop(ws);
    running.shutdown();
}

#[tokio::test]
async fn persistent_survives_zero_clients() {
    let host = host_state_for_test().expect("host");
    let (_dir, path) = temp_reg();
    let (running, port) = serve_registered(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: Some(path.clone()),
            lifecycle: ServeLifecycle::Persistent,
            idle_timeout: Duration::from_millis(40),
            ws_keepalive: Some(Duration::from_secs(20)),
        },
        host,
        path.clone(),
    )
    .await
    .expect("serve");
    drop(open_ws(port).await);
    let idle = tokio::time::timeout(Duration::from_millis(200), running.last_client_idle()).await;
    assert!(idle.is_err(), "persistent must not idle");
    let (st, _) = healthz(port).await;
    assert_eq!(st, 200);
    running.shutdown();
}

#[tokio::test]
async fn unary_post_does_not_count_as_client() {
    let host = host_state_for_test().expect("host");
    let (_dir, path) = temp_reg();
    let (running, port) = serve_registered(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: Some(path.clone()),
            lifecycle: ServeLifecycle::LastClient,
            idle_timeout: Duration::from_millis(120),
            ws_keepalive: Some(Duration::from_secs(20)),
        },
        host,
        path.clone(),
    )
    .await
    .expect("serve");
    let _ = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/rpc"))
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "host.describe",
            "params": {}
        }))
        .send()
        .await
        .expect("unary");
    let idle = tokio::time::timeout(Duration::from_millis(400), running.last_client_idle()).await;
    assert!(idle.is_err(), "unary must not start last-client idle");
    let (st, _) = healthz(port).await;
    assert_eq!(st, 200);
    running.shutdown();
}

#[tokio::test]
async fn last_client_ws_open_is_not_idle() {
    let host = host_state_for_test().expect("host");
    let (_dir, path) = temp_reg();
    let (running, port) = serve_registered(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: Some(path.clone()),
            lifecycle: ServeLifecycle::LastClient,
            idle_timeout: Duration::from_millis(80),
            ws_keepalive: Some(Duration::from_secs(20)),
        },
        host,
        path.clone(),
    )
    .await
    .expect("serve");
    let idle = running.last_client_idle();
    tokio::pin!(idle);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), idle.as_mut())
            .await
            .is_err(),
        "must still be waiting before any WS"
    );
    let ws = open_ws(port).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), idle.as_mut())
            .await
            .is_err(),
        "opening a WS must not look like last-client idle"
    );
    let (st, _) = healthz(port).await;
    assert_eq!(st, 200);
    drop(ws);
    running.shutdown();
}

#[tokio::test]
async fn last_client_never_connected_stays_up() {
    let host = host_state_for_test().expect("host");
    let (_dir, path) = temp_reg();
    let (running, port) = serve_registered(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: Some(path.clone()),
            lifecycle: ServeLifecycle::LastClient,
            idle_timeout: Duration::from_millis(120),
            ws_keepalive: Some(Duration::from_secs(20)),
        },
        host,
        path.clone(),
    )
    .await
    .expect("serve");
    let idle = tokio::time::timeout(Duration::from_millis(400), running.last_client_idle()).await;
    assert!(idle.is_err(), "boot with zero WS must not idle");
    let (st, _) = healthz(port).await;
    assert_eq!(st, 200);
    running.shutdown();
}

async fn open_ws(
    port: u16,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let url = format!("ws://127.0.0.1:{port}/rpc");
    let (ws, _) = connect_async(url).await.expect("ws");
    ws
}

async fn healthz(port: u16) -> (u16, String) {
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/healthz"))
        .send()
        .await
        .expect("healthz");
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    (status, body)
}
