use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::TodoList;

fn default_error_kind() -> String {
    "Message".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    AgentStart {
        session_id: String,
        model: String,
    },
    AgentEnd,
    TurnStart {
        turn_index: u32,
    },
    TurnEnd {
        turn_index: u32,
    },
    MessageStart {
        role: String,
        #[serde(default)]
        message: Option<Value>,
    },
    MessageEnd {
        role: String,
        #[serde(default)]
        message: Option<Value>,
    },
    MessageUpdate {
        #[serde(default)]
        message: Option<Value>,
    },
    TextDelta {
        text: String,
    },
    ThinkingDelta {
        text: String,
    },
    ToolExecutionStart {
        id: String,
        name: String,
        args: Value,
    },
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    ToolExecutionEnd {
        id: String,
        name: String,
        result: String,
        is_error: bool,
    },
    CompactionStart {
        reason: String,
    },
    CompactionEnd {
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        aborted: bool,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        will_retry: bool,
        #[serde(default)]
        error_message: Option<String>,
        #[serde(default)]
        summary: Option<String>,
        #[serde(default)]
        tokens_before: Option<u64>,
        #[serde(default)]
        tokens_after: Option<u64>,
        #[serde(default)]
        notice: Option<String>,
    },
    ContextTokenSettlement {
        tokens: u64,
        provenance: String,
        usage_tokens: u64,
        trailing_tokens: u64,
        #[serde(default)]
        last_usage_index: Option<usize>,
        #[serde(default)]
        prompt_cache: String,
        #[serde(default)]
        prompt_cache_tokens: Option<u64>,
        reason: String,
        generation: u64,
    },
    ModelSelect {
        provider: String,
        model_id: String,
    },
    ThinkingLevelChanged {
        level: String,
    },
    QueueUpdate {
        steer_count: usize,
        follow_up_count: usize,
    },
    TodoUpdated {
        list: TodoList,
    },
    PlanDocUpdated {
        doc: Value,
    },
    WorkflowUpdated {
        workflow: Value,
    },
    PlanModeChanged {
        mode: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        plan_id: Option<String>,
    },
    AutoRetryStart {
        attempt: u32,
        max_retries: u32,
        delay_ms: u64,
    },
    AutoRetryEnd {
        success: bool,
        attempt: u32,
    },
    Error {
        #[serde(default = "default_error_kind")]
        kind: String,
        message: String,
    },
    SessionInfoChanged {
        key: String,
        value: Value,
    },
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_execution_tags_are_snake_case() {
        let ev = Event::ToolExecutionStart {
            id: "t1".into(),
            name: "read".into(),
            args: serde_json::json!({"path": "a.rs"}),
        };
        let v = serde_json::to_value(&ev).expect("json");
        assert_eq!(v["type"], "tool_execution_start");
        let back: Event = serde_json::from_value(v).expect("round trip");
        assert_eq!(back, ev);
    }

    #[test]
    fn unknown_type_degrades_to_unknown() {
        let ev: Event = serde_json::from_str(r#"{"type":"future_variant","x":1}"#).expect("ok");
        assert_eq!(ev, Event::Unknown);
    }

    #[test]
    fn error_kind_defaults_when_absent() {
        let ev: Event = serde_json::from_str(r#"{"type":"error","message":"boom"}"#).expect("ok");
        assert_eq!(
            ev,
            Event::Error {
                kind: "Message".into(),
                message: "boom".into()
            }
        );
    }

    #[test]
    fn tool_end_requires_is_error() {
        let missing = r#"{"type":"tool_execution_end","id":"t","name":"n","result":"r"}"#;
        assert!(serde_json::from_str::<Event>(missing).is_err());
    }

    #[test]
    fn every_variant_round_trips_through_json() {
        let samples = vec![
            Event::AgentStart {
                session_id: "s".into(),
                model: "m".into(),
            },
            Event::AgentEnd,
            Event::TurnStart { turn_index: 1 },
            Event::TurnEnd { turn_index: 1 },
            Event::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            Event::MessageEnd {
                role: "assistant".into(),
                message: Some(serde_json::json!({"k": 1})),
            },
            Event::MessageUpdate { message: None },
            Event::TextDelta { text: "d".into() },
            Event::ThinkingDelta { text: "d".into() },
            Event::ToolExecutionStart {
                id: "t1".into(),
                name: "read".into(),
                args: serde_json::json!({}),
            },
            Event::ToolExecutionUpdate {
                id: "t1".into(),
                output: "o".into(),
            },
            Event::ToolExecutionEnd {
                id: "t1".into(),
                name: "read".into(),
                result: "r".into(),
                is_error: false,
            },
            Event::CompactionStart {
                reason: "auto".into(),
            },
            Event::CompactionEnd {
                result: None,
                aborted: true,
                reason: "r".into(),
                will_retry: false,
                error_message: None,
                summary: Some("s".into()),
                tokens_before: Some(10),
                tokens_after: Some(5),
                notice: None,
            },
            Event::ContextTokenSettlement {
                tokens: 1,
                provenance: "p".into(),
                usage_tokens: 2,
                trailing_tokens: 3,
                last_usage_index: Some(4),
                prompt_cache: "hit".into(),
                prompt_cache_tokens: Some(2),
                reason: "r".into(),
                generation: 5,
            },
            Event::ModelSelect {
                provider: "p".into(),
                model_id: "m".into(),
            },
            Event::ThinkingLevelChanged {
                level: "high".into(),
            },
            Event::QueueUpdate {
                steer_count: 1,
                follow_up_count: 2,
            },
            Event::TodoUpdated {
                list: TodoList::default(),
            },
            Event::AutoRetryStart {
                attempt: 1,
                max_retries: 3,
                delay_ms: 100,
            },
            Event::AutoRetryEnd {
                success: true,
                attempt: 1,
            },
            Event::Error {
                kind: "Message".into(),
                message: "boom".into(),
            },
            Event::SessionInfoChanged {
                key: "k".into(),
                value: serde_json::json!(1),
            },
            Event::Unknown,
        ];
        for ev in samples {
            let wire = serde_json::to_value(&ev).expect("serialize");
            let back: Event = serde_json::from_value(wire.clone()).expect("deserialize");
            assert_eq!(back, ev, "round trip of {wire}");
        }
    }
}
