use serde::Serialize;
use serde_json::Value;

use xylitol_plugin_sdk::session::TodoList as DomainTodoList;
use xylitol_plugin_sdk::{
    AgentMessage, ContextTokenEstimate, TokenProvenance, XyEvent, XyEventError,
};
use xylitol_protocol::{Event, TodoList as WireTodoList};

#[derive(Debug, thiserror::Error)]
pub enum WireEventError {
    #[error("wire event payload is not JSON-representable: {0}")]
    Payload(#[from] serde_json::Error),
    #[error("unknown wire event variant")]
    Unknown,
}

pub fn event_to_wire(event: &XyEvent) -> Result<Event, WireEventError> {
    Ok(match event {
        XyEvent::AgentStart { session_id, model } => Event::AgentStart {
            session_id: session_id.clone(),
            model: model.clone(),
        },
        XyEvent::AgentEnd { .. } => Event::AgentEnd,
        XyEvent::TurnStart { turn_index } => Event::TurnStart {
            turn_index: *turn_index,
        },
        XyEvent::TurnEnd { turn_index } => Event::TurnEnd {
            turn_index: *turn_index,
        },
        XyEvent::MessageStart { role, message } => Event::MessageStart {
            role: role.clone(),
            message: to_value_opt(message)?,
        },
        XyEvent::MessageUpdate { message } => Event::MessageUpdate {
            message: to_value_opt(message)?,
        },
        XyEvent::MessageEnd { role, message } => Event::MessageEnd {
            role: role.clone(),
            message: to_value_opt(message)?,
        },
        XyEvent::TextDelta(text) => Event::TextDelta { text: text.clone() },
        XyEvent::ThinkingDelta(text) => Event::ThinkingDelta { text: text.clone() },
        XyEvent::ToolExecutionStart { id, name, args } => Event::ToolExecutionStart {
            id: id.clone(),
            name: name.clone(),
            args: args.clone(),
        },
        XyEvent::ToolExecutionUpdate { id, output } => Event::ToolExecutionUpdate {
            id: id.clone(),
            output: output.clone(),
        },
        XyEvent::ToolExecutionEnd {
            id,
            name,
            result,
            is_error,
        } => Event::ToolExecutionEnd {
            id: id.clone(),
            name: name.clone(),
            result: result.clone(),
            is_error: *is_error,
        },
        XyEvent::CompactionStart { reason } => Event::CompactionStart {
            reason: reason.clone(),
        },
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
        } => Event::CompactionEnd {
            result: result.clone(),
            aborted: *aborted,
            reason: reason.clone(),
            will_retry: *will_retry,
            error_message: error_message.clone(),
            summary: summary.clone(),
            tokens_before: *tokens_before,
            tokens_after: *tokens_after,
            notice: notice.clone(),
        },
        XyEvent::ContextTokenSettlement {
            estimate,
            reason,
            generation,
        } => Event::ContextTokenSettlement {
            tokens: estimate.tokens,
            provenance: estimate.provenance.as_str().to_string(),
            usage_tokens: estimate.usage_tokens,
            trailing_tokens: estimate.trailing_tokens,
            last_usage_index: estimate.last_usage_index,
            reason: reason.clone(),
            generation: *generation,
        },
        XyEvent::ModelSelect { provider, model_id } => Event::ModelSelect {
            provider: provider.clone(),
            model_id: model_id.clone(),
        },
        XyEvent::ThinkingLevelChanged { level } => Event::ThinkingLevelChanged {
            level: level.clone(),
        },
        XyEvent::QueueUpdate {
            steer_count,
            follow_up_count,
        } => Event::QueueUpdate {
            steer_count: *steer_count,
            follow_up_count: *follow_up_count,
        },
        XyEvent::TodoUpdated { list } => Event::TodoUpdated {
            list: todo_to_wire(list)?,
        },
        XyEvent::AutoRetryStart {
            attempt,
            max_retries,
            delay_ms,
        } => Event::AutoRetryStart {
            attempt: *attempt,
            max_retries: *max_retries,
            delay_ms: *delay_ms,
        },
        XyEvent::AutoRetryEnd { success, attempt } => Event::AutoRetryEnd {
            success: *success,
            attempt: *attempt,
        },
        XyEvent::Error(err) => Event::Error {
            kind: err.kind.clone(),
            message: err.message.clone(),
        },
        XyEvent::SessionInfoChanged { key, value } => Event::SessionInfoChanged {
            key: key.clone(),
            value: value.clone(),
        },
    })
}

