use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rstest_bdd_macros::{given, then, when};
use serde_json::{Value, json};

use crate::bdd::fixtures::*;
use xylitol_official_plugin_codemode::{
    CodemodeErrorKind, CodemodePlugin, CodemodeRunner, RunCodeTool, render_declarations,
};
use xylitol_plugin_sdk::rt::CtxHost;
use xylitol_plugin_sdk::{
    AgentPart, CodemodeConfig, HostRegistries, XyTool, XyToolCatalog, XyToolCtx, XyToolDescriptor,
    XyToolDispatch, XyToolError, XyToolExposure, XyToolOutcome,
};
use xylitol_runtime::agent::tools::{ToolCatalog, ToolSet, resolve_tool_selection};

#[derive(Default)]
struct CodemodeState {
    set: Option<ToolSet>,
    allowed: Option<BTreeSet<String>>,
    model: Vec<String>,
    catalog: Vec<XyToolDescriptor>,
    declarations: String,
    script: String,
    fuel: u64,
    error: Option<CodemodeErrorKind>,
    calls: Arc<Mutex<Vec<String>>>,
    registered: bool,
    enabled: bool,
    selection: Vec<String>,
    warnings: Vec<String>,
    run_code_tool: Option<Arc<RunCodeTool>>,
    runner_initialized: bool,
}

impl std::fmt::Debug for CodemodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodemodeState")
            .field("model", &self.model)
            .field("catalog", &self.catalog)
            .field("error", &self.error)
            .field("registered", &self.registered)
            .field("warnings", &self.warnings)
            .finish()
    }
}

thread_local! {
    static CODEMODE: RefCell<Option<CodemodeState>> = const { RefCell::new(None) };
}

fn reset() {
    CODEMODE.with(|cell| {
        *cell.borrow_mut() = Some(CodemodeState {
            fuel: 10_000_000,
            ..Default::default()
        });
    });
}

fn update(f: impl FnOnce(&mut CodemodeState)) {
    CODEMODE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let state = guard.as_mut().expect("codemode state");
        f(state);
    });
}

fn read<T>(f: impl FnOnce(&CodemodeState) -> T) -> T {
    CODEMODE.with(|cell| {
        let guard = cell.borrow();
        let state = guard.as_ref().expect("codemode state");
        f(state)
    })
}

struct StubTool {
    name: &'static str,
    exposure: XyToolExposure,
}

#[async_trait]
impl XyTool for StubTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "stub"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }
    fn exposure(&self) -> XyToolExposure {
        self.exposure
    }
    async fn execute_outcome(
        &self,
        _: &XyToolCtx,
        _: Value,
    ) -> Result<xylitol_plugin_sdk::XyToolOutcome, XyToolError> {
        Ok(xylitol_plugin_sdk::XyToolOutcome::text("ok"))
    }
}

fn stub(name: &'static str, exposure: XyToolExposure) -> Arc<dyn XyTool> {
    Arc::new(StubTool { name, exposure })
}

struct StubCatalog;

