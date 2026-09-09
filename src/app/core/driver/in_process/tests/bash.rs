//! Interactive bash run relay (c2760 follow-up): chunk → event UTF-8
//! reassembly and the out-of-band sink cancel.

use super::*;

#[test]
fn utf8_emit_len_complete_input() {
    assert_eq!(super::utf8_emit_len(b"abc"), 3);
    assert_eq!(super::utf8_emit_len("你好".as_bytes()), 6);
}

#[test]
fn utf8_emit_len_holds_back_split_multibyte() {
    // "你" = E4 B8 AD, split mid-sequence.
    assert_eq!(super::utf8_emit_len(&[0xE4]), 0);
    assert_eq!(super::utf8_emit_len(&[0xE4, 0xB8]), 0);
    // A valid prefix before the truncated tail still emits.
    assert_eq!(super::utf8_emit_len(&[b'a', 0xE4, 0xB8]), 1);
}

#[test]
fn utf8_emit_len_emits_invalid_bytes_now() {
    // A lone continuation byte is invalid, not a truncated lead.
    assert_eq!(super::utf8_emit_len(&[0x80]), 1);
    assert_eq!(super::utf8_emit_len(&[b'x', 0xFF, b'y']), 2);
}

#[tokio::test]
async fn relay_reassembles_split_utf8_across_chunks() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel(8);
    tokio::spawn(super::relay_bash_chunks("b1".into(), rx, out_tx));
    let text: &[u8] = "你好!".as_bytes();
    tx.send(text[..2].to_vec()).await.unwrap();
    tx.send(text[2..5].to_vec()).await.unwrap();
    tx.send(text[5..].to_vec()).await.unwrap();
    drop(tx);
    let mut joined = String::new();
    while let Some(chunk) = out_rx.recv().await {
        assert_eq!(chunk.bash_id, "b1");
        joined.push_str(&chunk.data);
    }
    assert_eq!(joined, "你好!");
}

#[tokio::test]
async fn relay_flushes_truncated_tail_at_stream_end() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel(8);
    tokio::spawn(super::relay_bash_chunks("b1".into(), rx, out_tx));
    tx.send(vec![0xE4]).await.unwrap();
    drop(tx);
    let chunk = out_rx.recv().await.expect("flushed chunk");
    assert!(chunk.data.contains('\u{FFFD}'), "got {:?}", chunk.data);
}

#[tokio::test]
async fn relay_replaces_invalid_bytes_without_blocking() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel(8);
    tokio::spawn(super::relay_bash_chunks("b1".into(), rx, out_tx));
    tx.send(vec![b'a', 0xFF, b'b']).await.unwrap();
    drop(tx);
    let mut joined = String::new();
    while let Some(chunk) = out_rx.recv().await {
        joined.push_str(&chunk.data);
    }
    assert_eq!(joined, "a\u{FFFD}b");
}

#[tokio::test]
async fn sink_cancel_reaches_execute_bash_out_of_band() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _scope) = build_test_driver(store).await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let cancel = tokio_util::sync::CancellationToken::new();
    driver.set_bash_run_sink(Some(crate::protocol::ports::BashOutputSink {
        tx,
        cancel: cancel.clone(),
    }));
    // Cancel before dispatch: stands in for host `abort` racing the running
    // bash unary (which holds the writer lock, so dispatch cannot reach it).
    cancel.cancel();
    let result = driver
        .execute_bash("sleep 30", false)
        .await
        .expect("bash result");
    assert!(result.cancelled, "sink cancel must kill the run");
}
