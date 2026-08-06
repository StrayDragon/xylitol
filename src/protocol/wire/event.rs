//! Event vocabulary — core → client messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::lifecycle::XyEvent;

/// An event from the core: either a response to a command or a streamed
/// occurrence during a turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        message: String,
    },
    Response {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
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
    },
    ToolEnd {
        id: String,
        name: String,
        result: String,
    },
    AgentEnd,
    ModelSelect {
        provider: String,
        model_id: String,
    },
    CompactionStart {
        reason: String,
    },
    /// Acknowledgment of a Subscribe command.
    Subscribed {
        session_id: String,
        seq: u64,
    },
    BashResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        output: String,
        exit_code: Option<i32>,
        cancelled: bool,
        truncated: bool,
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
    },
    /// Message ended.
    MessageEnd {
        role: String,
    },
    /// Streaming message update (replaces previous text/thinking for this message).
    MessageUpdate {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },
    /// Streaming tool execution output.
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    /// Compaction completed.
    CompactionEnd,
    /// Shared context-token settlement (c1860) for footer / cross-client chrome.
    ContextTokenSettlement {
        tokens: u64,
        provenance: String,
        usage_tokens: u64,
        trailing_tokens: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_usage_index: Option<usize>,
        reason: String,
        generation: u64,
    },
    /// Pending steer / follow-up queue depths (cross-client badge).
    QueueUpdate {
        steer_count: usize,
        follow_up_count: usize,
    },
}

impl XyEvent {
    /// Convert a domain lifecycle event to its wire-protocol representation.
    ///
    /// Returns `None` for internal-only events that should not cross the
    /// client boundary (auto-retry bookkeeping, etc.). [`XyEvent::QueueUpdate`]
    /// is wire-visible (c540).
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
            XyEvent::MessageStart { role, .. } => Some(Event::MessageStart { role: role.clone() }),
            XyEvent::MessageEnd { role, .. } => Some(Event::MessageEnd { role: role.clone() }),
            XyEvent::MessageUpdate { text, thinking, .. } => Some(Event::MessageUpdate {
                text: text.clone(),
                thinking: thinking.clone(),
            }),
            XyEvent::ToolExecutionStart { id, name, .. } => Some(Event::ToolStart {
                id: id.clone(),
                name: name.clone(),
            }),
            XyEvent::ToolExecutionEnd {
                id, name, result, ..
            } => Some(Event::ToolEnd {
                id: id.clone(),
                name: name.clone(),
                result: result.clone(),
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
                id: None,
                kind: Some(err.kind.clone()),
                message: err.message.clone(),
            }),
            XyEvent::QueueUpdate {
                steer_count,
                follow_up_count,
            } => Some(Event::QueueUpdate {
                steer_count: *steer_count,
                follow_up_count: *follow_up_count,
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

impl TryFrom<&Event> for XyEvent {
    type Error = String;

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
            Event::MessageStart { role } => Ok(XyEvent::MessageStart {
                role: role.clone(),
                message: None,
            }),
            Event::MessageEnd { role } => Ok(XyEvent::MessageEnd {
                role: role.clone(),
                message: None,
            }),
            Event::MessageUpdate { text, thinking } => Ok(XyEvent::MessageUpdate {
                text: text.clone(),
                thinking: thinking.clone(),
                message: None,
            }),
            Event::ToolStart { id, name } => Ok(XyEvent::ToolExecutionStart {
                id: id.clone(),
                name: name.clone(),
                args: Value::Null,
            }),
            Event::ToolEnd { id, name, result } => Ok(XyEvent::ToolExecutionEnd {
                id: id.clone(),
                name: name.clone(),
                result: result.clone(),
                is_error: false,
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
                use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};
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
            Event::Error { message, kind, .. } => Ok(XyEvent::Error({
                let mut e = crate::protocol::lifecycle::XyEventError::message_only(message.clone());
                if let Some(k) = kind.clone() {
                    e.kind = k;
                }
                e
            })),
            Event::QueueUpdate {
                steer_count,
                follow_up_count,
            } => Ok(XyEvent::QueueUpdate {
                steer_count: *steer_count,
                follow_up_count: *follow_up_count,
            }),
            other => Err(format!(
                "event variant not convertible to XyEvent: {other:?}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn context_token_settlement_roundtrips_through_wire_event() {
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};
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
                kind: Some(ref k),
                ref message,
                ..
            } if k == "Provider" && message == "provider error: 503"
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
    fn error_legacy_aborted_message_without_kind() {
        let wire = Event::Error {
            id: None,
            kind: None,
            message: "aborted".into(),
        };
        let back = XyEvent::try_from(&wire).expect("legacy error ok");
        match back {
            XyEvent::Error(err) => assert!(err.is_aborted()),
            other => panic!("unexpected: {other:?}"),
        }
    }
}