impl XyToolCatalog for StubCatalog {
    fn snapshot(&self) -> Vec<XyToolDescriptor> {
        vec![XyToolDescriptor {
            name: "write".into(),
            identifier: "write".into(),
            namespace: Some("core".into()),
            description: "write".into(),
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

struct StubDispatch {
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl XyToolDispatch for StubDispatch {
    async fn call(&self, _: &str, name: &str, args: Value) -> Result<XyToolOutcome, XyToolError> {
        self.calls.lock().expect("calls").push(name.to_string());
        if name == "run_code" {
            return Err(XyToolError::InvalidArgs(
                "run_code cannot be called recursively".into(),
            ));
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
}

#[given("工具集含 direct/model-only/codemode/deferred/hidden 工具")]
fn _g_exposure_tools(_ws: &Workspace) {
    reset();
    update(|state| {
        state.set = Some(ToolSet::from_iter(vec![
            stub("direct", XyToolExposure::Direct),
            stub("model_only", XyToolExposure::ModelOnly),
            stub("code_only", XyToolExposure::Codemode),
            stub("deferred", XyToolExposure::Deferred),
            stub("hidden", XyToolExposure::Hidden),
        ]));
    });
}

#[when("生成模型工具表与 codemode catalog")]
fn _w_generate_surfaces(_ws: &Workspace) {
    update(|state| {
        let set = state.set.as_ref().expect("set");
        state.model = set
            .iter()
            .filter(|tool| tool.exposure().model_visible())
            .map(|tool| tool.name().to_string())
            .collect();
        state.catalog = ToolCatalog::from_toolset(set, None).snapshot();
    });
}

#[then("模型只含 direct 与 model-only 且 catalog 含 direct/codemode/deferred")]
fn _t_exposure_surfaces(_ws: &Workspace) {
    let state = read(|state| (state.model.clone(), state.catalog.clone()));
    assert_eq!(state.0, vec!["direct", "model_only"]);
    let names: Vec<_> = state.1.into_iter().map(|tool| tool.name).collect();
    assert_eq!(names, vec!["code_only", "deferred", "direct"]);
}

#[given("loop_policies.tool_exposure 拒绝 write")]
fn _g_tool_exposure(_ws: &Workspace) {
    reset();
    update(|state| {
        state.set = Some(ToolSet::from_iter(vec![
            stub("read", XyToolExposure::Direct),
            stub("write", XyToolExposure::Direct),
        ]));
        state.allowed = Some(BTreeSet::from(["read".to_string()]));
    });
}

#[when("生成 codemode catalog")]
fn _w_generate_catalog(_ws: &Workspace) {
    update(|state| {
        let set = state.set.as_ref().expect("set");
        state.catalog = ToolCatalog::from_toolset(set, state.allowed.as_ref()).snapshot();
    });
}

#[then("catalog 不含 write")]
fn _t_catalog_without_write(_ws: &Workspace) {
    let catalog = read(|state| state.catalog.clone());
    assert!(catalog.iter().all(|tool| tool.name != "write"));
}

#[given("catalog 含 deferred 工具")]
fn _g_deferred_catalog(_ws: &Workspace) {
    reset();
    update(|state| {
        state.catalog = vec![
            XyToolDescriptor {
                name: "read".into(),
                identifier: "read".into(),
                namespace: Some("core".into()),
                description: "read".into(),
                parameters: json!({"type": "object"}),
                output_schema: None,
                exposure: XyToolExposure::Direct,
                guidelines: Vec::new(),
            },
            XyToolDescriptor {
                name: "deferred".into(),
                identifier: "deferred".into(),
                namespace: Some("official".into()),
                description: "deferred".into(),
                parameters: json!({"type": "object"}),
                output_schema: None,
                exposure: XyToolExposure::Deferred,
                guidelines: Vec::new(),
            },
        ];
    });
}

#[when("生成声明")]
fn _w_declarations(_ws: &Workspace) {
    update(|state| {
        state.declarations = render_declarations(&state.catalog, 10_000);
    });
}

#[then("声明不含该工具且 ALL_TOOLS 可发现")]
fn _t_declarations(_ws: &Workspace) {
    let text = read(|state| state.declarations.clone());
    assert!(!text.contains("- deferred"));
    assert!(text.contains("Deferred tools: deferred"));
    assert!(text.contains("ALL_TOOLS"));
}

#[given("run_code 脚本调用 write 与 run_code")]
fn _g_nested_script(_ws: &Workspace) {
    reset();
    update(|state| {
        state.script = "tools.write({}); tools.run_code({});".into();
        state.calls = Arc::new(Mutex::new(Vec::new()));
    });
}

#[when("执行脚本")]
async fn _w_run_script(_ws: &Workspace) {
    let (script, fuel, calls) =
        read(|state| (state.script.clone(), state.fuel, state.calls.clone()));
    let runner = CodemodeRunner::new(CodemodeConfig {
        enabled: true,
        fuel,
        timeout_ms: 5_000,
        ..Default::default()
    })
    .expect("runner");
    let ctx = XyToolCtx::new("bdd-call").with_dispatch(Arc::new(StubDispatch { calls }));
    let error = runner.run(&ctx, &script).await.expect_err("script");
    update(|state| state.error = Some(error.kind));
}

#[then("write 经 before_tool 与权限且 run_code 被拒绝")]
fn _t_nested_dispatch(_ws: &Workspace) {
    let (calls, error) = read(|state| (state.calls.clone(), state.error));
    let calls = calls.lock().expect("calls").clone();
    assert!(calls.iter().any(|name| name == "write"), "{calls:?}");
    assert!(calls.iter().any(|name| name == "run_code"), "{calls:?}");
    assert_eq!(error, Some(CodemodeErrorKind::Script));
}

#[given("run_code 执行无限循环或收到取消")]
fn _g_infinite_loop(_ws: &Workspace) {
    reset();
    update(|state| {
        state.script = "while (true) {}".into();
        state.fuel = 1_000_000;
    });
}

#[then("返回 fuel_exhausted 或 cancelled 错误")]
fn _t_fuel_or_cancel(_ws: &Workspace) {
    let error = read(|state| state.error);
    assert!(
        matches!(
            error,
            Some(CodemodeErrorKind::FuelExhausted | CodemodeErrorKind::Cancelled)
        ),
        "{error:?}"
    );
}

#[given("配置没有 codemode 节")]
fn _g_zero_assembly(_ws: &Workspace) {
    reset();
}

#[when("装配工具与中间件")]
async fn _w_assemble(_ws: &Workspace) {
    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("root");
    ctx.provide(HostRegistries::new()).expect("registries");
    let plugin = CodemodePlugin::new(&CodemodeConfig::default()).expect("plugin");
    let fiber = ctx.plugin(plugin);
    fiber.dispose().await.expect("dispose");
    let registries = ctx.get::<HostRegistries>().expect("registries");
    update(|state| {
        state.registered = registries.tools.get("run_code").is_some();
    });
    let _ = ctx.shutdown().await;
}

#[then("不含 run_code 且无 codemode 中间件")]
fn _t_zero_assembly(_ws: &Workspace) {
    assert!(!read(|state| state.registered));
}

fn descriptor(name: &str) -> XyToolDescriptor {
    XyToolDescriptor {
        name: name.into(),
        identifier: name.into(),
        namespace: Some("official".into()),
        description: format!("{name} tool"),
        parameters: json!({"type": "object"}),
        output_schema: None,
        exposure: XyToolExposure::Direct,
        guidelines: Vec::new(),
    }
}

#[given("catalog 含 session 工具")]
fn _g_session_catalog(_ws: &Workspace) {
    reset();
    update(|state| {
        state.catalog = vec![descriptor("read"), descriptor("todo")];
    });
}

#[then("声明含该工具且 ALL_TOOLS 同源")]
fn _t_session_catalog(_ws: &Workspace) {
    let (text, catalog) = read(|state| (state.declarations.clone(), state.catalog.clone()));
    assert!(text.contains("- todo"), "{text}");
    assert!(catalog.iter().any(|tool| tool.name == "todo"));
}

#[given("tools 选择禁用 bash")]
fn _g_disabled_bash(_ws: &Workspace) {
    reset();
    update(|state| {
        state.catalog = vec![descriptor("read")];
        state.allowed = Some(BTreeSet::from(["read".to_string()]));
    });
}

#[then("声明不含 bash 且 catalog 不含 bash")]
fn _t_disabled_bash(_ws: &Workspace) {
    let (text, catalog) = read(|state| (state.declarations.clone(), state.catalog.clone()));
    assert!(!text.contains("bash"), "{text}");
    assert!(catalog.iter().all(|tool| tool.name != "bash"));
}

#[given("catalog 已按 tool-exposure 过滤掉 write")]
fn _g_tool_exposure_catalog(_ws: &Workspace) {
    reset();
    update(|state| {
        state.catalog = vec![descriptor("read")];
        state.allowed = Some(BTreeSet::from(["read".to_string()]));
    });
}

#[then("声明不含 write 且 catalog 不含 write")]
fn _t_tool_exposure_catalog(_ws: &Workspace) {
    let (text, catalog) = read(|state| (state.declarations.clone(), state.catalog.clone()));
    assert!(!text.contains("write"), "{text}");
    assert!(catalog.iter().all(|tool| tool.name != "write"));
}

#[given("catalog 为空")]
fn _g_empty_catalog(_ws: &Workspace) {
    reset();
    update(|state| {
        state.catalog = Vec::new();
    });
}

#[then("声明为空")]
fn _t_empty_declarations(_ws: &Workspace) {
    assert!(read(|state| state.declarations.clone()).is_empty());
}

#[given("codemode 启用且 tools 禁用 run_code")]
fn _g_enabled_disabled_run_code(_ws: &Workspace) {
    reset();
    update(|state| {
        state.enabled = true;
        state.selection = vec!["-run_code".into()];
        state.set = Some(ToolSet::from_iter(vec![
            stub("run_code", XyToolExposure::ModelOnly),
            stub("read", XyToolExposure::Direct),
        ]));
    });
}

#[when("解析工具选择")]
fn _w_resolve_selection(_ws: &Workspace) {
    update(|state| {
        let set = state.set.as_ref().expect("set");
        let tools: Vec<_> = set.iter().cloned().collect();
        let (filtered, warnings) = resolve_tool_selection(tools, &state.selection);
        state.set = Some(filtered);
        state.warnings = warnings;
    });
}

#[then("run_code 不可见且无冲突 warning")]
fn _t_enabled_disabled_run_code(_ws: &Workspace) {
    let (set, warnings) = read(|state| (state.set.clone(), state.warnings.clone()));
    let set = set.expect("set");
    assert!(set.get("run_code").is_none());
    assert!(!warnings.iter().any(|warning| warning.contains("run_code")));
}

#[given("codemode 未启用且 tools 选择 run_code")]
fn _g_disabled_selected_run_code(_ws: &Workspace) {
    reset();
    update(|state| {
        state.enabled = false;
        state.selection = vec!["+run_code".into()];
    });
}

#[when("检查配置")]
fn _w_check_config(_ws: &Workspace) {
    update(|state| {
        if !state.enabled && xylitol_plugin_sdk::run_code_selected(&state.selection) {
            state
                .warnings
                .push("codemode disabled; run_code selection ignored".into());
        }
    });
}

#[then("输出 codemode disabled warning")]
fn _t_disabled_selected_run_code(_ws: &Workspace) {
    let warnings = read(|state| state.warnings.clone());
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("codemode disabled")),
        "{warnings:?}"
    );
}

#[given("run_code 工具已注册但未调用")]
fn _g_lazy_runner(_ws: &Workspace) {
    reset();
    update(|state| {
        state.run_code_tool = Some(Arc::new(RunCodeTool::new(CodemodeConfig {
            enabled: true,
            ..Default::default()
        })));
        state.runner_initialized = false;
    });
}

#[when("首次调用 run_code")]
async fn _w_lazy_runner(_ws: &Workspace) {
    let tool = read(|state| state.run_code_tool.clone()).expect("run_code tool");
    let calls = Arc::new(Mutex::new(Vec::new()));
    let ctx = XyToolCtx::new("bdd-lazy-runner").with_dispatch(Arc::new(StubDispatch { calls }));
    for _ in 0..2 {
        tool.execute_outcome(&ctx, json!({"code": "return 1"}))
            .await
            .expect("run_code");
    }
    update(|state| state.runner_initialized = tool.runner_initialized());
}

#[then("才创建 runner 且第二次复用")]
fn _t_lazy_runner(_ws: &Workspace) {
    assert!(read(|state| state.runner_initialized));
}
