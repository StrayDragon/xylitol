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
use crate::protocol::wire::method::DOWNLINK_METHODS;
use crate::protocol::wire::registry;

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
            "summary": "Liveness/readiness probe (c2465)",
            "responses": {"200": {"description": "ok"}, "503": {"description": "starting (retry_after) / stopping / failed"}}
        }
    });
    paths.insert("/healthz".into(), healthz);

    paths.insert(
        "/rpc".into(),
        json!({
            "post": {
                "operationId": "jsonrpc",
                "summary": "JSON-RPC 2.0 product entry",
                "requestBody": {"$ref": "#/components/schemas/JsonRpcRequest"},
                "responses": {"200": {"$ref": "#/components/schemas/JsonRpcResponse"}}
            }
        }),
    );

    let methods = registry::names().collect::<Vec<_>>().join(", ");
    let downlink_note = DOWNLINK_METHODS
        .iter()
        .map(|m| format!("`{m}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let description = format!(
        "Debug documentation for the xylitol Host JSON-RPC 2.0 surface \
         (protocol v{PROTOCOL_VERSION}). Product entry is POST /rpc and WS /rpc. \
         HTTP 200 means the JSON-RPC envelope parsed; business failures ride \
         error.data.code (string). Envelope numeric codes are carriers only.\n\n\
         Registered methods: {methods}.\n\n\
         WebSocket is NOT an OpenAPI operation path: subscribe then receive \
         JSON-RPC notifications ({downlink_note}) on WS /rpc. \
         Concrete payload shapes live in the product method table, which — not \
         this document — is the wire-type reference. Do not generate product \
         clients from this file."
    );

    let doc = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "xylitol host JSON-RPC debug API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": description,
        },
        "paths": Value::Object(paths),
        "components": {
            "schemas": {
                "JsonRpcRequest": jsonrpc_request_schema(),
                "JsonRpcResponse": jsonrpc_response_schema(),
                "JsonRpcError": jsonrpc_error_schema(),
            }
        }
    });

    doc.to_string()
}

fn jsonrpc_request_schema() -> Value {
    json!({
        "type": "object",
        "description": "JSON-RPC 2.0 request",
        "properties": {
            "jsonrpc": {"const": "2.0"},
            "id": {"type": ["string", "number"]},
            "method": {"type": "string"},
            "params": {"description": "method params; shape per product method table"}
        },
        "required": ["jsonrpc", "method"]
    })
}

fn jsonrpc_response_schema() -> Value {
    json!({
        "type": "object",
        "description": "JSON-RPC 2.0 result or error (mutually exclusive)",
        "properties": {
            "jsonrpc": {"const": "2.0"},
            "id": {"type": ["string", "number", "null"]},
            "result": {"description": "success value; shape per product method table"},
            "error": {"$ref": "#/components/schemas/JsonRpcError"}
        },
        "required": ["jsonrpc"]
    })
}

fn jsonrpc_error_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "code": {"type": "integer", "description": "JSON-RPC carrier numeric code"},
            "message": {"type": "string"},
            "data": {
                "type": "object",
                "properties": {
                    "code": {"type": "string", "description": "product business code"}
                }
            }
        },
        "required": ["code", "message"]
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
        assert!(paths.contains_key("/healthz"));
        assert!(paths.contains_key("/rpc"));
        assert!(!paths.contains_key("/api/respond"));
        for m in registry::names() {
            assert!(
                !paths.contains_key(&format!("/api/{m}")),
                "per-method /api path leaked: {m}"
            );
        }
        let desc = v["info"]["description"].as_str().expect("description");
        assert!(desc.contains("JSON-RPC"), "{desc}");
        for m in registry::names() {
            assert!(desc.contains(m), "method {m} must appear in prose");
        }
    }

    #[test]
    fn doc_excludes_ws_downlink_paths() {
        let v: Value = serde_json::from_str(openapi_doc()).expect("valid json");
        let paths = v["paths"].as_object().expect("paths object");
        assert!(!paths.keys().any(|k| k.contains("events.mux")));
        for d in DOWNLINK_METHODS {
            assert!(!paths.contains_key(*d), "downlink path leaked: {d}");
        }
        let desc = v["info"]["description"].as_str().expect("description");
        assert!(desc.contains("WS /rpc"), "{desc}");
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
            ["JsonRpcError", "JsonRpcRequest", "JsonRpcResponse"],
            "envelope-level only; per-method schemas would be a second vocabulary"
        );
        assert_eq!(
            schemas["JsonRpcRequest"]["properties"]["jsonrpc"]["const"],
            "2.0"
        );
    }
}
