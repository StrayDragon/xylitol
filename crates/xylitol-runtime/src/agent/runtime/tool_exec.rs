use std::sync::Arc;

use fastrace::prelude::*;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::hooks::AgentHooks;
use super::middleware::{MiddlewareFault, run_after_tool, run_before_tool};
use super::obs::ToolExecuteSpan;
use super::permission_router::permission_target;
use crate::agent::tools::ToolSet;
use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_plugin_sdk::message::{AgentMessage, AgentPart};
use xylitol_plugin_sdk::{
    AfterToolCtx, BeforeToolCtx, LoopChain, LoopControl, XyBatchMode, XyHookBus, XyHookOutcome,
    XyTool, XyToolCtx,
};

type PermissionCheckFn = dyn Fn(&str, &str) -> Option<String> + Send + Sync;

pub(crate) struct ToolExecEnv<'a> {
    pub tools: &'a ToolSet,
    pub hooks: &'a AgentHooks,
    pub hook_bus: &'a Option<Arc<dyn XyHookBus>>,
    pub permission_check: &'a Option<Arc<PermissionCheckFn>>,
    pub cancel: &'a CancellationToken,
    pub turn_id: Option<&'a str>,
    pub batch_mode: XyBatchMode,
    pub workspace: &'a str,
    pub obs_session: &'a xylitol_plugin_sdk::ObsSessionContext,
    pub loop_chain: &'a LoopChain,
    pub turn_index: u32,
}

pub(crate) struct PreparedCall {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) args: Value,
    pub(crate) tool: Option<Arc<dyn XyTool>>,
    pub(crate) denied_reason: Option<String>,
    pub(crate) stop_run: bool,
    pub(crate) fault: Option<MiddlewareFault>,
}

fn middleware_fault_reason(fault: &MiddlewareFault) -> String {
    fault.message()
}

pub(crate) fn capture_iteration_parent(parent: Option<&Span>) -> Option<SpanContext> {
    parent.and_then(SpanContext::from_span)
}

fn batch_mode_label(mode: XyBatchMode) -> &'static str {
    match mode {
        XyBatchMode::Sequential => "sequential",
        XyBatchMode::BarrierParallel => "barrier_parallel",
    }
}

pub(crate) fn parts_preview_text(parts: &[AgentPart]) -> String {
    let mut out = Vec::new();
    for part in parts {
        match part {
            AgentPart::Text { text } => out.push(text.clone()),
            AgentPart::Image(_) => out.push("[image]".into()),
            AgentPart::Thinking { thinking, .. } => out.push(thinking.clone()),
            AgentPart::ToolCall { name, .. } => out.push(format!("[toolCall:{name}]")),
        }
    }
    out.join("\n")
}

fn details_from_parts(parts: &[AgentPart]) -> Option<Value> {
    match parts {
        [AgentPart::Text { text }] => serde_json::from_str::<Value>(text)
            .ok()
            .filter(Value::is_object),
        _ => None,
    }
}

fn emit(tx: &mpsc::UnboundedSender<XyEvent>, event: XyEvent) {
    let _ = tx.send(event);
}

