use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use fastrace::prelude::{Event, Span, SpanContext};
use tokio_util::sync::CancellationToken;

use xylitol_plugin_sdk::{
    AfterModelCtx, AfterToolCtx, AfterTurnCtx, BeforeModelCtx, BeforeToolCtx, BeforeTurnCtx,
    DecideContinueCtx, LoopChain, LoopControl, LoopMiddleware, LoopStage, LoopStep,
    provider_trace_active,
};

pub(crate) const LOOP_MIDDLEWARE_TIMEOUT: Duration = Duration::from_secs(2);
pub(crate) const LOOP_MIDDLEWARE_CONTINUE_LIMIT: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MiddlewareFault {
    pub(crate) id: String,
    pub(crate) stage: LoopStage,
    pub(crate) timed_out: bool,
}

impl MiddlewareFault {
    pub(crate) fn cancelled(id: String, stage: LoopStage) -> Self {
        Self {
            id,
            stage,
            timed_out: false,
        }
    }

    pub(crate) fn timed_out(id: String, stage: LoopStage) -> Self {
        Self {
            id,
            stage,
            timed_out: true,
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        if self.timed_out {
            "LoopMiddlewareTimeout"
        } else {
            "Aborted"
        }
    }

    pub(crate) fn message(&self) -> String {
        if self.timed_out {
            format!(
                "loop middleware {} timed out after {:?} at {}",
                self.id,
                LOOP_MIDDLEWARE_TIMEOUT,
                self.stage.as_str()
            )
        } else {
            format!(
                "loop middleware {} cancelled at {}",
                self.id,
                self.stage.as_str()
            )
        }
    }
}

pub(crate) struct StageOutcome<C> {
    pub(crate) ctx: C,
    pub(crate) control: LoopControl,
    pub(crate) stopped_by: Option<String>,
}

struct StageCall<'a> {
    middleware: &'a Arc<dyn LoopMiddleware>,
    stage: LoopStage,
    cancel: &'a CancellationToken,
    parent: Option<SpanContext>,
    scope: xylitol_plugin_sdk::LoopScope,
}

async fn call<C, F>(
    call: StageCall<'_>,
    before: String,
    fut: F,
    summarize: impl FnOnce(&C) -> String,
) -> Result<LoopStep<C>, MiddlewareFault>
where
    F: Future<Output = LoopStep<C>>,
{
    let id = call.middleware.id().to_string();
    let stage = call.stage;
    let order = call.middleware.order();
    let span = provider_trace_active().then(|| {
        let parent = call.parent.unwrap_or_else(SpanContext::random);
        let id_attr = id.clone();
        let scope_attr = call.scope;
        Span::root("loop.middleware", parent).with_properties(move || {
            vec![
                ("loop.middleware.id".to_string(), id_attr),
                (
                    "loop.middleware.stage".to_string(),
                    stage.as_str().to_string(),
                ),
                ("loop.middleware.order".to_string(), order.to_string()),
                (
                    "loop.middleware.scope".to_string(),
                    scope_attr.as_str().to_string(),
                ),
            ]
        })
    });

    let outcome = tokio::select! {
        biased;
        _ = call.cancel.cancelled() => Err(MiddlewareFault::cancelled(id.clone(), stage)),
        res = tokio::time::timeout(LOOP_MIDDLEWARE_TIMEOUT, fut) => match res {
            Ok(step) => Ok(step),
            Err(_) => Err(MiddlewareFault::timed_out(id.clone(), stage)),
        },
    };

    if let Some(span) = span {
        match &outcome {
            Ok(step) => {
                let after = summarize(&step.ctx);
                let changed = if after == before {
                    "none"
                } else {
                    after.as_str()
                };
                span.add_property(|| {
                    ("loop.middleware.control", step.control.as_str().to_string())
                });
                span.add_property(|| ("loop.middleware.changed", changed.to_string()));
                span.add_property(|| ("loop.middleware.outcome", "ok".to_string()));
            }
            Err(fault) => {
                span.add_property(|| ("loop.middleware.outcome", fault.kind().to_string()));
                span.add_event(Event::new("loop.middleware.fault").with_properties(|| {
                    vec![(
                        "loop.middleware.fault_kind".to_string(),
                        fault.kind().to_string(),
                    )]
                }));
            }
        }
        drop(span);
    }
    outcome
}

