//! Typed method registry — unary 方法接缝的 SSOT（c2530）。
//!
//! 一行声明一个方法的能力位：租约准入（[`Auth`]）、幂等准入（[`Idem`]）、
//! 响应种类（[`Resp`]）、是否有对应 [`Command`] 变体、以及迁移期（design D4）
//! 的载荷解析路径开关。命令词表本身仍是 [`Command`]（port 级 SSOT）；
//! 本表通过 tag-injection 让 serde 承担载荷解析，字段名只存在于 Command 上。
//! 方法名清单（oapi / 404 语义）从本表派生，不再有第二份字符串表。

use serde_json::{Value, json};

use crate::protocol::Command;

/// 写者租约准入：`Writer` 走 `WriterLease`，`Readonly` 无租约。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    Readonly,
    Writer,
}

/// 幂等准入（c2460）：`PerRpc` 正常 admit；`Bypass` 由宿主在 admit 之前短路
/// （现为 host.describe / reload / loaded_resources 三个 pre-admit 分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idem {
    PerRpc,
    Bypass,
}

/// 响应种类（design D3/D5）：`Job` 仅枚举占位，作业语义另行立项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resp {
    Result,
    /// ack + 事件走 mux（现为 prompt：spawn 后立即返回）。
    Stream,
    Job,
}

/// 执行类（c2780）：一条命令相对会话循环的生效时机。
///
/// - `Exclusive`：独占会话循环直至完成（prompt / bash / reload）；
/// - `Inline`：任何交互循环（含 bang / reload select）内立即执行生效——
///   effect MUST 非阻塞（缓存 / 本地直出）；
/// - `Queued`：排队至循环归还后由主循环 `drain_pending` 处理（默认）。
///
/// 消费只走 [`exec_class`]（无通配穷举 match：新增 [`Command`] 变体不声明
/// 执行类即编译失败）；本表行与推导的一致性由守卫测试锁定。
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
    /// 存在同名 serde tag 的 [`Command`] 变体，tag-injection 解析可路由。
    pub command_backed: bool,
    /// 执行类（c2780）。经 [`m`] 声明的行默认 [`Exec::Queued`]。
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

/// `m` 的显式执行类形式：仅 `Inline` / `Exclusive` 行使用（c2780）。
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

/// 行尾 `command_backed` 的命名形式：存在同名 serde tag 的 Command 变体。
const CMD: bool = true;
/// 非 Command 特例方法（host.describe / arm_tool_freeze / persist_trust）。
const RAW: bool = false;

/// 方法名词表单一定义点：REGISTRY 行与 host 分派共用；改名只动此处。
pub const METHOD_PROMPT: &str = "prompt";
pub const METHOD_ABORT: &str = "abort";
pub const METHOD_GET_STATE: &str = "get_state";
pub const METHOD_SET_MODEL: &str = "set_model";
pub const METHOD_CYCLE_MODEL: &str = "cycle_model";
pub const METHOD_GET_AVAILABLE_MODELS: &str = "get_available_models";
pub const METHOD_SET_THINKING_LEVEL: &str = "set_thinking_level";
pub const METHOD_BASH: &str = "bash";
pub const METHOD_COMPACT: &str = "compact";
pub const METHOD_GET_SESSION_STATS: &str = "get_session_stats";
pub const METHOD_EXPORT_HTML: &str = "export_html";
pub const METHOD_EXPORT_JSONL: &str = "export_jsonl";
pub const METHOD_IMPORT_JSONL: &str = "import_jsonl";
pub const METHOD_SWITCH_SESSION: &str = "switch_session";
pub const METHOD_FORK: &str = "fork";
pub const METHOD_GET_MESSAGES: &str = "get_messages";
pub const METHOD_GET_COMMANDS: &str = "get_commands";
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

/// 顺序与 `UNARY_METHODS` 保持一致（守卫测试锁定）。
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
    mx(
        METHOD_GET_COMMANDS,
        Auth::Readonly,
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
    // pre-admit 短路：见 handle_unary 的三个早期分支（Idem::Bypass）。
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
    // subscribe：无租约位（不在 WRITER_METHODS），但 materialize_writer_at 语义留在自身分支。
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

/// 注册表派生的方法名清单（404 语义 / OpenAPI 条目的唯一来源）。
pub fn names() -> impl Iterator<Item = &'static str> {
    REGISTRY.iter().map(|e| e.name)
}

