use serde::Serialize;
use serde_json::Value;

use xylitol_plugin_sdk::session::TodoList as DomainTodoList;
use xylitol_plugin_sdk::{
    AgentMessage, ContextTokenEstimate, PromptCacheRead, TokenProvenance, XyEvent, XyEventError,
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
    match event {
        XyEvent::AgentStart { session_id, model } => Ok(Event::AgentStart {
            session_id: session_id.clone(),
            model: model.clone(),
        }),
        XyEvent::AgentEnd { .. } => Ok(Event::AgentEnd),
        XyEvent::TurnStart { turn_index } => Ok(Event::TurnStart {
            turn_index: *turn_index,
        }),
        XyEvent::TurnEnd { turn_index } => Ok(Event::TurnEnd {
            turn_index: *turn_index,
        }),
        XyEvent::MessageStart { role, message } => message_start_to_wire(role, message),
        XyEvent::MessageUpdate { message } => message_update_to_wire(message),
        XyEvent::MessageEnd { role, message } => message_end_to_wire(role, message),
        XyEvent::TextDelta(text) => Ok(Event::TextDelta { text: text.clone() }),
        XyEvent::ThinkingDelta(text) => Ok(Event::ThinkingDelta { text: text.clone() }),
        XyEvent::ToolExecutionStart { id, name, args } => Ok(Event::ToolExecutionStart {
            id: id.clone(),
            name: name.clone(),
            args: args.clone(),
        }),
        XyEvent::ToolExecutionUpdate { id, output } => Ok(Event::ToolExecutionUpdate {
            id: id.clone(),
            output: output.clone(),
        }),
        XyEvent::ToolExecutionEnd {
            id,
            name,
            result,
            is_error,
        } => Ok(Event::ToolExecutionEnd {
            id: id.clone(),
            name: name.clone(),
            result: result.clone(),
            is_error: *is_error,
        }),
        XyEvent::CompactionStart { reason } => Ok(Event::CompactionStart {
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
        } => Ok(Event::CompactionEnd {
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
        } => Ok(Event::ContextTokenSettlement {
            tokens: estimate.tokens,
            provenance: estimate.provenance.as_str().to_string(),
            usage_tokens: estimate.usage_tokens,
            trailing_tokens: estimate.trailing_tokens,
            last_usage_index: estimate.last_usage_index,
            prompt_cache: prompt_cache_status(estimate.prompt_cache_read).to_string(),
            prompt_cache_tokens: prompt_cache_tokens(estimate.prompt_cache_read),
            reason: reason.clone(),
            generation: *generation,
        }),
        XyEvent::ModelSelect { provider, model_id } => Ok(Event::ModelSelect {
            provider: provider.clone(),
            model_id: model_id.clone(),
        }),
        XyEvent::ThinkingLevelChanged { level } => Ok(Event::ThinkingLevelChanged {
            level: level.clone(),
        }),
        XyEvent::QueueUpdate {
            steer_count,
            follow_up_count,
        } => Ok(Event::QueueUpdate {
            steer_count: *steer_count,
            follow_up_count: *follow_up_count,
        }),
        XyEvent::TodoUpdated { list } => todo_updated_to_wire(list),
        XyEvent::PlanDocUpdated { doc } => plan_doc_updated_to_wire(doc),
        XyEvent::WorkflowUpdated { workflow } => workflow_updated_to_wire(workflow),
        XyEvent::PlanModeChanged { mode, plan_id } => plan_mode_changed_to_wire(mode, plan_id),
        XyEvent::AutoRetryStart {
            attempt,
            max_retries,
            delay_ms,
        } => Ok(Event::AutoRetryStart {
            attempt: *attempt,
            max_retries: *max_retries,
            delay_ms: *delay_ms,
        }),
        XyEvent::AutoRetryEnd { success, attempt } => Ok(Event::AutoRetryEnd {
            success: *success,
            attempt: *attempt,
        }),
        XyEvent::Error(err) => Ok(Event::Error {
            kind: err.kind.clone(),
            message: err.message.clone(),
        }),
        XyEvent::SessionInfoChanged { key, value } => Ok(Event::SessionInfoChanged {
            key: key.clone(),
            value: value.clone(),
        }),
    }
}

pub fn event_from_wire(event: &Event) -> Result<XyEvent, WireEventError> {
    match event {
        Event::AgentStart { session_id, model } => Ok(XyEvent::AgentStart {
            session_id: session_id.clone(),
            model: model.clone(),
        }),
        Event::AgentEnd => Ok(XyEvent::AgentEnd {
            messages: Vec::new(),
        }),
        Event::TurnStart { turn_index } => Ok(XyEvent::TurnStart {
            turn_index: *turn_index,
        }),
        Event::TurnEnd { turn_index } => Ok(XyEvent::TurnEnd {
            turn_index: *turn_index,
        }),
        Event::MessageStart { role, message } => message_start_from_wire(role, message),
        Event::MessageUpdate { message } => message_update_from_wire(message),
        Event::MessageEnd { role, message } => message_end_from_wire(role, message),
        Event::TextDelta { text } => Ok(XyEvent::TextDelta(text.clone())),
        Event::ThinkingDelta { text } => Ok(XyEvent::ThinkingDelta(text.clone())),
        Event::ToolExecutionStart { id, name, args } => Ok(XyEvent::ToolExecutionStart {
            id: id.clone(),
            name: name.clone(),
            args: args.clone(),
        }),
        Event::ToolExecutionUpdate { id, output } => Ok(XyEvent::ToolExecutionUpdate {
            id: id.clone(),
            output: output.clone(),
        }),
        Event::ToolExecutionEnd {
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
            prompt_cache,
            prompt_cache_tokens,
            reason,
            generation,
        } => Ok(XyEvent::ContextTokenSettlement {
            estimate: ContextTokenEstimate {
                tokens: *tokens,
                provenance: TokenProvenance::from_key(provenance),
                usage_tokens: *usage_tokens,
                trailing_tokens: *trailing_tokens,
                last_usage_index: *last_usage_index,
                prompt_cache_read: prompt_cache_from_wire(prompt_cache, *prompt_cache_tokens),
            },
            reason: reason.clone(),
            generation: *generation,
        }),
        Event::ModelSelect { provider, model_id } => Ok(XyEvent::ModelSelect {
            provider: provider.clone(),
            model_id: model_id.clone(),
        }),
        Event::ThinkingLevelChanged { level } => Ok(XyEvent::ThinkingLevelChanged {
            level: level.clone(),
        }),
        Event::QueueUpdate {
            steer_count,
            follow_up_count,
        } => Ok(XyEvent::QueueUpdate {
            steer_count: *steer_count,
            follow_up_count: *follow_up_count,
        }),
        Event::TodoUpdated { list } => todo_updated_from_wire(list),
        Event::PlanDocUpdated { doc } => plan_doc_updated_from_wire(doc),
        Event::WorkflowUpdated { workflow } => workflow_updated_from_wire(workflow),
        Event::PlanModeChanged { mode, plan_id } => plan_mode_changed_from_wire(mode, plan_id),
        Event::AutoRetryStart {
            attempt,
            max_retries,
            delay_ms,
        } => Ok(XyEvent::AutoRetryStart {
            attempt: *attempt,
            max_retries: *max_retries,
            delay_ms: *delay_ms,
        }),
        Event::AutoRetryEnd { success, attempt } => Ok(XyEvent::AutoRetryEnd {
            success: *success,
            attempt: *attempt,
        }),
        Event::Error { kind, message } => Ok(XyEvent::Error(XyEventError::new(kind, message))),
        Event::SessionInfoChanged { key, value } => Ok(XyEvent::SessionInfoChanged {
            key: key.clone(),
            value: value.clone(),
        }),
        Event::Unknown => Err(WireEventError::Unknown),
    }
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

fn prompt_cache_status(read: PromptCacheRead) -> &'static str {
    match read {
        PromptCacheRead::Tokens(_) => "hit",
        PromptCacheRead::NotReported => "unknown",
        PromptCacheRead::NotApplicable => "not_applicable",
    }
}

fn prompt_cache_tokens(read: PromptCacheRead) -> Option<u64> {
    match read {
        PromptCacheRead::Tokens(n) => Some(n),
        PromptCacheRead::NotReported | PromptCacheRead::NotApplicable => None,
    }
}

fn prompt_cache_from_wire(status: &str, tokens: Option<u64>) -> PromptCacheRead {
    match status {
        "hit" => tokens.map_or(PromptCacheRead::NotReported, PromptCacheRead::Tokens),
        "unknown" => PromptCacheRead::NotReported,
        "" | "not_applicable" => PromptCacheRead::NotApplicable,
        _ => PromptCacheRead::NotReported,
    }
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
            ..Default::default()
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
    fn settlement_prompt_cache_tri_state_round_trips() {
        for read in [
            PromptCacheRead::Tokens(7),
            PromptCacheRead::NotReported,
            PromptCacheRead::NotApplicable,
        ] {
            let mut estimate = sample_estimate();
            estimate.prompt_cache_read = read;
            let event = XyEvent::ContextTokenSettlement {
                estimate,
                reason: "turn_settled".into(),
                generation: 1,
            };
            let wire = event_to_wire(&event).expect("to wire");
            match &wire {
                Event::ContextTokenSettlement {
                    prompt_cache,
                    prompt_cache_tokens,
                    ..
                } => match read {
                    PromptCacheRead::Tokens(n) => {
                        assert_eq!(prompt_cache, "hit");
                        assert_eq!(*prompt_cache_tokens, Some(n));
                    }
                    PromptCacheRead::NotReported => {
                        assert_eq!(prompt_cache, "unknown");
                        assert_eq!(*prompt_cache_tokens, None);
                    }
                    PromptCacheRead::NotApplicable => {
                        assert_eq!(prompt_cache, "not_applicable");
                        assert_eq!(*prompt_cache_tokens, None);
                    }
                },
                other => panic!("expected settlement, got {other:?}"),
            }
            let back = event_from_wire(&wire).expect("from wire");
            match back {
                XyEvent::ContextTokenSettlement { estimate, .. } => {
                    assert_eq!(estimate.prompt_cache_read, read);
                }
                other => panic!("expected settlement, got {}", other.description()),
            }
        }

        let wire = Event::ContextTokenSettlement {
            tokens: 1,
            provenance: "Api".into(),
            usage_tokens: 1,
            trailing_tokens: 0,
            last_usage_index: None,
            prompt_cache: "mystery".into(),
            prompt_cache_tokens: Some(9),
            reason: "turn_settled".into(),
            generation: 1,
        };
        match event_from_wire(&wire).expect("unknown cache status") {
            XyEvent::ContextTokenSettlement { estimate, .. } => {
                assert_eq!(estimate.prompt_cache_read, PromptCacheRead::NotReported);
            }
            other => panic!("expected settlement, got {}", other.description()),
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

fn message_start_to_wire(
    role: &str,
    message: &Option<AgentMessage>,
) -> Result<Event, WireEventError> {
    Ok(Event::MessageStart {
        role: role.to_string(),
        message: to_value_opt(message)?,
    })
}

fn message_update_to_wire(message: &Option<AgentMessage>) -> Result<Event, WireEventError> {
    Ok(Event::MessageUpdate {
        message: to_value_opt(message)?,
    })
}

fn message_end_to_wire(
    role: &str,
    message: &Option<AgentMessage>,
) -> Result<Event, WireEventError> {
    Ok(Event::MessageEnd {
        role: role.to_string(),
        message: to_value_opt(message)?,
    })
}

fn todo_updated_to_wire(list: &DomainTodoList) -> Result<Event, WireEventError> {
    Ok(Event::TodoUpdated {
        list: todo_to_wire(list)?,
    })
}

fn plan_doc_updated_to_wire(
    doc: &xylitol_plugin_sdk::session::PlanDoc,
) -> Result<Event, WireEventError> {
    Ok(Event::PlanDocUpdated {
        doc: serde_json::to_value(doc)?,
    })
}

fn workflow_updated_to_wire(
    workflow: &xylitol_plugin_sdk::session::Workflow,
) -> Result<Event, WireEventError> {
    Ok(Event::WorkflowUpdated {
        workflow: serde_json::to_value(workflow)?,
    })
}

fn plan_mode_changed_to_wire(
    mode: &xylitol_plugin_sdk::session::PlanModeKind,
    plan_id: &Option<String>,
) -> Result<Event, WireEventError> {
    Ok(Event::PlanModeChanged {
        mode: serde_json::to_value(mode)?
            .as_str()
            .unwrap_or("execute")
            .to_string(),
        plan_id: plan_id.clone(),
    })
}

fn message_start_from_wire(role: &str, message: &Option<Value>) -> Result<XyEvent, WireEventError> {
    Ok(XyEvent::MessageStart {
        role: role.to_string(),
        message: from_value_opt(message)?,
    })
}

fn message_update_from_wire(message: &Option<Value>) -> Result<XyEvent, WireEventError> {
    Ok(XyEvent::MessageUpdate {
        message: from_value_opt(message)?,
    })
}

fn message_end_from_wire(role: &str, message: &Option<Value>) -> Result<XyEvent, WireEventError> {
    Ok(XyEvent::MessageEnd {
        role: role.to_string(),
        message: from_value_opt(message)?,
    })
}

fn todo_updated_from_wire(list: &WireTodoList) -> Result<XyEvent, WireEventError> {
    Ok(XyEvent::TodoUpdated {
        list: todo_from_wire(list)?,
    })
}

fn plan_doc_updated_from_wire(doc: &Value) -> Result<XyEvent, WireEventError> {
    Ok(XyEvent::PlanDocUpdated {
        doc: serde_json::from_value::<xylitol_plugin_sdk::session::PlanDoc>(doc.clone())?,
    })
}

fn workflow_updated_from_wire(workflow: &Value) -> Result<XyEvent, WireEventError> {
    Ok(XyEvent::WorkflowUpdated {
        workflow: serde_json::from_value::<xylitol_plugin_sdk::session::Workflow>(
            workflow.clone(),
        )?,
    })
}

fn plan_mode_changed_from_wire(
    mode: &str,
    plan_id: &Option<String>,
) -> Result<XyEvent, WireEventError> {
    Ok(XyEvent::PlanModeChanged {
        mode: serde_json::from_value(Value::String(mode.to_string()))?,
        plan_id: plan_id.clone(),
    })
}