fn messages_fingerprint(messages: &[xylitol_plugin_sdk::AgentMessage]) -> String {
    let chars: usize = messages.iter().map(|m| m.text().chars().count()).sum();
    let tool_calls: usize = messages
        .iter()
        .map(|m| {
            m.content()
                .iter()
                .filter(|p| matches!(p, xylitol_plugin_sdk::AgentPart::ToolCall { .. }))
                .count()
        })
        .sum();
    format!("{}:{chars}:{tool_calls}", messages.len())
}

fn before_turn_fingerprint(ctx: &BeforeTurnCtx) -> String {
    let prompt = ctx.system_prompt.as_deref().map(str::len).unwrap_or(0);
    format!("{prompt}:{}", ctx.system_fragments.len())
}

fn before_model_fingerprint(ctx: &BeforeModelCtx) -> String {
    let mut schemas: Vec<&str> = ctx.tool_schemas.iter().map(|s| s.name.as_str()).collect();
    schemas.sort_unstable();
    format!(
        "{}|{}|{}|{:?}|{:?}",
        messages_fingerprint(&ctx.messages),
        schemas.join(","),
        ctx.options.thinking_level,
        ctx.options.max_output_tokens,
        ctx.options.system_prompt.as_deref().map(str::len)
    )
}

fn after_model_fingerprint(ctx: &AfterModelCtx) -> String {
    format!(
        "{}|{}",
        ctx.assistant.text().chars().count(),
        ctx.assistant
            .content()
            .iter()
            .filter(|p| matches!(p, xylitol_plugin_sdk::AgentPart::ToolCall { .. }))
            .count()
    )
}

fn before_tool_fingerprint(ctx: &BeforeToolCtx) -> String {
    format!(
        "{}|{}",
        ctx.tool_name,
        serde_json::to_string(&ctx.args).unwrap_or_default()
    )
}

fn after_tool_fingerprint(ctx: &AfterToolCtx) -> String {
    format!(
        "{}:{}:{:?}",
        ctx.parts
            .iter()
            .map(|p| match p {
                xylitol_plugin_sdk::AgentPart::Text { text } => text.chars().count(),
                _ => 1,
            })
            .sum::<usize>(),
        ctx.is_error,
        ctx.details.is_some()
    )
}

async fn run_stage<C>(
    chain: &LoopChain,
    stage: LoopStage,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    ctx: C,
    fingerprint: impl Fn(&C) -> String,
    invoke: impl for<'a> Fn(
        &'a Arc<dyn LoopMiddleware>,
        C,
    ) -> std::pin::Pin<Box<dyn Future<Output = LoopStep<C>> + Send + 'a>>,
) -> Result<StageOutcome<C>, MiddlewareFault> {
    let mut ctx = ctx;
    for mw in chain.stage(stage) {
        let before = fingerprint(&ctx);
        let step = call(
            StageCall {
                middleware: mw,
                stage,
                cancel,
                parent,
                scope: chain.scope,
            },
            before,
            invoke(mw, ctx),
            &fingerprint,
        )
        .await?;
        ctx = step.ctx;
        if step.control != LoopControl::Continue {
            return Ok(StageOutcome {
                ctx,
                control: step.control,
                stopped_by: Some(mw.id().to_string()),
            });
        }
    }
    Ok(StageOutcome {
        ctx,
        control: LoopControl::Continue,
        stopped_by: None,
    })
}

pub(crate) async fn run_before_turn(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    ctx: BeforeTurnCtx,
) -> Result<StageOutcome<BeforeTurnCtx>, MiddlewareFault> {
    run_stage(
        chain,
        LoopStage::BeforeTurn,
        cancel,
        parent,
        ctx,
        before_turn_fingerprint,
        |mw, ctx| Box::pin(mw.before_turn(ctx)),
    )
    .await
}

pub(crate) async fn run_before_model(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    ctx: BeforeModelCtx,
) -> Result<StageOutcome<BeforeModelCtx>, MiddlewareFault> {
    run_stage(
        chain,
        LoopStage::BeforeModel,
        cancel,
        parent,
        ctx,
        before_model_fingerprint,
        |mw, ctx| Box::pin(mw.before_model(ctx)),
    )
    .await
}

pub(crate) async fn run_after_model(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    ctx: AfterModelCtx,
) -> Result<StageOutcome<AfterModelCtx>, MiddlewareFault> {
    run_stage(
        chain,
        LoopStage::AfterModel,
        cancel,
        parent,
        ctx,
        after_model_fingerprint,
        |mw, ctx| Box::pin(mw.after_model(ctx)),
    )
    .await
}

pub(crate) async fn run_before_tool(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    ctx: BeforeToolCtx,
) -> Result<StageOutcome<BeforeToolCtx>, MiddlewareFault> {
    run_stage(
        chain,
        LoopStage::BeforeTool,
        cancel,
        parent,
        ctx,
        before_tool_fingerprint,
        |mw, ctx| Box::pin(mw.before_tool(ctx)),
    )
    .await
}

