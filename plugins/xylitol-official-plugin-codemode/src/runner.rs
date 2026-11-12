use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;
use wasmtime::{
    Caller, Config, Engine, Instance, Linker, Memory, Module, Store, StoreLimits,
    StoreLimitsBuilder, Trap, UpdateDeadline,
};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use xylitol_plugin_sdk::{CodemodeConfig, XyToolCtx, XyToolDispatch, XyToolOutcome};

const GUEST_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodemodeErrorKind {
    Disabled,
    DispatchMissing,
    InvalidArgs,
    FuelExhausted,
    Timeout,
    Cancelled,
    MemoryLimit,
    Script,
    Runtime,
    OutputTooLarge,
}

#[derive(Debug)]
pub struct CodemodeError {
    pub kind: CodemodeErrorKind,
    pub message: String,
}

impl CodemodeError {
    fn new(kind: CodemodeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self.kind {
            CodemodeErrorKind::Disabled => "disabled",
            CodemodeErrorKind::DispatchMissing => "dispatch_missing",
            CodemodeErrorKind::InvalidArgs => "invalid_args",
            CodemodeErrorKind::FuelExhausted => "fuel_exhausted",
            CodemodeErrorKind::Timeout => "timeout",
            CodemodeErrorKind::Cancelled => "cancelled",
            CodemodeErrorKind::MemoryLimit => "memory_limit",
            CodemodeErrorKind::Script => "script_error",
            CodemodeErrorKind::Runtime => "runtime_error",
            CodemodeErrorKind::OutputTooLarge => "output_too_large",
        }
    }
}

impl std::fmt::Display for CodemodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind(), self.message)
    }
}

impl std::error::Error for CodemodeError {}

pub struct CodemodeRunner {
    engine: Engine,
    module: Option<Arc<Module>>,
    linker: Linker<HostState>,
    settings: CodemodeConfig,
}

struct HostState {
    wasi: WasiP1Ctx,
    dispatch: Arc<dyn XyToolDispatch>,
    parent_call_id: String,
    progress_tx: Option<mpsc::Sender<String>>,
    limits: StoreLimits,
    host_buffer: u32,
}

