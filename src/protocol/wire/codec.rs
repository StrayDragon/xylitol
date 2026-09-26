//! Product wire bytes for the four-quadrant envelope.
//!
//! Today this is `serde_json` of [`RpcMessage`]. Dual-read JSON-RPC (research
//! 2026-09, `docs/research/agent-interop-surface-2026.md`) inserts **here**,
//! not in TUI or dispatch. Do not add a dialect enum until `protocol-app`
//! allows JSON-RPC 2.0 on the product path.

use super::envelope::RpcMessage;

pub fn encode(msg: &RpcMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(msg)
}

pub fn encode_to_string(msg: &RpcMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

pub fn decode(bytes: &[u8]) -> Result<RpcMessage, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn decode_str(text: &str) -> Result<RpcMessage, serde_json::Error> {
    serde_json::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::RpcMessage;
    use serde_json::json;

    #[test]
    fn four_quadrant_json_roundtrip() {
        let msg = RpcMessage::ClientRequest {
            rpc_id: "r1".into(),
            method: "prompt".into(),
            payload: json!({"message": "hi"}),
            writer_token: None,
        };
        let bytes = encode(&msg).expect("encode");
        let text = std::str::from_utf8(&bytes).expect("utf8");
        assert!(text.contains("\"type\":\"client-request\""), "{text}");
        assert!(!text.contains("jsonrpc"), "{text}");
        assert_eq!(decode(&bytes).expect("decode"), msg);
        assert_eq!(
            decode_str(&encode_to_string(&msg).expect("string")).expect("str"),
            msg
        );
    }
}