pub(crate) async fn prepare_call(
    env: &ToolExecEnv<'_>,
    id: &str,
    name: &str,
    args: &Value,
    parent_ctx: Option<SpanContext>,
) -> PreparedCall {
    let mut tool_name = name.to_string();
    let mut tool_args = args.clone();
    let mut denied_reason: Option<String> = None;
    let mut stop_run = false;
    let mut fault: Option<MiddlewareFault> = None;

    if let Some(bus) = env.hook_bus {
        let (ty, phase, hook_ctx) = super::script_hook_ctx::tool_call_pre(name, &tool_args);
        match bus.dispatch(ty, phase, hook_ctx).await {
            XyHookOutcome::Blocked { reason } => denied_reason = Some(reason),
            XyHookOutcome::Modified { args: modified } => tool_args = modified,
            XyHookOutcome::Allowed => {}
        }
    }
    if denied_reason.is_none() && !env.hooks.before_tool_call.is_empty() {
        for hook in &env.hooks.before_tool_call {
            if let Some(reason) = hook(name, id, &tool_args) {
                denied_reason = Some(reason);
                break;
            }
        }
    }

    if denied_reason.is_none() && !env.loop_chain.before_tool.is_empty() {
        let ctx = BeforeToolCtx {
            turn_index: env.turn_index,
            tool_call_id: id.to_string(),
            tool_name: tool_name.clone(),
            args: tool_args.clone(),
        };
        match run_before_tool(env.loop_chain, env.cancel, parent_ctx, ctx).await {
            Ok(out) => {
                tool_name = out.ctx.tool_name;
                tool_args = out.ctx.args;
                if out.control != LoopControl::Continue {
                    stop_run = true;
                    denied_reason = Some(format!(
                        "loop middleware {} stopped at before_tool",
                        out.stopped_by.unwrap_or_else(|| "unknown".into())
                    ));
                }
            }
            Err(mw_fault) => {
                denied_reason = Some(mw_fault.message());
                stop_run = true;
                fault = Some(mw_fault);
            }
        }
    }

    let tool = env.tools.get(&tool_name);
    if denied_reason.is_none()
        && let Some(check) = env.permission_check
    {
        let target = permission_target(&tool_name, &tool_args);
        if let Some(reason) = check(&tool_name, &target) {
            denied_reason = Some(format!("permission denied: {reason}"));
        }
    }

    PreparedCall {
        id: id.to_string(),
        name: tool_name,
        args: tool_args,
        tool,
        denied_reason,
        stop_run,
        fault,
    }
}

pub(crate) struct ToolRunOutcome {
    pub(crate) message: AgentMessage,
    pub(crate) stop_run: bool,
    pub(crate) fault: Option<MiddlewareFault>,
}