impl CodemodeRunner {
    pub fn new(settings: CodemodeConfig) -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(to_anyhow)?;
        let module = if settings.module_cache {
            Some(Arc::new(
                Module::from_binary(&engine, GUEST_WASM).map_err(to_anyhow)?,
            ))
        } else {
            None
        };
        let mut linker = Linker::new(&engine);
        p1::add_to_linker_sync(&mut linker, |state: &mut HostState| &mut state.wasi)
            .map_err(to_anyhow)?;
        linker
            .func_wrap_async(
                "env",
                "host_tool_call",
                |mut caller: Caller<'_, HostState>, (req_ptr, req_len): (u32, u32)| {
                    Box::new(async move { host_tool_call(&mut caller, req_ptr, req_len).await })
                },
            )
            .map_err(to_anyhow)?;
        linker
            .func_wrap(
                "env",
                "host_log",
                |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| {
                    let Ok(memory) = memory_of(&mut caller) else {
                        return;
                    };
                    let data = memory.data(&caller);
                    let start = ptr as usize;
                    let end = start.saturating_add(len as usize);
                    if end > data.len() {
                        return;
                    }
                    let text = String::from_utf8_lossy(&data[start..end]).to_string();
                    if let Some(tx) = &caller.data().progress_tx {
                        let _ = tx.try_send(text);
                    }
                },
            )
            .map_err(to_anyhow)?;
        Ok(Self {
            engine,
            module,
            linker,
            settings,
        })
    }

    pub async fn run(&self, ctx: &XyToolCtx, code: &str) -> Result<String, CodemodeError> {
        self.run_with_timeout(ctx, code, None).await
    }

    pub async fn run_with_timeout(
        &self,
        ctx: &XyToolCtx,
        code: &str,
        timeout_ms: Option<u64>,
    ) -> Result<String, CodemodeError> {
        self.run_with_details(ctx, code, timeout_ms)
            .await
            .map(|(text, _)| text)
    }

    pub async fn run_with_details(
        &self,
        ctx: &XyToolCtx,
        code: &str,
        timeout_ms: Option<u64>,
    ) -> Result<(String, serde_json::Value), CodemodeError> {
        let started = std::time::Instant::now();
        let script_summary = summarize_script(code);
        let Some(dispatch) = ctx.dispatch().cloned() else {
            return Err(CodemodeError::new(
                CodemodeErrorKind::DispatchMissing,
                "run_code requires a tool dispatch",
            ));
        };
        let catalog = dispatch.catalog().snapshot();
        let preamble = preamble_for(&catalog);
        let script = format!("{preamble}\nconst __result = (() => {{\n{code}\n}})();\n__result");

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.settings.max_memory_bytes)
            .build();
        let state = HostState {
            wasi: WasiCtxBuilder::new().build_p1(),
            dispatch,
            parent_call_id: ctx.call_id.clone(),
            progress_tx: ctx.output_tx.clone(),
            limits,
            host_buffer: 0,
        };
        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupted_for_callback = Arc::clone(&interrupted);
        let mut store = Store::new(&self.engine, state);
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(self.settings.fuel)
            .map_err(|err| CodemodeError::new(CodemodeErrorKind::Runtime, err.to_string()))?;
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(move |_state| {
            if interrupted_for_callback.load(Ordering::SeqCst) {
                Ok(UpdateDeadline::Interrupt)
            } else {
                Ok(UpdateDeadline::Continue(1))
            }
        });

        let module = match &self.module {
            Some(module) => module.clone(),
            None => Arc::new(
                Module::from_binary(&self.engine, GUEST_WASM).map_err(|err| {
                    CodemodeError::new(CodemodeErrorKind::Runtime, err.to_string())
                })?,
            ),
        };
        let instance = self
            .linker
            .instantiate_async(&mut store, &module)
            .await
            .map_err(|err| CodemodeError::new(CodemodeErrorKind::Runtime, err.to_string()))?;
        let host_buffer = call_export_u32(&instance, &mut store, "host_buffer").await?;
        store.data_mut().host_buffer = host_buffer;

        let alloc = instance
            .get_typed_func::<u32, u32>(&mut store, "alloc")
            .map_err(runtime_error)?;
        let ptr = alloc
            .call_async(&mut store, script.len() as u32)
            .await
            .map_err(runtime_error)?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            CodemodeError::new(CodemodeErrorKind::Runtime, "guest memory missing")
        })?;
        memory
            .write(&mut store, ptr as usize, script.as_bytes())
            .map_err(runtime_error)?;

        let eval = instance
            .get_typed_func::<(u32, u32), i32>(&mut store, "eval")
            .map_err(runtime_error)?;
        let timeout = Duration::from_millis(timeout_ms.unwrap_or(self.settings.timeout_ms));
        let eval_future = eval.call_async(&mut store, (ptr, script.len() as u32));
        let code = tokio::select! {
            result = tokio::time::timeout(timeout, eval_future) => {
                match result {
                    Ok(Ok(code)) => code,
                    Ok(Err(err)) => return Err(map_trap(err)),
                    Err(_) => {
                        interrupted.store(true, Ordering::SeqCst);
                        self.engine.increment_epoch();
                        return Err(CodemodeError::new(
                            CodemodeErrorKind::Timeout,
                            format!("run_code timed out (call {})", ctx.call_id),
                        ));
                    }
                }
            }
            _ = ctx.cancel.cancelled() => {
                interrupted.store(true, Ordering::SeqCst);
                self.engine.increment_epoch();
                return Err(CodemodeError::new(
                    CodemodeErrorKind::Cancelled,
                    format!("run_code cancelled (call {})", ctx.call_id),
                ));
            }
        };

        let result_ptr = call_export_u32(&instance, &mut store, "result_ptr").await?;
        let result_len = call_export_u32(&instance, &mut store, "result_len").await?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            CodemodeError::new(CodemodeErrorKind::Runtime, "guest memory missing")
        })?;
        let data = memory.data(&store);
        let start = result_ptr as usize;
        let end = start.saturating_add(result_len as usize);
        if end > data.len() {
            return Err(CodemodeError::new(
                CodemodeErrorKind::Runtime,
                "guest result out of bounds",
            ));
        }
        let mut text = String::from_utf8_lossy(&data[start..end]).to_string();
        if code == 1 {
            return Err(CodemodeError::new(CodemodeErrorKind::Script, text));
        }
        if code == 2 {
            return Err(CodemodeError::new(CodemodeErrorKind::InvalidArgs, text));
        }
        if code == 3 {
            return Err(CodemodeError::new(CodemodeErrorKind::Runtime, text));
        }
        let output_bytes = text.len();
        let limit = self
            .settings
            .max_result_bytes
            .min(self.settings.max_output_tokens.saturating_mul(4));
        let mut truncated = false;
        if text.len() > limit {
            let mut cut = limit;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
            text.push_str("\n[truncated]");
            truncated = true;
        }
        let nested_calls = store
            .data()
            .dispatch
            .nested_summary()
            .unwrap_or_else(|| serde_json::json!([]));
        let details = serde_json::json!({
            "script_summary": script_summary,
            "elapsed_ms": started.elapsed().as_millis(),
            "nested_calls": nested_calls,
            "truncated": truncated,
            "output_bytes": output_bytes,
        });
        Ok((text, details))
    }
}

