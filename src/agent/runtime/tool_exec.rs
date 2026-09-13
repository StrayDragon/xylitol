//! Single-tool execution helpers extracted from ReAct (c1545).
//!
//! Emits lifecycle events on a channel so the ReAct `async_stream` can `yield`
//! them (including live Update mux) while awaiting Sequential or BarrierParallel
//! fan-out. History messages are returned to the caller for **source-order**
//! persist — independent of End completion order.

use std::sync::Arc;

use fastrace::prelude::*;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::hooks::AgentHooks;
use super::obs::ToolExecuteSpan;
use super::permission_router::permission_target;
use crate::agent::tools::ToolSet;
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::{AgentMessage, AgentPart};
use crate::protocol::ports::{XyBatchMode, XyHookBus, XyHookOutcome, XyToolCtx};

/// Permission gate: `(tool_name, target) -> Some(deny_reason)`.
type PermissionCheckFn = dyn Fn(&str, &str) -> Option<String> + Send + Sync;

/// Shared environment for one tool-batch flush / Sequential run_one.
pub(crate) struct ToolExecEnv<'a> {
    pub tools: &'a ToolSet,
    pub hooks: &'a AgentHooks,
    pub hook_bus: &'a Option<Arc<dyn XyHookBus>>,
    pub permission_check: &'a Option<Arc<PermissionCheckFn>>,
    pub cancel: &'a CancellationToken,
    pub turn_id: Option<&'a str>,
    pub batch_mode: XyBatchMode,
    /// Frozen session workspace for this run — injected into every
    /// [`XyToolCtx`] so tools resolve relative paths / spawn shells in it.
    pub workspace: &'a str,
    /// This run's obs session snapshot (otel24): tool spans stamp it, not the slot.
    pub obs_session: &'a xylitol_ai_bridge::ObsSessionContext,
}

/// Capture parent for parallel fan-out (explicit; not the global parent slot).
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

fn emit(tx: &mpsc::UnboundedSender<XyEvent>, event: XyEvent) {
    let _ = tx.send(event);
}

/// Run one tool call end-to-end: Start → preflight → exec(+Update) → after → End.
///
/// Returns the history tool-result message (caller persists in source order).
pub(crate) async fn run_one(
    env: &ToolExecEnv<'_>,
    id: &str,
    name: &str,
    args: &Value,
    events: mpsc::UnboundedSender<XyEvent>,
    parent_ctx: Option<SpanContext>,
    barrier_index: u32,
) -> AgentMessage {
    let tool_span = ToolExecuteSpan::start_with_parent_ctx(
        name,
        id,
        parent_ctx,
        batch_mode_label(env.batch_mode),
        barrier_index,
        env.obs_session,
    );
    let args_io = serde_json::to_string(args).unwrap_or_else(|_| "{}".into());

    emit(
        &events,
        XyEvent::ToolExecutionStart {
            id: id.to_string(),
            name: name.to_string(),
            args: args.clone(),
        },
    );

    let tool = env.tools.get(name);
    let tool_missing = tool.is_none();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<String>(64);
    // Typed state-event uplink (atd13): tools publishing domain-state events
    // (todo_* → TodoUpdated) ride the same run stream as tool tape.
    let (state_tx, mut state_rx) = mpsc::unbounded_channel::<XyEvent>();
    let ctx = XyToolCtx::with_cancel(id, env.cancel.clone())
        .with_output_tx(out_tx)
        .with_state_event_tx(state_tx)
        .with_workspace(env.workspace);
    let mut tool_args = args.clone();

    let mut denied_reason: Option<String> = None;
    if let Some(bus) = env.hook_bus {
        let (ty, phase, hook_ctx) = super::script_hook_ctx::tool_call_pre(name, &tool_args);
        match bus.dispatch(ty, phase, hook_ctx).await {
            XyHookOutcome::Blocked { reason } => {
                denied_reason = Some(reason);
            }
            XyHookOutcome::Modified { args: modified } => {
                tool_args = modified;
            }
            XyHookOutcome::Allowed => {}
        }
    }
    if !env.hooks.before_tool_call.is_empty() {
        for hook in &env.hooks.before_tool_call {
            if let Some(reason) = hook(name, id, &tool_args) {
                denied_reason = Some(reason);
                break;
            }
        }
    }

    if denied_reason.is_none()
        && let Some(check) = env.permission_check
    {
        let target = permission_target(name, &tool_args);
        if let Some(reason) = check(name, &target) {
            denied_reason = Some(format!("permission denied: {reason}"));
        }
    }

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
        return AgentMessage::tool_result(
            id.to_string(),
            name.to_string(),
            vec![AgentPart::text(err)],
            true,
        );
    }

    let tool_arc = tool;
    let exec_fut = async {
        match tool_arc {
            Some(t) => t.execute_as_parts(&ctx, tool_args.clone()).await,
            None => Err(crate::protocol::error::XyToolError::ExecutionFailed(
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
                break Err(crate::protocol::error::XyToolError::Aborted);
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
        Err(crate::protocol::error::XyToolError::Aborted) => {
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

    let (history_parts, details) =
        crate::agent::runtime::tool_result_quiet::quiet_write_edit_for_history(
            name,
            &result_text,
            result.1,
        );
    AgentMessage::tool_result_with_details(
        id.to_string(),
        name.to_string(),
        history_parts,
        details,
        result.1,
    )
}
