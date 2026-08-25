//! OpenAPI 3.1 debug document for the unary surface (sr-oapi1).
//!
//! Generated from the wire method table — never a hand-written second
//! vocabulary. Schemas stay envelope-level; concrete payload shapes' SSOT is
//! the Rust protocol types (`crate::protocol::wire`). This document is
//! documentation only and MUST NOT be used to generate product clients
//! (OpenAPI is not a wire-type source).

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::protocol::wire::envelope::PROTOCOL_VERSION;
use crate::protocol::wire::method::{DOWNLINK_METHODS, UNARY_METHODS};

/// The checked-in document body (built once; the method table is static).
pub fn openapi_doc() -> &'static str {
    static DOC: OnceLock<String> = OnceLock::new();
    DOC.get_or_init(build)
}

fn build() -> String {
    let mut paths = serde_json::Map::new();

    let healthz = json!({
        "get": {
            "operationId": "healthz",
            "summary": "Liveness probe",
            "responses": {"200": {"description": "ok / shutting_down"}}
        }
    });
    paths.insert("/healthz".into(), healthz);

    let respond = json!({
        "post": {
            "operationId": "respond",
            "summary": "Reverse-RPC answer (approval / question)",
            "requestBody": {"$ref": "#/components/schemas/ClientResponse"},
            "responses": {"200": {"$ref": "#/components/schemas/RpcResult"}}
        }
    });
    paths.insert("/api/respond".into(), respond);

    for method in UNARY_METHODS {
        let entry = json!({
            "post": {
                "operationId": method,
                "summary": format!("unary `{method}`"),
                "requestBody": {"$ref": "#/components/schemas/ClientRequest"},
                "responses": {"200": {"$ref": "#/components/schemas/RpcResult"}}
            }
        });
        paths.insert(format!("/api/{method}"), entry);
    }

    // WS downlink (`GET /api/events.mux`) has no first-class server-push model
    // in OpenAPI; it stays out of `paths` on purpose (D4).
    let downlink_note = DOWNLINK_METHODS
        .iter()
        .map(|m| format!("`{m}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let description = format!(
        "Debug documentation for the xylitol Host four-quadrant unary surface \
         (protocol v{PROTOCOL_VERSION}). HTTP 200 means carrier success; business \
         failures ride `RpcResult.ok=false`.\n\n\
         WebSocket downlink is NOT an OpenAPI path: subscribe via unary, then \
         receive ServerRequest frames ({downlink_note}) on `GET /api/events.mux`. \
         Concrete payload shapes live in the Rust protocol types \
         (`xylitol::protocol::wire`), which — not this document — is the \
         wire-type reference. Do not generate product clients from this file."
    );

    let doc = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "xylitol host unary debug API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": description,
        },
        "paths": Value::Object(paths),
        "components": {
            "schemas": {
                "ClientRequest": client_request_schema(),
                "ClientResponse": client_response_schema(),
                "ServerResponse": server_response_schema(),
                "RpcResult": rpc_result_schema(),
                "RpcError": rpc_error_schema(),
            }
        }
    });

    doc.to_string()
}

fn client_request_schema() -> Value {
    json!({
        "type": "object",
        "description": "client-request quadrant",
        "properties": {
            "type": {"const": "client-request"},
            "rpcId": {"type": "string"},
            "method": {"type": "string"},
            "payload": {"description": "method payload; shape per Rust protocol wire types"},
            "writerToken": {"type": ["string", "null"]}
        },
        "required": ["type", "rpcId", "method"]
    })
}

fn client_response_schema() -> Value {
    json!({
        "type": "object",
        "description": "client-response quadrant (reverse-RPC answer)",
        "properties": {
            "type": {"const": "client-response"},
            "rpcId": {"type": "string"},
            "payload": {"description": "answer payload; shape per Rust protocol wire types"}
        },
        "required": ["type", "rpcId"]
    })
}

fn server_response_schema() -> Value {
    json!({
        "type": "object",
        "description": "server-response quadrant",
        "properties": {
            "type": {"const": "server-response"},
            "rpcId": {"type": "string"},
            "result": {"$ref": "#/components/schemas/RpcResult"}
        },
        "required": ["type", "rpcId", "result"]
    })
}

fn rpc_result_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ok": {"type": "boolean"},
            "value": {"description": "result value; shape per Rust protocol wire types"},
            "error": {"$ref": "#/components/schemas/RpcError"}
        },
        "required": ["ok"]
    })
}

fn rpc_error_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "code": {"type": "string"},
            "details": {"type": "string"}
        },
        "required": ["code", "details"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_lists_every_unary_method() {
        let v: Value = serde_json::from_str(openapi_doc()).expect("valid json");
        let paths = v["paths"].as_object().expect("paths object");
        assert_eq!(v["openapi"], "3.1.0");
        for m in UNARY_METHODS {
            let key = format!("/api/{m}");
            let entry = paths
                .get(key.as_str())
                .unwrap_or_else(|| panic!("missing entry for {m}"));
            assert!(
                entry["post"]["operationId"].as_str() == Some(*m),
                "mismatched operationId for {m}"
            );
        }
        assert!(paths.contains_key("/healthz"));
        assert!(paths.contains_key("/api/respond"));
    }

    #[test]
    fn doc_excludes_ws_downlink_paths() {
        let v: Value = serde_json::from_str(openapi_doc()).expect("valid json");
        let paths = v["paths"].as_object().expect("paths object");
        // WS downlink must not leak into `paths`…
        assert!(!paths.keys().any(|k| k.contains("events.mux")));
        for d in DOWNLINK_METHODS {
            assert!(!paths.contains_key(*d), "downlink path leaked: {d}");
        }
        // …but the prose pointer to the mux channel + Rust protocol types is the contract.
        let desc = v["info"]["description"].as_str().expect("description");
        assert!(
            desc.contains("events.mux"),
            "must explain mux channel: {desc}"
        );
        assert!(
            desc.contains("protocol::wire"),
            "must point at Rust protocol types"
        );
        for d in DOWNLINK_METHODS {
            assert!(
                desc.contains(d),
                "downlink method {d} must be listed in prose"
            );
        }
    }

    #[test]
    fn components_are_envelope_level() {
        let v: Value = serde_json::from_str(openapi_doc()).expect("valid json");
        let schemas = v["components"]["schemas"]
            .as_object()
            .expect("schemas object");
        let mut keys: Vec<_> = schemas.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "ClientRequest",
                "ClientResponse",
                "RpcError",
                "RpcResult",
                "ServerResponse"
            ],
            "envelope-level only; per-method schemas would be a second vocabulary"
        );
    }
}