fn summarize_script(code: &str) -> String {
    let line = code
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    if line.len() <= 120 {
        return line.to_string();
    }
    let mut cut = 120;
    while !line.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &line[..cut])
}

fn preamble_for(catalog: &[xylitol_plugin_sdk::XyToolDescriptor]) -> String {
    let tools = serde_json::to_string(catalog).unwrap_or_else(|_| "[]".to_string());
    format!(
        r#"const ALL_TOOLS = {tools};
function __call(name, args) {{
  const response = JSON.parse(__call_tool(String(name), JSON.stringify(args ?? {{}})));
  if (response.ok === false) throw new Error(response.error);
  return response.value;
}}
const tools = new Proxy({{}}, {{
  get: (_, name) => (...args) => __call(String(name), args[0] ?? {{}}),
}});
function searchTools(query) {{
  const q = String(query).toLowerCase();
  return ALL_TOOLS.filter(t => t.name.toLowerCase().includes(q) || t.description.toLowerCase().includes(q));
}}
function describeTool(name) {{
  return ALL_TOOLS.find(t => t.name === name || t.identifier === name) || null;
}}
function text(value) {{
  return typeof value === "string" ? value : JSON.stringify(value);
}}
const console = {{
  log: (...args) => __log(args.map(text).join(" ")),
  error: (...args) => __log(args.map(text).join(" ")),
}};"#
    )
}

async fn host_tool_call(caller: &mut Caller<'_, HostState>, req_ptr: u32, req_len: u32) -> u32 {
    match host_tool_call_inner(caller, req_ptr, req_len).await {
        Ok(ptr) => ptr,
        Err(err) => write_host_error(caller, err.to_string()).unwrap_or(0),
    }
}

async fn host_tool_call_inner(
    caller: &mut Caller<'_, HostState>,
    req_ptr: u32,
    req_len: u32,
) -> Result<u32> {
    let request = {
        let memory = memory_of(caller)?;
        let data = memory.data(&*caller);
        let start = req_ptr as usize;
        let end = start.saturating_add(req_len as usize);
        if end > data.len() {
            return Err(anyhow::anyhow!("host request out of bounds"));
        }
        serde_json::from_slice::<HostRequest>(&data[start..end])?
    };
    let dispatch = caller.data().dispatch.clone();
    let parent = caller.data().parent_call_id.clone();
    let result = dispatch.call(&parent, &request.name, request.args).await?;
    let json = serialize_result(&result)?;
    let host_buffer = caller.data().host_buffer;
    let memory = memory_of(caller)?;
    let data = memory.data_mut(&mut *caller);
    let start = host_buffer as usize;
    let end = start.saturating_add(json.len() + 4);
    if end > data.len() {
        return Err(anyhow::anyhow!("host result buffer too small"));
    }
    data[start..start + 4].copy_from_slice(&(json.len() as u32).to_le_bytes());
    data[start + 4..end].copy_from_slice(&json);
    Ok(host_buffer)
}

fn write_host_error(caller: &mut Caller<'_, HostState>, message: String) -> Result<u32> {
    let json = serde_json::to_vec(&serde_json::json!({"ok": false, "error": message}))?;
    let host_buffer = caller.data().host_buffer;
    if host_buffer == 0 {
        return Err(anyhow::anyhow!("host buffer missing"));
    }
    let memory = memory_of(caller)?;
    let data = memory.data_mut(&mut *caller);
    let start = host_buffer as usize;
    let end = start.saturating_add(json.len() + 4);
    if end > data.len() {
        return Err(anyhow::anyhow!("host result buffer too small"));
    }
    data[start..start + 4].copy_from_slice(&(json.len() as u32).to_le_bytes());
    data[start + 4..end].copy_from_slice(&json);
    Ok(host_buffer)
}

