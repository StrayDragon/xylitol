//! XyEvent ↔ wire Event mapping (envelope types live in xylitol-protocol).

use xylitol_protocol::Event;

use xylitol_runtime::XyEvent;

fn todo_list_to_wire(list: &xylitol_runtime::TodoList) -> xylitol_protocol::TodoList {
    serde_json::from_value(serde_json::to_value(list).expect("todo list json"))
        .expect("wire todo list")
}

fn todo_list_from_wire(list: &xylitol_protocol::TodoList) -> xylitol_runtime::TodoList {
    serde_json::from_value(serde_json::to_value(list).expect("wire todo json"))
        .expect("sdk todo list")
}

/// Convert a domain lifecycle event to its wire-protocol representation.
///
/// Lives next to the wire [`Event`] enum (orphan-safe: local trait, foreign
/// [`XyEvent`]). Returns `None` for internal-only events that should not cross
/// the client boundary (auto-retry bookkeeping, etc.). [`XyEvent::QueueUpdate`]
/// and [`XyEvent::TodoUpdated`] are wire-visible (c540 / pa-todo1).
pub trait ToWireEvent {
    fn to_wire_event(&self) -> Option<Event>;
}

/// Project a wire [`Event`] into a domain [`XyEvent`] (orphan-safe local trait).
pub trait FromWireEvent: Sized {
    fn from_wire_event(event: &Event) -> Result<Self, WireEventConvertError>;
}

impl ToWireEvent for XyEvent {
    fn to_wire_event(&self) -> Option<Event> {
        match self {
            XyEvent::TextDelta(text) => Some(Event::TextDelta { text: text.clone() }),
            XyEvent::ThinkingDelta(text) => Some(Event::ThinkingDelta { text: text.clone() }),
            XyEvent::TurnStart { turn_index } => Some(Event::TurnStart {
                turn_index: *turn_index,
            }),
            XyEvent::TurnEnd { turn_index } => Some(Event::TurnEnd {
                turn_index: *turn_index,
            }),
            XyEvent::MessageStart { role, message } => Some(Event::MessageStart {
                role: role.clone(),
                message: message
                    .as_ref()
                    .and_then(|value| serde_json::to_value(value).ok()),
            }),
            XyEvent::MessageEnd { role, message } => Some(Event::MessageEnd {
                role: role.clone(),
                message: message
                    .as_ref()
                    .and_then(|value| serde_json::to_value(value).ok()),
            }),
            XyEvent::MessageUpdate {
                text,
                thinking,
                message,
            } => Some(Event::MessageUpdate {
                text: text.clone(),
                thinking: thinking.clone(),
                message: message
                    .as_ref()
                    .and_then(|value| serde_json::to_value(value).ok()),
            }),
            XyEvent::ToolExecutionStart { id, name, args, .. } => Some(Event::ToolStart {
                id: id.clone(),
                name: name.clone(),
                args: args.clone(),
            }),
            XyEvent::ToolExecutionEnd {
                id,
                name,
                result,
                is_error,
            } => Some(Event::ToolEnd {
                id: id.clone(),
                name: name.clone(),
                result: result.clone(),
                is_error: *is_error,
            }),
            XyEvent::ToolExecutionUpdate { id, output } => Some(Event::ToolExecutionUpdate {
                id: id.clone(),
                output: output.clone(),
            }),
            XyEvent::ModelSelect { provider, model_id } => Some(Event::ModelSelect {
                provider: provider.clone(),
                model_id: model_id.clone(),
            }),
            XyEvent::CompactionStart { reason } => Some(Event::CompactionStart {
                reason: reason.clone(),
            }),
            XyEvent::CompactionEnd {
                result,
                aborted,
                reason,
                will_retry,
                error_message,
                summary,
                tokens_before,
                tokens_after,
                notice,
            } => Some(Event::CompactionEnd {
                result: result.clone(),
                aborted: *aborted,
                reason: reason.clone(),
                will_retry: *will_retry,
                error_message: error_message.clone(),
                summary: summary.clone(),
                tokens_before: *tokens_before,
                tokens_after: *tokens_after,
                notice: notice.clone(),
            }),
            XyEvent::ContextTokenSettlement {
                estimate,
                reason,
                generation,
            } => Some(Event::ContextTokenSettlement {
                tokens: estimate.tokens,
                provenance: estimate.provenance.as_str().to_string(),
                usage_tokens: estimate.usage_tokens,
                trailing_tokens: estimate.trailing_tokens,
                last_usage_index: estimate.last_usage_index,
                reason: reason.clone(),
                generation: *generation,
            }),
            XyEvent::AgentEnd { .. } => Some(Event::AgentEnd),
            XyEvent::Error(err) => Some(Event::Error {
                kind: err.kind.clone(),
                message: err.message.clone(),
            }),
            XyEvent::QueueUpdate {
                steer_count,
                follow_up_count,
            } => Some(Event::QueueUpdate {
                steer_count: *steer_count,
                follow_up_count: *follow_up_count,
            }),
            XyEvent::TodoUpdated { list } => Some(Event::TodoUpdated {
                list: todo_list_to_wire(list),
            }),
            // Degraded (not on the wire): process-local or REST-covered.
            XyEvent::AgentStart { .. }
            | XyEvent::AutoRetryStart { .. }
            | XyEvent::AutoRetryEnd { .. }
            | XyEvent::SessionInfoChanged { .. }
            | XyEvent::ThinkingLevelChanged { .. } => {
                log::debug!(
                    "wire: dropping non-mapped XyEvent event={}",
                    self.description()
                );
                None
            }
        }
    }
}