/// [`Command`] 的执行类（c2780）：与 REGISTRY 行的 [`Exec`] 声明同源，
/// 一致性由守卫测试锁定。无通配穷举 match——新增 [`Command`] 变体不在
/// 此分类即编译失败，执行语义由编译器强制成为命令的一等声明。
pub fn exec_class(cmd: &Command) -> Exec {
    match cmd {
        Command::Prompt { .. } | Command::Bash { .. } | Command::Reload { .. } => Exec::Exclusive,
        Command::GetState { .. }
        | Command::SetModel { .. }
        | Command::CycleModel { .. }
        | Command::GetAvailableModels { .. }
        | Command::SetThinkingLevel { .. }
        | Command::GetQueueStats { .. }
        | Command::GetCommands { .. }
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

/// 由方法名 + 载荷构造 [`Command`]：注入 serde tag 后交给 Command 自己的
/// derive 解析。字段名真值只在 Command 上；载荷内未知键（含历史遗留的
/// `id`）由 serde 忽略——传输级关联唯一走信封 rpcId（c2460）。
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
                    // 字段缺失是预期（载荷为空）；tag 解析失败才是漂移。
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
                "bash",
                "compact",
                "export_html",
                "export_jsonl",
                "import_jsonl",
                "switch_session",
                "fork",
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
            // c2710: tags match registry names directly except GetQueueStats,
            // whose explicit serde rename is `queue_stats`.
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
                "{v} 声明为非 wire 面，却存在 registry 行"
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
        // 手搓表：缺 provider → 空串；缺 model_id → 报错。
        assert!(matches!(
            parse_command("set_model", &json!({"model_id": "m"})),
            Ok(Command::SetModel { provider, .. }) if provider.is_empty()
        ));
        assert!(parse_command("set_model", &json!({})).is_err());
        // 手搓表：缺 kind → message_history。
        assert!(matches!(
            parse_command("session_tree", &json!({})),
            Ok(Command::SessionTree { .. })
        ));
        // 载荷携带的 id 字段现在只是未知键，serde 忽略之。
        assert!(matches!(
            parse_command("get_state", &json!({"id": "x"})),
            Ok(Command::GetState {})
        ));
        // sr-imp1：input_path 直传不受暂存引入影响；content-only 走 Host 暂存（parse 层不做）。
        assert!(matches!(
            parse_command("import_jsonl", &json!({"input_path": "a.jsonl"})),
            Ok(Command::ImportJsonl { .. })
        ));
        assert!(parse_command("import_jsonl", &json!({})).is_err());
    }

    /// c2780：执行类列快照锁——REGISTRY 行的 Inline/Exclusive 名单变动必须
    /// 是有意的规格变更；`exec_class` 侧由无通配穷举 match 由编译器强制。
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
                "cycle_model",
                "get_available_models",
                "get_commands",
                "get_state",
                "loaded_resources",
                "queue_stats",
                "set_model",
                "set_session_name",
                "set_session_name_for",
                "set_thinking_level",
            ]
            .as_slice()
        );
    }

    /// c2780：`exec_class` 推导与 REGISTRY 行声明同源——command_backed 行
    /// 逐一对照（Command 实例手工构造成本高，按代表 + 类全集名单双保险）。
    #[test]
    fn exec_class_matches_registry_declaration() {
        use crate::protocol::session::SessionTreeKind;

        // atm18：逐变体穷举对照——每个 command_backed REGISTRY 行的 Exec 列
        // MUST 与 `exec_class` 推导一致（Quit 无 wire 行，单独断言 Queued）。
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
            ("get_messages", Command::GetMessages {}),
            ("get_commands", Command::GetCommands {}),
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
        ];
        // NON_WIRE（ApproveTool / AnswerQuestion / Quit）无 wire 行：approve /
        // answer 走反向 RPC 响应路径、Quit 为客户端本地命令，均 Queued 语义。
        assert_eq!(cases.len(), 34, "all command_backed rows must be covered");
        for (method, cmd) in &cases {
            let row = lookup(method).expect("row exists for command_backed method");
            assert!(row.command_backed, "row {method} must be command_backed");
            assert_eq!(
                row.exec,
                exec_class(cmd),
                "row {method} Exec column vs exec_class() drift"
            );
        }
        // REGISTRY 行与 Command 变体一一对应（无漏行）：名单即上表 method 集合。
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
