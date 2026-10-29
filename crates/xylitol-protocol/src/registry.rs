use serde_json::{Value, json};

use crate::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    Readonly,
    Writer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idem {
    PerRpc,
    Bypass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resp {
    Result,
    Stream,
    Job,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exec {
    Exclusive,
    Inline,
    Queued,
}

#[derive(Debug, Clone, Copy)]
pub struct MethodEntry {
    pub name: &'static str,
    pub auth: Auth,
    pub idem: Idem,
    pub resp: Resp,
    pub command_backed: bool,
    pub exec: Exec,
}

const fn m(
    name: &'static str,
    auth: Auth,
    idem: Idem,
    resp: Resp,
    command_backed: bool,
) -> MethodEntry {
    MethodEntry {
        name,
        auth,
        idem,
        resp,
        command_backed,
        exec: Exec::Queued,
    }
}

const fn mx(
    name: &'static str,
    auth: Auth,
    idem: Idem,
    resp: Resp,
    command_backed: bool,
    exec: Exec,
) -> MethodEntry {
    MethodEntry {
        name,
        auth,
        idem,
        resp,
        command_backed,
        exec,
    }
}

const CMD: bool = true;
const RAW: bool = false;

pub const METHOD_PROMPT: &str = "prompt";
pub const METHOD_ABORT: &str = "abort";
pub const METHOD_GET_STATE: &str = "get_state";
pub const METHOD_SET_MODEL: &str = "set_model";
pub const METHOD_CYCLE_MODEL: &str = "cycle_model";
pub const METHOD_GET_AVAILABLE_MODELS: &str = "get_available_models";
pub const METHOD_SET_THINKING_LEVEL: &str = "set_thinking_level";
pub const METHOD_SET_PLAN_MODE: &str = "set_plan_mode";
pub const METHOD_BASH: &str = "bash";
pub const METHOD_COMPACT: &str = "compact";
pub const METHOD_GET_SESSION_STATS: &str = "get_session_stats";
pub const METHOD_EXPORT_HTML: &str = "export_html";
pub const METHOD_EXPORT_JSONL: &str = "export_jsonl";
pub const METHOD_IMPORT_JSONL: &str = "import_jsonl";
pub const METHOD_SWITCH_SESSION: &str = "switch_session";
pub const METHOD_FORK: &str = "fork";
pub const METHOD_GET_MESSAGES: &str = "get_messages";
pub const METHOD_ESTIMATE_CONTEXT: &str = "estimate_context";
pub const METHOD_GET_COMMANDS: &str = "get_commands";
pub const METHOD_GET_CAPABILITIES: &str = "get_capabilities";
pub const METHOD_SET_CAPABILITY: &str = "set_capability";
pub const METHOD_CLEAR_CAPABILITIES: &str = "clear_capabilities";
pub const METHOD_SESSION_TREE: &str = "session_tree";
pub const METHOD_TRAVEL_SESSION_TREE: &str = "travel_session_tree";
pub const METHOD_APPEND_ENTRY_LABEL: &str = "append_entry_label";
pub const METHOD_LIST_SESSIONS: &str = "list_sessions";
pub const METHOD_LOAD_SESSION_ENTRIES: &str = "load_session_entries";
pub const METHOD_NEW_SESSION: &str = "new_session";
pub const METHOD_GET_SESSION_NAME: &str = "get_session_name";
pub const METHOD_SET_SESSION_NAME: &str = "set_session_name";
pub const METHOD_SET_SESSION_NAME_FOR: &str = "set_session_name_for";
pub const METHOD_DELETE_SESSION: &str = "delete_session";
pub const METHOD_RELOAD: &str = "reload";
pub const METHOD_LOADED_RESOURCES: &str = "loaded_resources";
pub const METHOD_QUEUE_STATS: &str = "queue_stats";
pub const METHOD_ARM_TOOL_FREEZE: &str = "arm_tool_freeze";
pub const METHOD_PERSIST_TRUST: &str = "persist_trust";
pub const METHOD_STEER: &str = "steer";
pub const METHOD_FOLLOW_UP: &str = "follow_up";
pub const METHOD_CLEAR_QUEUE: &str = "clear_queue";
pub const METHOD_SUBSCRIBE: &str = "subscribe";
pub const METHOD_HOST_DESCRIBE: &str = "host.describe";

pub const REGISTRY: &[MethodEntry] = &[
    mx(
        METHOD_PROMPT,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Stream,
        CMD,
        Exec::Exclusive,
    ),
    m(METHOD_ABORT, Auth::Writer, Idem::PerRpc, Resp::Result, CMD),
    mx(
        METHOD_GET_STATE,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_SET_MODEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        CMD,
        Exec::Inline,
    ),
    mx(
        METHOD_CYCLE_MODEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_GET_AVAILABLE_MODELS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_SET_THINKING_LEVEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_SET_PLAN_MODE,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        CMD,
        Exec::Inline,
    ),
    mx(
        METHOD_BASH,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        CMD,
        Exec::Exclusive,
    ),
    m(
        METHOD_COMPACT,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        CMD,
    ),
    m(
        METHOD_GET_SESSION_STATS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_EXPORT_HTML,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_EXPORT_JSONL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_IMPORT_JSONL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_SWITCH_SESSION,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(METHOD_FORK, Auth::Writer, Idem::PerRpc, Resp::Result, CMD),
    m(
        METHOD_GET_MESSAGES,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_ESTIMATE_CONTEXT,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    mx(
        METHOD_GET_COMMANDS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_GET_CAPABILITIES,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_SET_CAPABILITY,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_CLEAR_CAPABILITIES,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    m(
        METHOD_SESSION_TREE,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_TRAVEL_SESSION_TREE,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    mx(
        METHOD_APPEND_ENTRY_LABEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    m(
        METHOD_LIST_SESSIONS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_LOAD_SESSION_ENTRIES,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_NEW_SESSION,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_GET_SESSION_NAME,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    mx(
        METHOD_SET_SESSION_NAME,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_SET_SESSION_NAME_FOR,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    m(
        METHOD_DELETE_SESSION,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    mx(
        METHOD_RELOAD,
        Auth::Readonly,
        Idem::Bypass,
        Resp::Result,
        CMD,
        Exec::Exclusive,
    ),
    mx(
        METHOD_LOADED_RESOURCES,
        Auth::Readonly,
        Idem::Bypass,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    mx(
        METHOD_QUEUE_STATS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
        Exec::Inline,
    ),
    m(
        METHOD_ARM_TOOL_FREEZE,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        RAW,
    ),
    m(
        METHOD_PERSIST_TRUST,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        RAW,
    ),
    m(METHOD_STEER, Auth::Writer, Idem::PerRpc, Resp::Result, CMD),
    m(
        METHOD_FOLLOW_UP,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        CMD,
    ),
    m(
        METHOD_CLEAR_QUEUE,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_SUBSCRIBE,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_HOST_DESCRIBE,
        Auth::Readonly,
        Idem::Bypass,
        Resp::Result,
        RAW,
    ),
];

pub fn lookup(name: &str) -> Option<&'static MethodEntry> {
    REGISTRY.iter().find(|e| e.name == name)
}

pub fn names() -> impl Iterator<Item = &'static str> {
    REGISTRY.iter().map(|e| e.name)
}

pub fn exec_class(cmd: &Command) -> Exec {
    match cmd {
        Command::Prompt { .. } | Command::Bash { .. } | Command::Reload { .. } => Exec::Exclusive,
        Command::GetState { .. }
        | Command::SetModel { .. }
        | Command::CycleModel { .. }
        | Command::GetAvailableModels { .. }
        | Command::SetThinkingLevel { .. }
        | Command::SetPlanMode { .. }
        | Command::GetQueueStats { .. }
        | Command::GetCommands { .. }
        | Command::GetCapabilities { .. }
        | Command::SetCapability { .. }
        | Command::ClearCapabilities { .. }
        | Command::LoadedResources { .. }
        | Command::AppendEntryLabel { .. }
        | Command::SetSessionName { .. }
        | Command::SetSessionNameFor { .. } => Exec::Inline,
        Command::Abort { .. }
        | Command::GetSessionStats { .. }
        | Command::Compact { .. }
        | Command::ExportHtml { .. }
        | Command::ExportJsonl { .. }
        | Command::ImportJsonl { .. }
        | Command::SwitchSession { .. }
        | Command::Fork { .. }
        | Command::GetMessages { .. }
        | Command::EstimateContext { .. }
        | Command::SessionTree { .. }
        | Command::TravelSessionTree { .. }
        | Command::ListSessions { .. }
        | Command::LoadSessionEntries { .. }
        | Command::NewSession { .. }
        | Command::GetSessionName { .. }
        | Command::DeleteSession { .. }
        | Command::Steer { .. }
        | Command::FollowUp { .. }
        | Command::ClearQueue { .. }
        | Command::Subscribe { .. }
        | Command::ApproveTool { .. }
        | Command::AnswerQuestion { .. }
        | Command::Quit { .. } => Exec::Queued,
    }
}

pub fn parse_command(method: &str, payload: &Value) -> Result<Command, String> {
    let mut merged = serde_json::Map::new();
    merged.insert("type".into(), json!(method));
    if let Value::Object(map) = payload {
        for (k, v) in map {
            merged.insert(k.clone(), v.clone());
        }
    }
    serde_json::from_value(Value::Object(merged)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use strum::VariantNames;

    #[test]
    fn command_backed_tags_resolve_via_serde() {
        for entry in REGISTRY.iter().filter(|e| e.command_backed) {
            match parse_command(entry.name, &json!({})) {
                Ok(_) => {}
                Err(msg) => {
                    assert!(
                        !msg.contains("unknown variant"),
                        "method {} tag drift: {msg}",
                        entry.name
                    );
                }
            }
        }
    }

    #[test]
    fn writer_set_and_bypass_set_match_declaration() {
        let writers: Vec<_> = REGISTRY
            .iter()
            .filter(|e| e.auth == Auth::Writer)
            .map(|e| e.name)
            .collect();
        assert_eq!(
            writers,
            vec![
                "prompt",
                "abort",
                "set_model",
                "cycle_model",
                "set_thinking_level",
                "set_plan_mode",
                "bash",
                "compact",
                "export_html",
                "export_jsonl",
                "import_jsonl",
                "switch_session",
                "fork",
                "set_capability",
                "clear_capabilities",
                "travel_session_tree",
                "append_entry_label",
                "new_session",
                "set_session_name",
                "set_session_name_for",
                "delete_session",
                "arm_tool_freeze",
                "persist_trust",
                "steer",
                "follow_up",
                "clear_queue",
            ]
        );
        let bypass: Vec<_> = REGISTRY
            .iter()
            .filter(|e| e.idem == Idem::Bypass)
            .map(|e| e.name)
            .collect();
        assert_eq!(bypass, vec!["reload", "loaded_resources", "host.describe"]);
    }

    #[test]
    fn every_wire_command_variant_has_registry_row() {
        const NON_WIRE: &[&str] = &["ApproveTool", "AnswerQuestion", "Quit"];
        let mut wired = 0;
        for variant in Command::VARIANTS {
            if NON_WIRE.contains(variant) {
                continue;
            }
            let snake = camel_to_snake(variant);
            let method = match snake.as_str() {
                "get_queue_stats" => "queue_stats",
                other => other,
            };
            assert!(
                lookup(method).is_some(),
                "Command::{variant} 没有 registry 行（wire 方法 `{method}` 未注册）"
            );
            wired += 1;
        }
        assert_eq!(
            wired,
            REGISTRY.len() - 3,
            "command_backed 行数应与 wire 变体数一致"
        );
        for v in NON_WIRE {
            assert!(
                !lookup(&camel_to_snake(v)).is_some(),
                "{v} 声明为非 wire 方法，却存在 registry 行"
            );
        }
    }

    fn camel_to_snake(name: &str) -> String {
        let mut out = String::with_capacity(name.len() + 4);
        for (i, ch) in name.chars().enumerate() {
            if ch.is_ascii_uppercase() {
                if i != 0 {
                    out.push('_');
                }
                out.extend(ch.to_lowercase());
            } else {
                out.push(ch);
            }
        }
        out
    }

    #[test]
    fn parse_accepts_primary_wire_keys() {
        assert!(matches!(
            parse_command("export_html", &json!({"output_path": "a.html"})),
            Ok(Command::ExportHtml {
                output_path: Some(_),
                ..
            })
        ));
        assert!(matches!(
            parse_command("switch_session", &json!({"session_path": "s.jsonl"})),
            Ok(Command::SwitchSession { .. })
        ));
        assert!(matches!(
            parse_command("queue_stats", &json!({})),
            Ok(Command::GetQueueStats { .. })
        ));
        assert!(matches!(
            parse_command("set_model", &json!({"model_id": "m"})),
            Ok(Command::SetModel { provider, .. }) if provider.is_empty()
        ));
        assert!(parse_command("set_model", &json!({})).is_err());
        assert!(matches!(
            parse_command("session_tree", &json!({})),
            Ok(Command::SessionTree { .. })
        ));
        assert!(matches!(
            parse_command("get_state", &json!({"id": "x"})),
            Ok(Command::GetState {})
        ));
        assert!(matches!(
            parse_command("import_jsonl", &json!({"input_path": "a.jsonl"})),
            Ok(Command::ImportJsonl { .. })
        ));
        assert!(parse_command("import_jsonl", &json!({})).is_err());
    }

    #[test]
    fn exec_columns_lock_declaration() {
        let exec_rows = |class| {
            let mut v: Vec<_> = REGISTRY
                .iter()
                .filter(|e| e.exec == class)
                .map(|e| e.name)
                .collect();
            v.sort_unstable();
            v
        };
        assert_eq!(
            exec_rows(Exec::Exclusive),
            ["bash", "prompt", "reload"].as_slice()
        );
        assert_eq!(
            exec_rows(Exec::Inline),
            [
                "append_entry_label",
                "clear_capabilities",
                "cycle_model",
                "get_available_models",
                "get_capabilities",
                "get_commands",
                "get_state",
                "loaded_resources",
                "queue_stats",
                "set_capability",
                "set_model",
                "set_plan_mode",
                "set_session_name",
                "set_session_name_for",
                "set_thinking_level",
            ]
            .as_slice()
        );
    }

    #[test]
    fn exec_class_matches_registry_declaration() {
        use crate::SessionTreeKind;

        let cases: Vec<(&str, Command)> = vec![
            (
                "prompt",
                Command::Prompt {
                    message: String::new(),
                },
            ),
            ("abort", Command::Abort {}),
            ("get_state", Command::GetState {}),
            (
                "set_model",
                Command::SetModel {
                    provider: String::new(),
                    model_id: String::new(),
                },
            ),
            ("cycle_model", Command::CycleModel {}),
            ("get_available_models", Command::GetAvailableModels {}),
            (
                "set_thinking_level",
                Command::SetThinkingLevel {
                    level: String::new(),
                },
            ),
            (
                "bash",
                Command::Bash {
                    command: String::new(),
                    exclude_from_context: false,
                },
            ),
            ("compact", Command::Compact { instructions: None }),
            ("get_session_stats", Command::GetSessionStats {}),
            ("export_html", Command::ExportHtml { output_path: None }),
            ("export_jsonl", Command::ExportJsonl { output_path: None }),
            (
                "import_jsonl",
                Command::ImportJsonl {
                    input_path: String::new(),
                },
            ),
            (
                "switch_session",
                Command::SwitchSession {
                    session_path: String::new(),
                },
            ),
            (
                "fork",
                Command::Fork {
                    entry_id: String::new(),
                    position: None,
                },
            ),
            (
                "get_messages",
                Command::GetMessages {
                    limit: None,
                    before_entry_id: None,
                },
            ),
            ("estimate_context", Command::EstimateContext {}),
            ("get_commands", Command::GetCommands {}),
            ("get_capabilities", Command::GetCapabilities {}),
            (
                "set_capability",
                Command::SetCapability {
                    id: String::new(),
                    enabled: false,
                },
            ),
            ("clear_capabilities", Command::ClearCapabilities {}),
            (
                "session_tree",
                Command::SessionTree {
                    kind: SessionTreeKind::MessageHistory,
                },
            ),
            (
                "travel_session_tree",
                Command::TravelSessionTree {
                    kind: SessionTreeKind::MessageHistory,
                    entry_id: String::new(),
                },
            ),
            (
                "append_entry_label",
                Command::AppendEntryLabel {
                    target_id: String::new(),
                    label: None,
                },
            ),
            ("list_sessions", Command::ListSessions {}),
            (
                "load_session_entries",
                Command::LoadSessionEntries {
                    session_id: String::new(),
                },
            ),
            ("new_session", Command::NewSession {}),
            ("get_session_name", Command::GetSessionName {}),
            (
                "set_session_name",
                Command::SetSessionName {
                    name: String::new(),
                },
            ),
            (
                "set_session_name_for",
                Command::SetSessionNameFor {
                    session_id: String::new(),
                    name: String::new(),
                },
            ),
            (
                "delete_session",
                Command::DeleteSession {
                    session_id: String::new(),
                },
            ),
            ("reload", Command::Reload {}),
            ("loaded_resources", Command::LoadedResources {}),
            ("queue_stats", Command::GetQueueStats {}),
            (
                "steer",
                Command::Steer {
                    message: String::new(),
                },
            ),
            (
                "follow_up",
                Command::FollowUp {
                    message: String::new(),
                },
            ),
            (
                "clear_queue",
                Command::ClearQueue {
                    clear_steer: true,
                    clear_follow_up: true,
                },
            ),
            (
                "subscribe",
                Command::Subscribe {
                    session_id: String::new(),
                    last_seq: 0,
                },
            ),
            (
                "set_plan_mode",
                Command::SetPlanMode {
                    mode: "toggle".into(),
                    plan_id: None,
                },
            ),
        ];
        assert_eq!(cases.len(), 39, "all command_backed rows must be covered");
        for (method, cmd) in &cases {
            let row = lookup(method).expect("row exists for command_backed method");
            assert!(row.command_backed, "row {method} must be command_backed");
            assert_eq!(
                row.exec,
                exec_class(cmd),
                "row {method} Exec column vs exec_class() drift"
            );
        }
        let backed_rows: Vec<&'static str> = REGISTRY
            .iter()
            .filter(|e| e.command_backed)
            .map(|e| e.name)
            .collect();
        assert_eq!(backed_rows.len(), cases.len(), "row/variant count drift");
        assert_eq!(exec_class(&Command::Quit {}), Exec::Queued);
        assert_eq!(
            exec_class(&Command::ApproveTool {
                call_id: String::new(),
                approved: true,
            }),
            Exec::Queued
        );
        assert_eq!(
            exec_class(&Command::AnswerQuestion {
                call_id: String::new(),
                answer: String::new(),
            }),
            Exec::Queued
        );
    }
}