fn serialize_result(result: &XyToolOutcome) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&serde_json::json!({
        "ok": true,
        "value": {
            "parts": result.parts,
            "details": result.details,
            "is_error": result.is_error,
            "error_kind": result.error_kind,
        },
    }))?)
}

fn memory_of(caller: &mut Caller<'_, HostState>) -> Result<Memory> {
    caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or_else(|| anyhow::anyhow!("guest memory missing"))
}

async fn call_export_u32(
    instance: &Instance,
    store: &mut Store<HostState>,
    name: &str,
) -> Result<u32, CodemodeError> {
    let func = instance
        .get_typed_func::<(), u32>(&mut *store, name)
        .map_err(runtime_error)?;
    func.call_async(&mut *store, ())
        .await
        .map_err(runtime_error)
}

fn runtime_error(err: impl std::fmt::Display) -> CodemodeError {
    CodemodeError::new(CodemodeErrorKind::Runtime, err.to_string())
}

fn to_anyhow(err: wasmtime::Error) -> anyhow::Error {
    anyhow::anyhow!(err.to_string())
}

fn map_trap(err: wasmtime::Error) -> CodemodeError {
    if let Some(trap) = err.downcast_ref::<Trap>() {
        return match trap {
            Trap::OutOfFuel => {
                CodemodeError::new(CodemodeErrorKind::FuelExhausted, "fuel exhausted")
            }
            Trap::Interrupt => {
                CodemodeError::new(CodemodeErrorKind::Cancelled, "guest interrupted")
            }
            Trap::MemoryOutOfBounds => CodemodeError::new(
                CodemodeErrorKind::MemoryLimit,
                "guest memory limit exceeded",
            ),
            _ => CodemodeError::new(CodemodeErrorKind::Runtime, trap.to_string()),
        };
    }
    CodemodeError::new(CodemodeErrorKind::Runtime, err.to_string())
}

#[derive(serde::Deserialize)]
struct HostRequest {
    name: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::{Value, json};

    use super::*;
    use xylitol_plugin_sdk::{
        AgentPart, XyToolCatalog, XyToolCtx, XyToolDescriptor, XyToolDispatch, XyToolError,
        XyToolExposure, XyToolOutcome,
    };

    struct StubCatalog;

    impl XyToolCatalog for StubCatalog {
        fn snapshot(&self) -> Vec<XyToolDescriptor> {
            vec![XyToolDescriptor {
                name: "echo".into(),
                identifier: "echo".into(),
                namespace: Some("core".into()),
                description: "echo".into(),
                parameters: json!({"type": "object"}),
                output_schema: None,
                exposure: XyToolExposure::Direct,
                guidelines: Vec::new(),
            }]
        }
        fn describe(&self, name: &str) -> Option<XyToolDescriptor> {
            self.snapshot().into_iter().find(|tool| tool.name == name)
        }
        fn search(&self, _: &str, _: usize) -> Vec<XyToolDescriptor> {
            self.snapshot()
        }
    }

    struct StubDispatch;

    #[async_trait]
    impl XyToolDispatch for StubDispatch {
        async fn call(
            &self,
            _: &str,
            name: &str,
            args: Value,
        ) -> Result<XyToolOutcome, XyToolError> {
            if name == "sleep" {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Ok(XyToolOutcome {
                parts: vec![AgentPart::text(format!("{name}:{args}"))],
                details: None,
                is_error: false,
                error_kind: None,
            })
        }
        fn catalog(&self) -> Arc<dyn XyToolCatalog> {
            Arc::new(StubCatalog)
        }

        fn nested_summary(&self) -> Option<Value> {
            Some(json!([{
                "name": "echo",
                "args_preview": "{\"value\":1}",
                "ok": true,
                "elapsed_ms": 1
            }]))
        }
    }