pub(crate) async fn run_one(
    env: &ToolExecEnv<'_>,
    call: &PreparedCall,
    events: mpsc::UnboundedSender<XyEvent>,
    parent_ctx: Option<SpanContext>,
    barrier_index: u32,
) -> ToolRunOutcome {
    let id = &call.id;
    let name = &call.name;
    let tool_args = &call.args;
    let tool = &call.tool;
    let denied_reason = &call.denied_reason;
    let mut middleware_stop = call.stop_run;
    let mut middleware_fault = call.fault.clone();
    let tool_span = ToolExecuteSpan::start_with_parent_ctx(
        name,
        id,
        parent_ctx,
        batch_mode_label(env.batch_mode),
        barrier_index,
        env.obs_session,
    );
    let tool_missing = tool.is_none();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<String>(64);
    let (state_tx, mut state_rx) = mpsc::unbounded_channel::<XyEvent>();
    let ctx = XyToolCtx::with_cancel(id, env.cancel.clone())
        .with_output_tx(out_tx)
        .with_state_event_tx(state_tx)
        .with_workspace(env.workspace);

    let args_io = serde_json::to_string(tool_args).unwrap_or_else(|_| "{}".into());
    emit(
        &events,
        XyEvent::ToolExecutionStart {
            id: id.to_string(),
            name: name.to_string(),
            args: tool_args.clone(),
        },
    );

    if let Some(reason) = denied_reason {
        let err = format!("Tool '{name}' blocked: {reason}");
        if let Some(span) = tool_span.as_ref() {
            span.attach_io(&args_io, &err);
        }
        emit(
            &events,
            XyEvent::ToolExecutionUpdate {
                id: id.to_string(),
                output: err.clone(),
            },
        );
        emit(
            &events,
            XyEvent::ToolExecutionEnd {
                id: id.to_string(),
                name: name.to_string(),
                result: err.clone(),
                is_error: true,
            },
        );
        return ToolRunOutcome {
            message: AgentMessage::tool_result(
                id.to_string(),
                name.to_string(),
                vec![AgentPart::text(err)],
                true,
            ),
            stop_run: middleware_stop,
            fault: middleware_fault.take(),
        };
    }

    let exec_fut = async {
        match tool {
            Some(t) => t.execute_as_parts(&ctx, tool_args.clone()).await,
            None => Err(xylitol_plugin_sdk::error::XyToolError::ExecutionFailed(
                anyhow::anyhow!("Unknown tool: {name}"),
            )),
        }
    };
    tokio::pin!(exec_fut);
    let mut streamed_output = false;
    let exec_outcome = loop {
        tokio::select! {
            biased;
            _ = env.cancel.cancelled() => {
                break Err(xylitol_plugin_sdk::error::XyToolError::Aborted);
            }
            Some(chunk) = out_rx.recv() => {
                streamed_output = true;
                emit(
                    &events,
                    XyEvent::ToolExecutionUpdate {
                        id: id.to_string(),
                        output: chunk,
                    },
                );
            }
            Some(state_event) = state_rx.recv() => {
                emit(&events, state_event);
            }
            done = &mut exec_fut => {
                break done;
            }
        }
    };
    while let Ok(output) = out_rx.try_recv() {
        streamed_output = true;
        emit(
            &events,
            XyEvent::ToolExecutionUpdate {
                id: id.to_string(),
                output,
            },
        );
    }
    while let Ok(state_event) = state_rx.try_recv() {
        emit(&events, state_event);
    }

    let mut result = match exec_outcome {
        Ok(parts) => (parts, false),
        Err(xylitol_plugin_sdk::error::XyToolError::Aborted) => {
            let err = format!("Tool '{name}' aborted");
            (vec![AgentPart::text(err)], true)
        }
        Err(e) => {
            super::obs::record_tool_error(name, &e, env.turn_id, parent_ctx, env.obs_session);
            let err = if tool_missing {
                format!("Unknown tool: {name}")
            } else {
                format!("Tool '{name}' error: {e}")
            };
            (vec![AgentPart::text(err)], true)
        }
    };

    if !env.hooks.after_tool_call.is_empty() {
        let mut hook_value = serde_json::Value::String(parts_preview_text(&result.0));
        let mut hook_err = result.1;
        for hook in &env.hooks.after_tool_call {
            if let Some((new_value, new_is_error)) = hook(name, id, hook_value.clone(), hook_err) {
                hook_value = new_value;
                hook_err = new_is_error;
                result = (
                    vec![AgentPart::text(match hook_value {
                        serde_json::Value::String(ref s) => s.clone(),
                        ref other => other.to_string(),
                    })],
                    hook_err,
                );
            }
        }
        result.1 = hook_err;
    }
    if let Some(bus) = env.hook_bus {
        let (ty, phase, hook_ctx) =
            super::script_hook_ctx::tool_result_post(name, parts_preview_text(&result.0), result.1);
        if let XyHookOutcome::Modified { args: modified } = bus.dispatch(ty, phase, hook_ctx).await
        {
            if let Some(val) = modified.get("result") {
                let text = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                result.0 = vec![AgentPart::text(text)];
            }
            if let Some(err) = modified.get("is_error").and_then(|v| v.as_bool()) {
                result.1 = err;
            }
        }
    }

    let mut details = details_from_parts(&result.0);
    if !env.loop_chain.after_tool.is_empty() {
        let ctx = AfterToolCtx {
            turn_index: env.turn_index,
            tool_call_id: id.to_string(),
            tool_name: name.to_string(),
            parts: result.0.clone(),
            details: details.clone(),
            is_error: result.1,
        };
        match run_after_tool(env.loop_chain, env.cancel, parent_ctx, ctx).await {
            Ok(out) => {
                result = (out.ctx.parts, out.ctx.is_error);
                details = out.ctx.details;
            }
            Err(fault) => {
                result = (vec![AgentPart::text(middleware_fault_reason(&fault))], true);
                details = None;
                middleware_stop = true;
                middleware_fault = Some(fault);
            }
        }
    }

    let result_text = parts_preview_text(&result.0);
    if let Some(span) = tool_span.as_ref() {
        span.attach_io(&args_io, &result_text);
    }

    if !streamed_output {
        emit(
            &events,
            XyEvent::ToolExecutionUpdate {
                id: id.to_string(),
                output: result_text.clone(),
            },
        );
    }
    emit(
        &events,
        XyEvent::ToolExecutionEnd {
            id: id.to_string(),
            name: name.to_string(),
            result: result_text.clone(),
            is_error: result.1,
        },
    );

    let (history_parts, quiet_details) = match (&result.0[..], name.as_str()) {
        ([AgentPart::Text { text }], "write" | "edit") if !result.1 => {
            crate::agent::runtime::tool_result_quiet::quiet_write_edit_for_history(
                name, text, result.1,
            )
        }
        _ => (result.0.clone(), None),
    };
    ToolRunOutcome {
        message: AgentMessage::tool_result_with_details(
            id.to_string(),
            name.to_string(),
            history_parts,
            quiet_details.or(details),
            result.1,
        ),
        stop_run: middleware_stop,
        fault: middleware_fault.take(),
    }
}
