//! Event vocabulary — core → client messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::lifecycle::XyEvent;

/// An event from the core: a streamed occurrence during a turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Error {
        /// Stable classification aligned with `protocol::lifecycle` kinds.
        kind: String,
        message: String,
    },
    TextDelta {
        text: String,
    },
    /// Streaming thinking / reasoning text (e.g. Anthropic extended thinking).
    ThinkingDelta {
        text: String,
    },
    ToolStart {
        id: String,
        name: String,
        args: Value,
    },
    ToolEnd {
        id: String,
        name: String,
        result: String,
        /// Terminal failure marker (c2440); required on the wire.
        is_error: bool,
    },
    AgentEnd,
    ModelSelect {
        provider: String,
        model_id: String,
    },
    CompactionStart {
        reason: String,
    },
    /// Turn started.
    TurnStart {
        turn_index: u32,
    },
    /// Turn ended.
    TurnEnd {
        turn_index: u32,
    },
    /// Message started.
    MessageStart {
        role: String,
        message: Option<Value>,
    },
    /// Message ended.
    MessageEnd {
        role: String,
        message: Option<Value>,
    },
    /// Streaming message update (replaces previous text/thinking for this message).
    MessageUpdate {
        text: String,
        thinking: Option<String>,
        message: Option<Value>,
    },
    /// Streaming tool execution output.
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    /// Compaction completed.
    CompactionEnd,
    /// Shared context-token settlement (c1860) for footer / cross-client fixed zone.
    ContextTokenSettlement {
        tokens: u64,
        provenance: String,
        usage_tokens: u64,
        trailing_tokens: u64,
        last_usage_index: Option<usize>,
        reason: String,
        generation: u64,
    },
    /// Pending steer / follow-up queue depths (cross-client badge).
    QueueUpdate {
        steer_count: usize,
        follow_up_count: usize,
    },
    /// Agent Todo checklist snapshot changed (atd13 / pa-todo1): attach clients
    /// refresh the checklist projection from this payload, never from parsing
    /// tool-result text.
    TodoUpdated {
        list: crate::protocol::session::TodoList,
    },
}

