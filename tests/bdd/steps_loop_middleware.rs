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
    BeforeModelCtx, BeforeToolCtx, BeforeTurnCtx, DecideContinueCtx, LoopControl, LoopMiddleware,
    LoopMiddlewareRegistry, LoopStage, LoopStep,
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
        &[LoopStage::AfterModel]
    }

    async fn after_model(
        &self,
        mut ctx: xylitol_plugin_sdk::AfterModelCtx,
    ) -> LoopStep<xylitol_plugin_sdk::AfterModelCtx> {
        ctx.assistant = xylitol_plugin_sdk::AgentMessage::Llm(
            xylitol_plugin_sdk::XyLlmMessage::assistant("replaced"),
        );
        LoopStep::continued(ctx)
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
        &[LoopStage::BeforeModel]
    }

    async fn before_model(&self, mut ctx: BeforeModelCtx) -> LoopStep<BeforeModelCtx> {
        ctx.messages.push(xylitol_plugin_sdk::AgentMessage::Llm(
            xylitol_plugin_sdk::XyLlmMessage::user("middleware fragment"),
        ));
        LoopStep::continued(ctx)
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

#[given("注册 before_model 阶段 order 10 与 order 5 的两个中间件")]
pub(crate) fn g_register_two_ordered() {
    let registry = fresh_registry();
    registry
        .register(scripted(
            "order-ten",
            10,
            &[LoopStage::BeforeModel],
            LoopControl::Continue,
        ))
        .expect("register order-ten");
    registry
        .register(scripted(
            "order-five",
            5,
            &[LoopStage::BeforeModel],
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
        .before_model
        .iter()
        .map(|mw| mw.id().to_string())
        .collect();
    assert!(registry.unregister("order-five"));
    let after: Vec<String> = registry
        .snapshot()
        .before_model
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

#[given("注册 before_model 中间件且在观测闸开启下运行一回合")]
pub(crate) async fn g_trace_before_model(agent: &AgentState, ws: &Workspace) {
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
            Some("before_model"),
            "{props:?}"
        );
        let changed = get("loop.middleware.changed").expect("changed summary");
        assert_ne!(changed, "none", "middleware changed messages: {props:?}");
    });
}

#[given("loop 示例插件配置缺省")]
pub(crate) fn g_example_default() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_loop_example::LoopExamplePlugin::new(
            &xylitol_official_plugin_loop_example::LoopExampleConfig::default(),
        )
        .expect("loop example plugin"),
    );
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
}

#[given("以启用 loop 示例插件装配运行时")]
pub(crate) async fn g_example_enabled() {
    use xylitol_plugin_sdk::rt::CtxHost;

    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("ctx root");
    let registry = fresh_registry();
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry.clone())
        .expect("provide registry");
    let fiber = ctx.plugin(
        xylitol_official_plugin_loop_example::LoopExamplePlugin::new(
            &xylitol_official_plugin_loop_example::LoopExampleConfig {
                enabled: true,
                max_settles: Some(2),
                deny_bash_patterns: vec!["rm -rf /".into()],
            },
        )
        .expect("loop example plugin"),
    );
    for _ in 0..64 {
        if !registry.is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(registry.ids(), vec!["loop-example".to_string()]);
    LOOP_PLUGIN_REGISTRY.with(|slot| slot.borrow_mut().replace(registry));
    LOOP_PLUGIN_FIBER.with(|slot| slot.borrow_mut().replace(fiber));
    LOOP_PLUGIN_CTX.with(|slot| slot.borrow_mut().replace(ctx));
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

#[given("注册 after_model 中间件把 assistant 文本改写为 replaced")]
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
