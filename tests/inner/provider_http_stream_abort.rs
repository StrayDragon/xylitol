//! PoC / evidence: dropping a reqwest response body stream closes the TCP
//! connection so the server stops writing (foundation for Esc abort → stop billing).
//!
//! Run: `cargo test -p xylitol --test provider_http_stream_abort -- --nocapture`

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn read_http_request(socket: &mut tokio::net::TcpStream) {
    let mut buf = vec![0u8; 4096];
    let mut acc = Vec::new();
    loop {
        let n = socket.read(&mut buf).await.expect("read request");
        if n == 0 {
            break;
        }
        acc.extend_from_slice(&buf[..n]);
        if acc.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
}

/// Slow chunked SSE-ish body. Counts successful writes; sets `disconnected` when
/// a write fails after the client dropped the body.
async fn serve_slow_chunked(
    listener: TcpListener,
    writes: Arc<AtomicUsize>,
    disconnected: Arc<AtomicBool>,
) {
    let (mut socket, _) = listener.accept().await.expect("accept");
    read_http_request(&mut socket).await;

    let headers = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/event-stream\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Connection: close\r\n",
        "\r\n",
    );
    socket.write_all(headers.as_bytes()).await.expect("headers");

    for i in 0..500u32 {
        let data = format!("data: {{\"i\":{i}}}\n\n");
        let chunk = format!("{:x}\r\n{data}\r\n", data.len());
        match socket.write_all(chunk.as_bytes()).await {
            Ok(()) => {
                writes.fetch_add(1, Ordering::SeqCst);
                let _ = socket.flush().await;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Err(_) => {
                disconnected.store(true, Ordering::SeqCst);
                return;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_reqwest_bytes_stream_stops_server_writes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let writes = Arc::new(AtomicUsize::new(0));
    let disconnected = Arc::new(AtomicBool::new(false));

    let w = writes.clone();
    let d = disconnected.clone();
    tokio::spawn(async move {
        serve_slow_chunked(listener, w, d).await;
    });

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/stream"))
        .send()
        .await
        .expect("client send");
    assert!(response.status().is_success());

    let mut stream = response.bytes_stream();
    let first = stream.next().await.expect("first chunk").expect("ok bytes");
    assert!(!first.is_empty(), "expected at least one body byte");

    let writes_at_drop = writes.load(Ordering::SeqCst);
    drop(stream);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if disconnected.load(Ordering::SeqCst) {
            break;
        }
        if writes.load(Ordering::SeqCst) > writes_at_drop + 30 {
            // Still pumping long after drop — cancel did not reach the peer.
            panic!(
                "server kept writing after client drop: at_drop={writes_at_drop} now={} disconnected={}",
                writes.load(Ordering::SeqCst),
                disconnected.load(Ordering::SeqCst)
            );
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    assert!(
        disconnected.load(Ordering::SeqCst),
        "server MUST observe client disconnect after drop (writes_at_drop={writes_at_drop}, now={})",
        writes.load(Ordering::SeqCst)
    );
    let final_writes = writes.load(Ordering::SeqCst);
    assert!(
        final_writes < writes_at_drop + 25,
        "writes must stall soon after drop: at_drop={writes_at_drop} final={final_writes}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anthropic_adapter_drop_stream_stops_server_writes() {
    use crate::infra::provider::adapter::AdapterXyModel;
    use crate::protocol::ports::XyModel;
    use xylitol_ai_bridge::provider::AnthropicMessagesAdapter as BridgeAnthropic;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let writes = Arc::new(AtomicUsize::new(0));
    let disconnected = Arc::new(AtomicBool::new(false));

    let w = writes.clone();
    let d = disconnected.clone();
    tokio::spawn(async move {
        serve_slow_chunked(listener, w, d).await;
    });

    let adapter = AdapterXyModel::new(Arc::new(BridgeAnthropic::new(
        "test-key".into(),
        "claude-test".into(),
        Some(format!("http://{addr}")),
        None,
    )));

    // Anthropic path POSTs /v1/messages; our stub ignores method/path and streams.
    let mut stream = XyModel::generate_stream(&adapter, vec![], &[], true, Default::default())
        .await
        .expect("adapter stream");

    // Drain until we get a parseable event or a few byte polls — server keeps
    // writing SSE JSON; adapter may skip bad events. Force progress by awaiting
    // with timeout then drop regardless.
    let _ = tokio::time::timeout(Duration::from_millis(200), stream.next()).await;

    let writes_at_drop = writes.load(Ordering::SeqCst);
    drop(stream);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if disconnected.load(Ordering::SeqCst) {
            break;
        }
        if writes.load(Ordering::SeqCst) > writes_at_drop + 30 {
            panic!(
                "adapter drop did not stop server: at_drop={writes_at_drop} now={}",
                writes.load(Ordering::SeqCst)
            );
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    assert!(
        disconnected.load(Ordering::SeqCst),
        "dropping Anthropic XyStream MUST close HTTP body (writes_at_drop={writes_at_drop}, now={})",
        writes.load(Ordering::SeqCst)
    );
}