    fn config() -> CodemodeConfig {
        CodemodeConfig {
            enabled: true,
            fuel: 10_000_000,
            timeout_ms: 5_000,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn guest_eval_simple() {
        let runner = CodemodeRunner::new(config()).expect("runner");
        let ctx = XyToolCtx::new("call-0").with_dispatch(Arc::new(StubDispatch));
        let result = runner.run(&ctx, "return 42;").await.expect("run");
        assert_eq!(result, "42");
    }

    #[tokio::test]
    async fn guest_calls_host_tool() {
        let runner = CodemodeRunner::new(config()).expect("runner");
        let ctx = XyToolCtx::new("call-1").with_dispatch(Arc::new(StubDispatch));
        let result = runner
            .run(&ctx, "const r = tools.echo({ value: 1 }); return r;")
            .await
            .expect("run");
        assert!(result.contains("echo:"), "result: {result}");
        assert!(result.contains("parts"), "result: {result}");
    }

    #[tokio::test]
    async fn details_include_nested_calls() {
        let runner = CodemodeRunner::new(config()).expect("runner");
        let ctx = XyToolCtx::new("call-details").with_dispatch(Arc::new(StubDispatch));
        let (_text, details) = runner
            .run_with_details(&ctx, "return tools.echo({ value: 1 });", None)
            .await
            .expect("run");
        assert_eq!(details["nested_calls"][0]["name"], "echo");
        assert_eq!(details["truncated"], false);
        assert!(details["script_summary"].as_str().is_some());
    }

    #[tokio::test]
    async fn details_mark_truncation() {
        let runner = CodemodeRunner::new(CodemodeConfig {
            max_result_bytes: 16,
            ..config()
        })
        .expect("runner");
        let ctx = XyToolCtx::new("call-details-trunc").with_dispatch(Arc::new(StubDispatch));
        let (_text, details) = runner
            .run_with_details(&ctx, "return 'x'.repeat(100);", None)
            .await
            .expect("run");
        assert_eq!(details["truncated"], true);
        assert!(details["output_bytes"].as_u64().is_some());
    }

    #[tokio::test]
    async fn output_is_truncated() {
        let runner = CodemodeRunner::new(CodemodeConfig {
            max_result_bytes: 16,
            ..config()
        })
        .expect("runner");
        let ctx = XyToolCtx::new("call-3").with_dispatch(Arc::new(StubDispatch));
        let result = runner
            .run(&ctx, "return 'x'.repeat(100);")
            .await
            .expect("run");
        assert!(result.len() <= 16 + "\n[truncated]".len(), "{result}");
        assert!(result.ends_with("[truncated]"), "{result}");
    }

    #[tokio::test]
    async fn fuel_exhaustion_is_reported() {
        let runner = CodemodeRunner::new(CodemodeConfig {
            fuel: 1_000_000,
            ..config()
        })
        .expect("runner");
        let ctx = XyToolCtx::new("call-2").with_dispatch(Arc::new(StubDispatch));
        let error = runner.run(&ctx, "while (true) {}").await.expect_err("fuel");
        assert_eq!(error.kind, CodemodeErrorKind::FuelExhausted, "{error}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_runs_are_isolated() {
        let runner = Arc::new(
            CodemodeRunner::new(CodemodeConfig {
                fuel: 2_000_000_000,
                ..config()
            })
            .expect("runner"),
        );
        let slow = Arc::clone(&runner);
        let slow_ctx = XyToolCtx::new("call-slow").with_dispatch(Arc::new(StubDispatch));
        let slow_task = tokio::spawn(async move {
            slow.run_with_timeout(&slow_ctx, "tools.sleep({}); return 0;", Some(30))
                .await
        });
        let fast = Arc::clone(&runner);
        let fast_ctx = XyToolCtx::new("call-fast").with_dispatch(Arc::new(StubDispatch));
        let fast_task = tokio::spawn(async move {
            fast.run_with_timeout(&fast_ctx, "tools.sleep({}); return 7;", Some(5_000))
                .await
        });
        let (slow_result, fast_result) = tokio::join!(slow_task, fast_task);
        let slow_error = slow_result.expect("join").expect_err("slow MUST time out");
        assert_eq!(slow_error.kind, CodemodeErrorKind::Timeout, "{slow_error}");
        assert!(
            slow_error.message.contains("call-slow"),
            "timeout MUST be attributable: {}",
            slow_error.message
        );
        let fast_value = fast_result.expect("join").expect("fast MUST complete");
        assert_eq!(fast_value, "7");
    }
}