/// Failure to project a wire [`Event`] into an [`XyEvent`] (`TryFrom` error type).
///
/// Typed (not `String`) so callers can distinguish conversion failures from
/// transport-level errors and keep the offending variant for diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum WireEventConvertError {
    /// The wire variant has no `XyEvent` projection (degraded / client-side only).
    #[error("event variant not convertible to XyEvent: {0}")]
    Unmapped(String),
}

impl FromWireEvent for XyEvent {
    fn from_wire_event(event: &Event) -> Result<Self, WireEventConvertError> {
        match event {
            Event::TextDelta { text } => Ok(XyEvent::TextDelta(text.clone())),
            Event::ThinkingDelta { text } => Ok(XyEvent::ThinkingDelta(text.clone())),
            Event::TurnStart { turn_index } => Ok(XyEvent::TurnStart {
                turn_index: *turn_index,
            }),
            Event::TurnEnd { turn_index } => Ok(XyEvent::TurnEnd {
                turn_index: *turn_index,
            }),
            Event::MessageStart { role, message } => Ok(XyEvent::MessageStart {
                role: role.clone(),
                message: message
                    .as_ref()
                    .and_then(|value| serde_json::from_value(value.clone()).ok()),
            }),
            Event::MessageEnd { role, message } => Ok(XyEvent::MessageEnd {
                role: role.clone(),
                message: message
                    .as_ref()
                    .and_then(|value| serde_json::from_value(value.clone()).ok()),
            }),
            Event::MessageUpdate {
                text,
                thinking,
                message,
            } => Ok(XyEvent::MessageUpdate {
                text: text.clone(),
                thinking: thinking.clone(),
                message: message
                    .as_ref()
                    .and_then(|value| serde_json::from_value(value.clone()).ok()),
            }),
            Event::ToolStart { id, name, args } => Ok(XyEvent::ToolExecutionStart {
                id: id.clone(),
                name: name.clone(),
                args: args.clone(),
            }),
            Event::ToolEnd {
                id,
                name,
                result,
                is_error,
            } => Ok(XyEvent::ToolExecutionEnd {
                id: id.clone(),
                name: name.clone(),
                result: result.clone(),
                is_error: *is_error,
            }),
            Event::ToolExecutionUpdate { id, output } => Ok(XyEvent::ToolExecutionUpdate {
                id: id.clone(),
                output: output.clone(),
            }),
            Event::ModelSelect { provider, model_id } => Ok(XyEvent::ModelSelect {
                provider: provider.clone(),
                model_id: model_id.clone(),
            }),
            Event::CompactionStart { reason } => Ok(XyEvent::CompactionStart {
                reason: reason.clone(),
            }),
            Event::CompactionEnd {
                result,
                aborted,
                reason,
                will_retry,
                error_message,
                summary,
                tokens_before,
                tokens_after,
                notice,
            } => Ok(XyEvent::CompactionEnd {
                result: result.clone(),
                aborted: *aborted,
                reason: reason.clone(),
                will_retry: *will_retry,
                error_message: error_message.clone(),
                summary: summary.clone(),
                tokens_before: *tokens_before,
                tokens_after: *tokens_after,
                notice: notice.clone(),
            }),
            Event::ContextTokenSettlement {
                tokens,
                provenance,
                usage_tokens,
                trailing_tokens,
                last_usage_index,
                reason,
                generation,
            } => {
                use xylitol_runtime::{ContextTokenEstimate, TokenProvenance};
                let provenance = TokenProvenance::from_key(provenance);
                Ok(XyEvent::ContextTokenSettlement {
                    estimate: ContextTokenEstimate {
                        tokens: *tokens,
                        provenance,
                        usage_tokens: *usage_tokens,
                        trailing_tokens: *trailing_tokens,
                        last_usage_index: *last_usage_index,
                    },
                    reason: reason.clone(),
                    generation: *generation,
                })
            }
            Event::AgentEnd => Ok(XyEvent::AgentEnd {
                messages: Vec::new(),
            }),
            Event::Error { message, kind } => Ok(XyEvent::Error(
                xylitol_runtime::XyEventError::new(kind, message),
            )),
            Event::QueueUpdate {
                steer_count,
                follow_up_count,
            } => Ok(XyEvent::QueueUpdate {
                steer_count: *steer_count,
                follow_up_count: *follow_up_count,
            }),
            Event::TodoUpdated { list } => Ok(XyEvent::TodoUpdated {
                list: todo_list_from_wire(list),
            }),
        }
    }
}