pub fn event_from_wire(event: &Event) -> Result<XyEvent, WireEventError> {
    Ok(match event {
        Event::AgentStart { session_id, model } => XyEvent::AgentStart {
            session_id: session_id.clone(),
            model: model.clone(),
        },
        Event::AgentEnd => XyEvent::AgentEnd {
            messages: Vec::new(),
        },
        Event::TurnStart { turn_index } => XyEvent::TurnStart {
            turn_index: *turn_index,
        },
        Event::TurnEnd { turn_index } => XyEvent::TurnEnd {
            turn_index: *turn_index,
        },
        Event::MessageStart { role, message } => XyEvent::MessageStart {
            role: role.clone(),
            message: from_value_opt(message)?,
        },
        Event::MessageUpdate { message } => XyEvent::MessageUpdate {
            message: from_value_opt(message)?,
        },
        Event::MessageEnd { role, message } => XyEvent::MessageEnd {
            role: role.clone(),
            message: from_value_opt(message)?,
        },
        Event::TextDelta { text } => XyEvent::TextDelta(text.clone()),
        Event::ThinkingDelta { text } => XyEvent::ThinkingDelta(text.clone()),
        Event::ToolExecutionStart { id, name, args } => XyEvent::ToolExecutionStart {
            id: id.clone(),
            name: name.clone(),
            args: args.clone(),
        },
        Event::ToolExecutionUpdate { id, output } => XyEvent::ToolExecutionUpdate {
            id: id.clone(),
            output: output.clone(),
        },
        Event::ToolExecutionEnd {
            id,
            name,
            result,
            is_error,
        } => XyEvent::ToolExecutionEnd {
            id: id.clone(),
            name: name.clone(),
            result: result.clone(),
            is_error: *is_error,
        },
        Event::CompactionStart { reason } => XyEvent::CompactionStart {
            reason: reason.clone(),
        },
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
        } => XyEvent::CompactionEnd {
            result: result.clone(),
            aborted: *aborted,
            reason: reason.clone(),
            will_retry: *will_retry,
            error_message: error_message.clone(),
            summary: summary.clone(),
            tokens_before: *tokens_before,
            tokens_after: *tokens_after,
            notice: notice.clone(),
        },
        Event::ContextTokenSettlement {
            tokens,
            provenance,
            usage_tokens,
            trailing_tokens,
            last_usage_index,
            reason,
            generation,
        } => XyEvent::ContextTokenSettlement {
            estimate: ContextTokenEstimate {
                tokens: *tokens,
                provenance: TokenProvenance::from_key(provenance),
                usage_tokens: *usage_tokens,
                trailing_tokens: *trailing_tokens,
                last_usage_index: *last_usage_index,
            },
            reason: reason.clone(),
            generation: *generation,
        },
        Event::ModelSelect { provider, model_id } => XyEvent::ModelSelect {
            provider: provider.clone(),
            model_id: model_id.clone(),
        },
        Event::ThinkingLevelChanged { level } => XyEvent::ThinkingLevelChanged {
            level: level.clone(),
        },
        Event::QueueUpdate {
            steer_count,
            follow_up_count,
        } => XyEvent::QueueUpdate {
            steer_count: *steer_count,
            follow_up_count: *follow_up_count,
        },
        Event::TodoUpdated { list } => XyEvent::TodoUpdated {
            list: todo_from_wire(list)?,
        },
        Event::AutoRetryStart {
            attempt,
            max_retries,
            delay_ms,
        } => XyEvent::AutoRetryStart {
            attempt: *attempt,
            max_retries: *max_retries,
            delay_ms: *delay_ms,
        },
        Event::AutoRetryEnd { success, attempt } => XyEvent::AutoRetryEnd {
            success: *success,
            attempt: *attempt,
        },
        Event::Error { kind, message } => XyEvent::Error(XyEventError::new(kind, message)),
        Event::SessionInfoChanged { key, value } => XyEvent::SessionInfoChanged {
            key: key.clone(),
            value: value.clone(),
        },
        Event::Unknown => return Err(WireEventError::Unknown),
    })
}

fn to_value_opt(message: &Option<AgentMessage>) -> Result<Option<Value>, WireEventError> {
    message
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(WireEventError::from)
}

fn from_value_opt(message: &Option<Value>) -> Result<Option<AgentMessage>, WireEventError> {
    message
        .as_ref()
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(WireEventError::from)
}

fn todo_to_wire(list: &DomainTodoList) -> Result<WireTodoList, WireEventError> {
    reshape(list)
}

fn todo_from_wire(list: &WireTodoList) -> Result<DomainTodoList, WireEventError> {
    reshape(list)
}

