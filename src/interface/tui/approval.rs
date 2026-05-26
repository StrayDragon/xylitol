//! Tool approval plumbing for the TUI.
//!
//! This module provides:
//! - An [`ApprovalHub`] which connects the UI overlay to tool execution.
//! - A [`SecureApprovalToolWrapper`] which enforces security policy and blocks
//!   tool execution until the user approves (when required).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};
use crate::infra::security::{SecurityEngine, SecurityVerdict};

/// User decision for a tool approval prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalDecision {
    Allow,
    Deny,
    AllowOnce,
    DenyOnce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StickyDecision {
    Allow,
    Deny,
}

/// Shared hub for approval requests keyed by function-call id.
///
/// The UI registers an approval prompt on `ToolCallStart`, and tool execution
/// later awaits the decision when the corresponding tool `execute()` begins.
#[derive(Debug, Default)]
pub(crate) struct ApprovalHub {
    pending: Mutex<HashMap<String, oneshot::Receiver<ApprovalDecision>>>,
    sticky: Mutex<HashMap<String, StickyDecision>>,
}

impl ApprovalHub {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Register an approval prompt for the given tool-call id and return the sender.
    pub(crate) fn register(&self, call_id: String) -> oneshot::Sender<ApprovalDecision> {
        let (tx, rx) = oneshot::channel();

        match self.pending.lock() {
            Ok(mut map) => {
                map.insert(call_id, rx);
            }
            Err(err) => {
                tracing::error!("ApprovalHub lock poisoned; dropping pending receiver: {err}");
            }
        }

        tx
    }

    /// Take the receiver for the given tool-call id (only one waiter exists).
    pub(crate) fn take(&self, call_id: &str) -> Option<oneshot::Receiver<ApprovalDecision>> {
        match self.pending.lock() {
            Ok(mut map) => map.remove(call_id),
            Err(err) => {
                tracing::error!("ApprovalHub lock poisoned; cannot take receiver: {err}");
                None
            }
        }
    }

    fn get_sticky(&self, tool_name: &str) -> Option<StickyDecision> {
        match self.sticky.lock() {
            Ok(map) => map.get(tool_name).copied(),
            Err(err) => {
                tracing::error!("ApprovalHub sticky lock poisoned; treating as no-decision: {err}");
                None
            }
        }
    }

    fn set_sticky(&self, tool_name: String, decision: StickyDecision) {
        match self.sticky.lock() {
            Ok(mut map) => {
                map.insert(tool_name, decision);
            }
            Err(err) => {
                tracing::error!("ApprovalHub sticky lock poisoned; cannot store decision: {err}");
            }
        }
    }
}

/// Returns `true` when a tool call should require user approval in the TUI.
pub(crate) fn requires_approval(security_enabled: bool, tool_name: &str) -> bool {
    if !security_enabled {
        return false;
    }
    matches!(tool_name, "write" | "edit" | "bash")
}

/// A tool wrapper that:
/// - Enforces [`SecurityEngine`] allow/block checks.
/// - When required, blocks tool execution until the user approves.
pub(crate) struct SecureApprovalToolWrapper {
    inner: Arc<dyn XyTool>,
    engine: SecurityEngine,
    approvals: Arc<ApprovalHub>,
    tool_name: String,
    security_enabled: bool,
    approval_tools: Arc<HashSet<String>>,
}

impl std::fmt::Debug for SecureApprovalToolWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureApprovalToolWrapper")
            .field("tool", &self.tool_name)
            .finish()
    }
}

impl SecureApprovalToolWrapper {
    pub(crate) fn new(
        tool: Arc<dyn XyTool>,
        engine: SecurityEngine,
        approvals: Arc<ApprovalHub>,
        approval_tools: Arc<HashSet<String>>,
        security_enabled: bool,
    ) -> Self {
        let tool_name = tool.name().to_string();
        Self {
            inner: tool,
            engine,
            approvals,
            tool_name,
            security_enabled,
            approval_tools,
        }
    }

    fn should_prompt(&self) -> bool {
        self.security_enabled && self.approval_tools.contains(&self.tool_name)
    }
}

#[async_trait]
impl XyTool for SecureApprovalToolWrapper {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        ctx: &XyToolCtx,
        args: serde_json::Value,
    ) -> Result<String, XyToolError> {
        let blocked_json = |reason: &str, rule: &str| -> String {
            serde_json::to_string(&serde_json::json!({
                "blocked": true,
                "reason": reason,
                "rule": rule,
            }))
            .unwrap()
        };

        // 1) Security policy always runs first.
        match self.engine.check_tool_call(&self.tool_name, &args) {
            SecurityVerdict::Blocked { reason, rule } => {
                tracing::warn!(
                    tool = self.tool_name,
                    reason = reason,
                    rule = rule,
                    "Tool call blocked by security policy"
                );
                return Ok(blocked_json(&reason, &rule));
            }
            SecurityVerdict::Allowed => {}
        }

        // 2) Optional human approval gate.
        if self.should_prompt() {
            if let Some(sticky) = self.approvals.get_sticky(&self.tool_name) {
                match sticky {
                    StickyDecision::Allow => {}
                    StickyDecision::Deny => {
                        return Ok(blocked_json(
                            "tool call denied by user (sticky decision)",
                            "approval",
                        ));
                    }
                }
            } else {
                let call_id = ctx.call_id.clone();
                let receiver = self.approvals.take(&call_id);
                match receiver {
                    Some(rx) => match rx.await {
                        Ok(decision) => match decision {
                            ApprovalDecision::Allow => {
                                self.approvals
                                    .set_sticky(self.tool_name.clone(), StickyDecision::Allow);
                            }
                            ApprovalDecision::Deny => {
                                self.approvals
                                    .set_sticky(self.tool_name.clone(), StickyDecision::Deny);
                                return Ok(blocked_json("tool call denied by user", "approval"));
                            }
                            ApprovalDecision::AllowOnce => {}
                            ApprovalDecision::DenyOnce => {
                                return Ok(blocked_json(
                                    "tool call denied by user (once)",
                                    "approval",
                                ));
                            }
                        },
                        Err(err) => {
                            tracing::warn!(
                                tool = self.tool_name,
                                call_id = call_id,
                                error = %err,
                                "Approval channel closed; denying tool call"
                            );
                            return Ok(blocked_json("approval channel closed", "approval"));
                        }
                    },
                    None => {
                        tracing::warn!(
                            tool = self.tool_name,
                            call_id = call_id,
                            "No approval receiver registered; denying tool call"
                        );
                        return Ok(blocked_json("missing approval prompt", "approval"));
                    }
                }
            }
        }

        // 3) Resource limit for bash.
        if self.tool_name == "bash" {
            match self.engine.acquire_subprocess() {
                Ok(_guard) => self.inner.execute(ctx, args).await,
                Err(verdict) => {
                    let (reason, rule) = match verdict {
                        SecurityVerdict::Blocked { reason, rule } => (reason, rule),
                        SecurityVerdict::Allowed => ("unknown".into(), "unknown".into()),
                    };
                    tracing::warn!(
                        tool = self.tool_name,
                        reason = reason,
                        rule = rule,
                        "Subprocess limit reached"
                    );
                    Ok(blocked_json(&reason, &rule))
                }
            }
        } else {
            self.inner.execute(ctx, args).await
        }
    }
}
