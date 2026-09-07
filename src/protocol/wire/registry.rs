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

#[derive(Debug, Clone, Copy)]
pub struct MethodEntry {
    pub name: &'static str,
    pub auth: Auth,
    pub idem: Idem,
    pub resp: Resp,
    /// 存在同名 serde tag 的 [`Command`] 变体，tag-injection 解析可路由。
    pub command_backed: bool,
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
    }
}

/// 行尾 `command_backed` 的命名形式：存在同名 serde tag 的 Command 变体。
const CMD: bool = true;
/// 非 Command 特例方法（host.describe / load_debug_scene / arm_tool_freeze / persist_trust）。
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
pub const METHOD_LOAD_DEBUG_SCENE: &str = "load_debug_scene";
pub const METHOD_ARM_TOOL_FREEZE: &str = "arm_tool_freeze";
pub const METHOD_PERSIST_TRUST: &str = "persist_trust";
pub const METHOD_STEER: &str = "steer";
pub const METHOD_FOLLOW_UP: &str = "follow_up";
pub const METHOD_CLEAR_QUEUE: &str = "clear_queue";
pub const METHOD_SUBSCRIBE: &str = "subscribe";
pub const METHOD_HOST_DESCRIBE: &str = "host.describe";

/// 顺序与 `UNARY_METHODS` 保持一致（守卫测试锁定）。
pub const REGISTRY: &[MethodEntry] = &[
    m(METHOD_PROMPT, Auth::Writer, Idem::PerRpc, Resp::Stream, CMD),
    m(METHOD_ABORT, Auth::Writer, Idem::PerRpc, Resp::Result, CMD),
    m(
        METHOD_GET_STATE,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_SET_MODEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        CMD,
    ),
    m(
        METHOD_CYCLE_MODEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_GET_AVAILABLE_MODELS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_SET_THINKING_LEVEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(METHOD_BASH, Auth::Writer, Idem::PerRpc, Resp::Result, CMD),
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
        METHOD_GET_COMMANDS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
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
    m(
        METHOD_APPEND_ENTRY_LABEL,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
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
    m(
        METHOD_SET_SESSION_NAME,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_SET_SESSION_NAME_FOR,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_DELETE_SESSION,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    // pre-admit 短路：见 handle_unary 的三个早期分支（Idem::Bypass）。
    m(
        METHOD_RELOAD,
        Auth::Readonly,
        Idem::Bypass,
        Resp::Result,
        CMD,
    ),
    m(
        METHOD_LOADED_RESOURCES,
        Auth::Readonly,
        Idem::Bypass,
        Resp::Result,
        true,
    ),
    m(
        METHOD_QUEUE_STATS,
        Auth::Readonly,
        Idem::PerRpc,
        Resp::Result,
        true,
    ),
    m(
        METHOD_LOAD_DEBUG_SCENE,
        Auth::Writer,
        Idem::PerRpc,
        Resp::Result,
        RAW,
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
                "load_debug_scene",
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
                // 唯一名实不符点：wire 方法是 queue_stats（serde alias 兜底）。
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
            REGISTRY.len() - 4,
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
    fn parse_accepts_hand_table_wire_aliases() {
        assert!(matches!(
            parse_command("export_html", &json!({"path": "a.html"})),
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
}