fn reshape<T: Serialize, U: serde::de::DeserializeOwned>(value: &T) -> Result<U, WireEventError> {
    Ok(serde_json::from_value(serde_json::to_value(value)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::session::{TodoItem, TodoStatus};

    fn sample_estimate() -> ContextTokenEstimate {
        ContextTokenEstimate {
            tokens: 42,
            provenance: TokenProvenance::Api,
            usage_tokens: 40,
            trailing_tokens: 2,
            last_usage_index: Some(3),
        }
    }

    #[test]
    fn every_domain_variant_round_trips() {
        let todo = DomainTodoList::new(vec![TodoItem {
            id: "1".into(),
            content: "write tests".into(),
            status: TodoStatus::InProgress,
        }]);
        let cases: Vec<XyEvent> = vec![
            XyEvent::AgentStart {
                session_id: "s0".into(),
                model: "m".into(),
            },
            XyEvent::AgentEnd {
                messages: Vec::new(),
            },
            XyEvent::TurnStart { turn_index: 1 },
            XyEvent::TurnEnd { turn_index: 1 },
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: Some(AgentMessage::assistant("start")),
            },
            XyEvent::MessageUpdate {
                message: Some(AgentMessage::assistant("partial")),
            },
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: Some(AgentMessage::assistant("done")),
            },
            XyEvent::TextDelta("a".into()),
            XyEvent::ThinkingDelta("b".into()),
            XyEvent::ToolExecutionStart {
                id: "t".into(),
                name: "read".into(),
                args: serde_json::json!({"path": "a"}),
            },
            XyEvent::ToolExecutionUpdate {
                id: "t".into(),
                output: "o".into(),
            },
            XyEvent::ToolExecutionEnd {
                id: "t".into(),
                name: "read".into(),
                result: "r".into(),
                is_error: true,
            },
            XyEvent::CompactionStart {
                reason: "threshold".into(),
            },
            XyEvent::CompactionEnd {
                result: Some("r".into()),
                aborted: false,
                reason: "threshold".into(),
                will_retry: true,
                error_message: None,
                summary: Some("s".into()),
                tokens_before: Some(100),
                tokens_after: Some(40),
                notice: Some("n".into()),
            },
            XyEvent::ContextTokenSettlement {
                estimate: sample_estimate(),
                reason: "usage".into(),
                generation: 9,
            },
            XyEvent::ModelSelect {
                provider: "p".into(),
                model_id: "m".into(),
            },
            XyEvent::ThinkingLevelChanged {
                level: "high".into(),
            },
            XyEvent::QueueUpdate {
                steer_count: 1,
                follow_up_count: 2,
            },
            XyEvent::TodoUpdated { list: todo },
            XyEvent::AutoRetryStart {
                attempt: 1,
                max_retries: 3,
                delay_ms: 500,
            },
            XyEvent::AutoRetryEnd {
                success: true,
                attempt: 1,
            },
            XyEvent::Error(XyEventError::new("Provider", "boom")),
            XyEvent::SessionInfoChanged {
                key: "name".into(),
                value: serde_json::json!("demo"),
            },
        ];
        assert_eq!(cases.len(), 23);
        for case in &cases {
            let wire = event_to_wire(case).expect("to wire");
            let back = event_from_wire(&wire).expect("from wire");
            assert_eq!(
                format!("{back:?}"),
                format!("{case:?}"),
                "round trip {}",
                case.description()
            );
        }
    }

    #[test]
    fn agent_end_drops_nonempty_messages_on_the_wire_by_design() {
        let event = XyEvent::AgentEnd {
            messages: vec![AgentMessage::user("kept face-locally only")],
        };
        let wire = event_to_wire(&event).expect("to wire");
        assert_eq!(wire, Event::AgentEnd);
        let back = event_from_wire(&wire).expect("from wire");
        assert!(
            matches!(back, XyEvent::AgentEnd { ref messages } if messages.is_empty()),
            "wire AgentEnd must rebuild with empty messages, got {back:?}"
        );
    }

    #[test]
    fn unknown_wire_variant_is_reported_not_panicked() {
        let err = event_from_wire(&Event::Unknown).expect_err("unknown");
        assert!(matches!(err, WireEventError::Unknown));
    }

    #[test]
    fn todo_updated_keeps_items_field_by_field() {
        let list = DomainTodoList::new(vec![TodoItem {
            id: "a".into(),
            content: "c".into(),
            status: TodoStatus::Completed,
        }]);
        let wire = event_to_wire(&XyEvent::TodoUpdated { list: list.clone() }).expect("wire");
        let Event::TodoUpdated { list: wire_list } = &wire else {
            panic!("expected todo_updated");
        };
        assert_eq!(wire_list.items.len(), 1);
        assert_eq!(wire_list.items[0].id, "a");
        assert_eq!(wire_list.items[0].content, "c");
        let back = event_from_wire(&wire).expect("back");
        assert!(matches!(back, XyEvent::TodoUpdated { list: ref l } if *l == list));
    }
}
