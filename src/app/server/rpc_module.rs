//! Product JSON-RPC method table (jsonrpsee [`RpcModule`]).
//!
//! Salvo still owns the TCP listener, `GET /healthz`, and OpenAPI. `POST /rpc`
//! dispatches through this module. Writer lease (`X-Writer-Token`) and the
//! JSON-RPC `id` ride a task-local — never `params`.

use std::sync::Arc;

use jsonrpsee::RpcModule;
use jsonrpsee::types::{ErrorObject, ErrorObjectOwned, Params};
use serde_json::{Value, json};

use crate::app::server::host::{HostState, handle_unary};
use crate::protocol::wire::envelope::RpcResult;
use crate::protocol::wire::registry;

tokio::task_local! {
    static CALL: CallCtx;
}

#[derive(Clone, Default)]
struct CallCtx {
    rpc_id: String,
    writer: Option<String>,
}

/// jsonrpsee's `RpcModule` requires `Context: Debug`.
#[derive(Clone)]
pub(crate) struct RpcHost(pub Arc<HostState>);

impl std::fmt::Debug for RpcHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RpcHost")
    }
}

pub(crate) type ProductRpc = RpcModule<RpcHost>;

pub(crate) fn build_rpc_module(host: Arc<HostState>) -> ProductRpc {
    let mut module = RpcModule::new(RpcHost(host));
    for entry in registry::REGISTRY {
        let name = entry.name;
        module
            .register_async_method(name, move |params, ctx, _ext| async move {
                dispatch_named(name, params, ctx).await
            })
            .expect("REGISTRY method names are unique");
    }
    for name in ["approve_tool", "answer_question"] {
        module
            .register_async_method(name, move |params, ctx, _ext| async move {
                dispatch_approval(params, ctx).await
            })
            .expect("approval method names are unique");
    }
    module
}

pub(crate) async fn dispatch_raw(
    module: &ProductRpc,
    request: &str,
    rpc_id: String,
    writer: Option<String>,
) -> Result<String, serde_json::Error> {
    CALL.scope(CallCtx { rpc_id, writer }, async {
        let (raw, _rx) = module.raw_json_request(request, 4 * 1024 * 1024).await?;
        Ok(raw.get().to_string())
    })
    .await
}

async fn dispatch_named(
    name: &'static str,
    params: Params<'static>,
    ctx: Arc<RpcHost>,
) -> Result<Value, ErrorObjectOwned> {
    let payload = params.parse().unwrap_or_else(|_| json!({}));
    let (rpc_id, writer) = CALL
        .try_with(|c| (c.rpc_id.clone(), c.writer.clone()))
        .unwrap_or_default();
    let rpc_id = (!rpc_id.is_empty()).then_some(rpc_id);
    let result = handle_unary(&ctx.0, rpc_id.as_deref(), name, payload, writer).await;
    into_jrpc(result)
}

async fn dispatch_approval(
    params: Params<'static>,
    ctx: Arc<RpcHost>,
) -> Result<Value, ErrorObjectOwned> {
    let payload: Value = params.parse().unwrap_or_else(|_| json!({}));
    let rpc_id = CALL.try_with(|c| c.rpc_id.clone()).unwrap_or_default();
    let call_id = payload
        .get("call_id")
        .or_else(|| payload.get("id"))
        .and_then(Value::as_str)
        .unwrap_or(rpc_id.as_str())
        .to_string();
    let _ = ctx.0.respond(&call_id, payload).await;
    Ok(json!({}))
}

fn into_jrpc(result: RpcResult) -> Result<Value, ErrorObjectOwned> {
    if result.ok {
        Ok(result.value.unwrap_or_else(|| json!({})))
    } else {
        let err = result
            .error
            .unwrap_or_else(|| crate::protocol::wire::envelope::RpcError {
                code: "rpc_error".into(),
                details: String::new(),
            });
        Err(ErrorObject::owned(
            -32000,
            err.details,
            Some(json!({ "code": err.code })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::server::host::HostState;

    #[tokio::test]
    async fn describe_ok_and_unknown_is_method_not_found() {
        let host = HostState::for_test().expect("host");
        let module = build_rpc_module(host);
        let raw = dispatch_raw(
            &module,
            r#"{"jsonrpc":"2.0","id":"1","method":"host.describe","params":{}}"#,
            "1".into(),
            None,
        )
        .await
        .expect("describe");
        let v: Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(v["jsonrpc"], "2.0", "{raw}");
        assert!(v.get("result").is_some(), "{raw}");
        assert!(v.get("error").is_none(), "{raw}");

        let raw = dispatch_raw(
            &module,
            r#"{"jsonrpc":"2.0","id":"2","method":"no_such_method","params":{}}"#,
            "2".into(),
            None,
        )
        .await
        .expect("unknown");
        let v: Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(v["error"]["code"], -32601, "{raw}");
    }
}