pub(crate) async fn run_after_tool(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    ctx: AfterToolCtx,
) -> Result<StageOutcome<AfterToolCtx>, MiddlewareFault> {
    run_stage(
        chain,
        LoopStage::AfterTool,
        cancel,
        parent,
        ctx,
        after_tool_fingerprint,
        |mw, ctx| Box::pin(mw.after_tool(ctx)),
    )
    .await
}

pub(crate) async fn run_decide_continue(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    ctx: DecideContinueCtx,
) -> Result<StageOutcome<DecideContinueCtx>, MiddlewareFault> {
    run_stage(
        chain,
        LoopStage::DecideContinue,
        cancel,
        parent,
        ctx,
        |_ctx| String::new(),
        |mw, ctx| Box::pin(mw.decide_continue(ctx)),
    )
    .await
}

pub(crate) async fn run_after_turn(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<SpanContext>,
    mut ctx: AfterTurnCtx,
) -> Result<AfterTurnCtx, MiddlewareFault> {
    for mw in chain.stage(LoopStage::AfterTurn) {
        let step = call(
            StageCall {
                middleware: mw,
                stage: LoopStage::AfterTurn,
                cancel,
                parent,
                scope: chain.scope,
            },
            String::new(),
            mw.after_turn(ctx),
            |_ctx| String::new(),
        )
        .await?;
        if step.control != LoopControl::Continue {
            log::warn!(
                target: "xylitol::react",
                "after_turn middleware {} returned control={} (ignored; after_turn is observe-only)",
                mw.id(),
                step.control.as_str()
            );
        }
        ctx = step.ctx;
    }
    Ok(ctx)
}

pub(crate) fn fragments_into_prompt(
    prompt: Option<String>,
    fragments: &[String],
) -> Option<String> {
    if fragments.is_empty() {
        return prompt;
    }
    let mut out = prompt.unwrap_or_default();
    for fragment in fragments {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(fragment);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragments_merge_into_prompt() {
        assert_eq!(fragments_into_prompt(None, &[]), None);
        assert_eq!(
            fragments_into_prompt(Some("base".into()), &["extra".into()]),
            Some("base\n\nextra".to_string())
        );
        assert_eq!(
            fragments_into_prompt(None, &["a".into(), "b".into()]),
            Some("a\n\nb".to_string())
        );
    }

    #[test]
    fn fingerprints_track_message_and_schema_changes() {
        let ctx = BeforeModelCtx {
            turn_index: 0,
            messages: vec![xylitol_plugin_sdk::AgentMessage::Llm(
                xylitol_plugin_sdk::XyLlmMessage::user("hi"),
            )],
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
        };
        let base = before_model_fingerprint(&ctx);
        let mut changed = ctx.clone();
        changed.messages.push(xylitol_plugin_sdk::AgentMessage::Llm(
            xylitol_plugin_sdk::XyLlmMessage::user("more"),
        ));
        assert_ne!(base, before_model_fingerprint(&changed));

        let mut schema_changed = ctx.clone();
        schema_changed
            .tool_schemas
            .push(xylitol_plugin_sdk::XyToolSchema {
                name: "read".into(),
                description: "d".into(),
                parameters: serde_json::json!({}),
            });
        assert_ne!(base, before_model_fingerprint(&schema_changed));
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
            LoopStep::stopped(ctx)
        }
    }

    #[tokio::test]
    async fn after_turn_control_is_ignored() {
        let mut chain = LoopChain::default();
        chain.after_turn.push(Arc::new(StopAfterTurn));
        let cancel = CancellationToken::new();
        let out = run_after_turn(
            &chain,
            &cancel,
            None,
            AfterTurnCtx {
                turn_index: 3,
                continued_tools: true,
                history_len: 9,
            },
        )
        .await
        .expect("after_turn");
        assert_eq!(out.turn_index, 3);
        assert_eq!(out.history_len, 9);
    }

    #[test]
    fn fault_kinds_are_stable() {
        let timeout = MiddlewareFault::timed_out("x".into(), LoopStage::BeforeTool);
        assert_eq!(timeout.kind(), "LoopMiddlewareTimeout");
        assert!(timeout.message().contains("2s"));
        let cancelled = MiddlewareFault::cancelled("y".into(), LoopStage::AfterTurn);
        assert_eq!(cancelled.kind(), "Aborted");
        assert!(cancelled.message().contains("after_turn"));
    }
}