impl XyEvent {
    /// Convert a domain lifecycle event to its wire-protocol representation.
    ///
/// Returns `None` for internal-only events that should not cross the
/// client boundary (auto-retry bookkeeping, etc.). [`XyEvent::QueueUpdate`]
/// and [`XyEvent::TodoUpdated`] are wire-visible (c540 / pa-todo1).
    pub fn to_wire_event(&self) -> Option<Event> {
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
            XyEvent::CompactionEnd { .. } => Some(Event::CompactionEnd),
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
            XyEvent::TodoUpdated { list } => Some(Event::TodoUpdated { list: list.clone() }),
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

impl TryFrom<&Event> for XyEvent {
    type Error = WireEventConvertError;

    fn try_from(event: &Event) -> Result<Self, <Self as TryFrom<&Event>>::Error> {
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
            Event::CompactionEnd => Ok(XyEvent::CompactionEnd {
                result: None,
                aborted: false,
                reason: String::new(),
                will_retry: false,
                error_message: None,
                summary: None,
                tokens_before: None,
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
                use crate::protocol::model::{ContextTokenEstimate, TokenProvenance};
                let provenance = match provenance.as_str() {
                    "Api" => TokenProvenance::Api,
                    "RemoteCount" => TokenProvenance::RemoteCount,
                    "LocalTokenizer" => TokenProvenance::LocalTokenizer,
                    "Heuristic" => TokenProvenance::Heuristic,
                    _ => TokenProvenance::Unknown,
                };
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
                crate::protocol::lifecycle::XyEventError::new(kind, message),
            )),
            Event::QueueUpdate {
                steer_count,
                follow_up_count,
            } => Ok(XyEvent::QueueUpdate {
                steer_count: *steer_count,
                follow_up_count: *follow_up_count,
            }),
            Event::TodoUpdated { list } => Ok(XyEvent::TodoUpdated { list: list.clone() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_start_carries_message_over_wire() {
        // ati12: steer/follow-up inject emits MessageStart/End (role=user, with
        // message); the wire projection must keep the payload so attach clients
        // can still uplink the user row into scrollback.
        let domain = XyEvent::MessageStart {
            role: "user".into(),
            message: Some(crate::protocol::message::AgentMessage::user("steer text")),
        };
        let wire = domain
            .to_wire_event()
            .expect("MessageStart is wire-visible");
        let back = XyEvent::try_from(&wire).expect("roundtrip");
        match back {
            XyEvent::MessageStart { role, message } => {
                assert_eq!(role, "user");
                let msg = message.expect("message payload must survive the wire");
                assert_eq!(msg.text(), "steer text");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn user_message_end_carries_message_over_wire() {
        let domain = XyEvent::MessageEnd {
            role: "user".into(),
            message: Some(crate::protocol::message::AgentMessage::user("follow text")),
        };
        let wire = domain.to_wire_event().expect("MessageEnd is wire-visible");
        let back = XyEvent::try_from(&wire).expect("roundtrip");
        match back {
            XyEvent::MessageEnd { role, message } => {
                assert_eq!(role, "user");
                assert_eq!(message.expect("message payload").text(), "follow text");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn thinking_delta_roundtrips_through_wire_event() {
        let domain = XyEvent::ThinkingDelta("reasoning...".into());
        let wire = domain
            .to_wire_event()
            .expect("thinking delta is wire-visible");
        assert!(matches!(
            wire,
            Event::ThinkingDelta {
                ref text,
            } if text == "reasoning..."
        ));

        let back = XyEvent::try_from(&wire).expect("roundtrip succeeds");
        assert!(matches!(
            back,
            XyEvent::ThinkingDelta(text) if text == "reasoning..."
        ));
    }

    #[test]
    fn queue_update_roundtrips_through_wire_event() {
        let domain = XyEvent::QueueUpdate {
            steer_count: 2,
            follow_up_count: 1,
        };
        let wire = domain
            .to_wire_event()
            .expect("QueueUpdate must be wire-visible (c540)");
        assert!(matches!(
            wire,
            Event::QueueUpdate {
                steer_count: 2,
                follow_up_count: 1,
            }
        ));
        let back = XyEvent::try_from(&wire).expect("roundtrip");
        assert!(matches!(
            back,
            XyEvent::QueueUpdate {
                steer_count: 2,
                follow_up_count: 1,
            }
        ));
    }

    #[test]
    fn todo_updated_roundtrips_through_wire_event() {
        // pa-todo1: full TodoList snapshot crosses the wire; empty list = cleared.
        let list = crate::protocol::session::TodoList::new(vec![crate::protocol::session::TodoItem {
            id: "todo-1".into(),
            content: "check env".into(),
            status: crate::protocol::session::TodoStatus::InProgress,
        }]);
        let domain = XyEvent::TodoUpdated { list };
        let wire = domain
            .to_wire_event()
            .expect("TodoUpdated must be wire-visible (pa-todo1)");
        let encoded = serde_json::to_value(&wire).expect("wire serializes");
        assert_eq!(encoded["type"], "todo_updated");
        assert_eq!(encoded["list"]["items"][0]["status"], "in_progress");

        let decoded: Event = serde_json::from_value(encoded).expect("wire deserializes");
        let back = XyEvent::try_from(&decoded).expect("roundtrip");
        assert!(matches!(
            back,
            XyEvent::TodoUpdated { ref list } if list.items.len() == 1
        ));

        let cleared = XyEvent::TodoUpdated {
            list: crate::protocol::session::TodoList::default(),
        };
        let wire = cleared.to_wire_event().expect("empty list still wire-visible");
        let back = XyEvent::try_from(&wire).expect("roundtrip");
        assert!(matches!(
            back,
            XyEvent::TodoUpdated { ref list } if list.is_empty()
        ));
    }

    #[test]
    fn context_token_settlement_roundtrips_through_wire_event() {
        use crate::protocol::model::{ContextTokenEstimate, TokenProvenance};
        let domain = XyEvent::ContextTokenSettlement {
            estimate: ContextTokenEstimate {
                tokens: 42,
                provenance: TokenProvenance::Api,
                usage_tokens: 40,
                trailing_tokens: 2,
                last_usage_index: Some(1),
            },
            reason: "turn_settled".into(),
            generation: 9,
        };
        let wire = domain
            .to_wire_event()
            .expect("ContextTokenSettlement is wire-visible (c1860)");
        let back = XyEvent::try_from(&wire).expect("roundtrip");
        match back {
            XyEvent::ContextTokenSettlement {
                estimate,
                reason,
                generation,
            } => {
                assert_eq!(estimate.tokens, 42);
                assert_eq!(estimate.provenance, TokenProvenance::Api);
                assert_eq!(reason, "turn_settled");
                assert_eq!(generation, 9);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn error_kind_roundtrips_through_wire_event() {
        let domain = XyEvent::Error(crate::protocol::lifecycle::XyEventError::new(
            "Provider",
            "provider error: 503",
        ));
        let wire = domain.to_wire_event().expect("Error is wire-visible");
        assert!(matches!(
            wire,
            Event::Error {
                ref kind,
                ref message,
            } if kind == "Provider" && message == "provider error: 503"
        ));
        let back = XyEvent::try_from(&wire).expect("roundtrip");
        match back {
            XyEvent::Error(err) => {
                assert_eq!(err.kind, "Provider");
                assert_eq!(err.message, "provider error: 503");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn error_requires_kind_on_the_wire() {
        // Strict wire: missing `kind` is rejected, not heuristically classified.
        let wire = serde_json::json!({"type": "error", "message": "aborted"});
        let decoded: Result<Event, _> = serde_json::from_value(wire);
        assert!(decoded.is_err(), "missing kind must be rejected");
    }

    #[test]
    fn tool_start_preserves_args_through_wire_json_roundtrip() {
        let domain = XyEvent::ToolExecutionStart {
            id: "read-1".into(),
            name: "read".into(),
            args: serde_json::json!({"path": "README.md"}),
        };
        let wire = domain.to_wire_event().expect("ToolStart is wire-visible");
        let encoded = serde_json::to_value(&wire).expect("wire serializes");
        assert_eq!(encoded["args"], serde_json::json!({"path": "README.md"}));

        let decoded: Event = serde_json::from_value(encoded).expect("wire deserializes");
        let back = XyEvent::try_from(&decoded).expect("roundtrip succeeds");
        assert!(matches!(
            back,
            XyEvent::ToolExecutionStart { args, .. }
                if args == serde_json::json!({"path": "README.md"})
        ));
    }

    #[test]
    fn message_update_preserves_streaming_tool_call_through_wire_roundtrip() {
        use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

        let partial = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::ToolCall {
                id: "call-1".into(),
                name: "read".into(),
                arguments: serde_json::json!({"path": "README.md"}),
            }],
            stop_reason: None,
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        });
        let expected = serde_json::to_value(&partial).expect("message serializes");
        let domain = XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(partial),
        };

        let wire = domain
            .to_wire_event()
            .expect("MessageUpdate is wire-visible");
        let encoded = serde_json::to_value(&wire).expect("wire serializes");
        assert_eq!(encoded["message"], expected);

        let decoded: Event = serde_json::from_value(encoded).expect("wire deserializes");
        let back = XyEvent::try_from(&decoded).expect("roundtrip succeeds");
        let XyEvent::MessageUpdate { message, .. } = back else {
            panic!("unexpected event");
        };
        assert_eq!(
            serde_json::to_value(message.expect("tool call message")).expect("message serializes"),
            expected
        );
    }
    #[test]
    fn tool_end_roundtrips_is_error_strictly() {
        let domain = XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "bash".into(),
            result: "Tool 'bash' error: timeout after 3s".into(),
            is_error: true,
        };
        let wire = domain.to_wire_event().expect("ToolEnd is wire-visible");
        let encoded = serde_json::to_value(&wire).expect("wire serializes");
        assert_eq!(
            encoded.get("is_error"),
            Some(&serde_json::Value::Bool(true)),
            "wire must carry the failure flag"
        );

        // Strict wire: missing `is_error` is rejected, not defaulted to success.
        let mut stripped = encoded.clone();
        stripped.as_object_mut().unwrap().remove("is_error");
        let decoded: Result<Event, _> = serde_json::from_value(stripped);
        assert!(decoded.is_err(), "missing is_error must be rejected");

        let decoded: Event = serde_json::from_value(encoded).expect("wire deserializes");
        let back = XyEvent::try_from(&decoded).expect("roundtrip");
        let XyEvent::ToolExecutionEnd { is_error, .. } = back else {
            panic!("unexpected event");
        };
        assert!(is_error, "failure flag survives the roundtrip");
    }
}
