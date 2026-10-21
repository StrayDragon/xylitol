use futures::StreamExt;
use xylitol_app_tui::host_client::{HostClient, HttpWsClient};

pub async fn dual_rail_event_parity()
-> Result<Vec<(xylitol_protocol::Event, xylitol_protocol::Event)>, String> {
    let host =
        xylitol_kernel::occupancy::host_state_for_test().map_err(|e| format!("host: {e}"))?;
    let (_running, port) = xylitol_kernel::occupancy::runtime::serve(
        xylitol_kernel::occupancy::runtime::ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host.clone(),
    )
    .await
    .map_err(|e| format!("serve: {e}"))?;
    let url = format!("http://127.0.0.1:{port}");

    let client_v3 = HttpWsClient::new(url).with_envelope(true);
    let mut mux_v3 = client_v3.mux().await.map_err(|e| format!("v3 mux: {e}"))?;

    let d = client_v3
        .unary("host.describe", serde_json::json!({}))
        .await
        .map_err(|e| format!("v3 describe: {e}"))?;
    if !d.ok {
        return Err(format!("v3 describe failed: {:?}", d.error));
    }

    let session = "s-parity";
    let r = client_v3
        .unary(
            "subscribe",
            serde_json::json!({"session_id": session, "last_seq": 0}),
        )
        .await
        .map_err(|e| format!("subscribe: {e}"))?;
    if !r.ok {
        return Err(format!("subscribe failed: {:?}", r.error));
    }

    let injected = vec![
        xylitol_protocol::Event::TextDelta {
            text: "parity chunk".into(),
        },
        xylitol_protocol::Event::ToolStart {
            id: "t-parity".into(),
            name: "grep".into(),
            args: serde_json::json!({"pattern": "TODO"}),
        },
        xylitol_protocol::Event::TurnEnd { turn_index: 1 },
    ];
    for ev in &injected {
        host.slot(session).await.append_and_push(ev.clone()).await;
    }

    let mut got_v3 = Vec::new();
    while let Ok(Some(Ok(frame))) =
        tokio::time::timeout(std::time::Duration::from_secs(3), mux_v3.next()).await
    {
        if let xylitol_protocol::RpcMessage::ServerRequest {
            method, payload, ..
        } = &frame
            && method == "session/event"
            && let Some(ev) = payload
                .get("event")
                .cloned()
                .and_then(|v| serde_json::from_value::<xylitol_protocol::Event>(v).ok())
        {
            got_v3.push(ev);
            if got_v3.len() >= injected.len() {
                break;
            }
        }
    }
    if got_v3.len() != injected.len() {
        return Err(format!(
            "v3 path collected {} events: {got_v3:?}",
            got_v3.len()
        ));
    }
    Ok(injected.into_iter().zip(got_v3).collect())
}
