//! Wire protocol v3 — fory 二进制信封(c2834 双轨数据面,spec r1902/r1904–r1910/r1911)。
//!
//! - 真源:[`xy_wire_v3.fbs`](../../../../../llmanspec/changes/c2834-update-v3-fory-flutter-poc/research/05-fbs-前端保留字实测.md)
//!   本目录同名的 fbs(fbs/FlatBuffers 前端,foryc 生成 Rust/Dart);字段名与既有
//!   JSON 线一致(保留字经 fbs 原名保留),动态块按 `_json` 原文过线(spec r1906)。
//! - 生成物:`generated.rs` check-in(与真源同步再生成;纪律见 fbs 头注释)。
//! - 生成器:fory compiler fbs 前端(dev 963cb37);运行时依赖 crates.io `fory` 1.7.5
//!   (已验证跨版本兼容生成物属性)。
//! - 手写补充见下方 impl：foryc 对递归类型(`SessionTreeNode`)保守省略
//!   `Clone`/`PartialEq`，链式拖累 `TreeResult`/`ResponsePayload`/`Frame`。
//! - 应答 union 当前产出面：`host.describe` → `DescribeResult`，其余一律 `RawOk`
//!   (JSON 原文)；`SubscribeResult` 仅客户端侧兼容读取。`MessagesResult` /
//!   `TreeResult` / `TravelResult` 是 schema-ahead（已声明未接入，任务 2.5b），
//!   接进产品面后大载荷才能拿到完整体积收益。
//!
//! 本模块只承载词表与信封形状,不承载传输;上行/下行路由接线在
//! `app::server` / `app::core::host_client`(双轨期与 JSON-RPC 并存,spec r1902)。

pub mod generated;
pub mod mapping;
pub use generated::*;

// ── foryc 递归类型 trait 补充(生成器保守降级,见模块 doc)────────────

impl Clone for SessionTreeNode {
    fn clone(&self) -> Self {
        Self {
            entry: self.entry.clone(),
            children: self.children.clone(),
            label: self.label.clone(),
        }
    }
}

impl PartialEq for SessionTreeNode {
    fn eq(&self, other: &Self) -> bool {
        self.entry == other.entry && self.children == other.children && self.label == other.label
    }
}

impl Eq for SessionTreeNode {}

impl Clone for TreeResult {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
        }
    }
}

impl PartialEq for TreeResult {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes
    }
}

impl Eq for TreeResult {}

impl PartialEq for ResponsePayload {
    fn eq(&self, other: &Self) -> bool {
        self.debug_repr() == other.debug_repr()
    }
}

impl Eq for ResponsePayload {}

impl PartialEq for Frame {
    fn eq(&self, other: &Self) -> bool {
        self.debug_repr() == other.debug_repr()
    }
}

impl Eq for Frame {}

/// Debug 格式稳定化辅助(供 PartialEq 与测试断言;生成物自带 `Debug`)。
trait DebugRepr {
    fn debug_repr(&self) -> String;
}

impl DebugRepr for ResponsePayload {
    fn debug_repr(&self) -> String {
        format!("{self:?}")
    }
}

impl Frame {
    fn kind_name(&self) -> &'static str {
        match self {
            Frame::ClientRequest(_) => "ClientRequest",
            Frame::ServerResponse(_) => "ServerResponse",
            Frame::ServerNotification(_) => "ServerNotification",
            Frame::Unknown(_) => "Unknown",
        }
    }
}

impl DebugRepr for Frame {
    fn debug_repr(&self) -> String {
        // 判别名前缀,避免不同变体 Debug 恰好同串的极端撞车。
        format!("{} {:?}", self.kind_name(), self)
    }
}

/// `Method` 判别值 ↔ 产品方法名(spec r1904;与 `wire::registry` SSOT 对齐,
/// 由 `tests::method_table_aligns_with_registry` 单测锁定)。
pub const METHOD_NAMES: &[(Method, &str)] = &[
    (
        Method::HostDescribe,
        crate::protocol::wire::registry::METHOD_HOST_DESCRIBE,
    ),
    (
        Method::Prompt,
        crate::protocol::wire::registry::METHOD_PROMPT,
    ),
    (Method::Abort, crate::protocol::wire::registry::METHOD_ABORT),
    (
        Method::GetState,
        crate::protocol::wire::registry::METHOD_GET_STATE,
    ),
    (
        Method::SetModel,
        crate::protocol::wire::registry::METHOD_SET_MODEL,
    ),
    (
        Method::CycleModel,
        crate::protocol::wire::registry::METHOD_CYCLE_MODEL,
    ),
    (
        Method::GetAvailableModels,
        crate::protocol::wire::registry::METHOD_GET_AVAILABLE_MODELS,
    ),
    (
        Method::SetThinkingLevel,
        crate::protocol::wire::registry::METHOD_SET_THINKING_LEVEL,
    ),
    (Method::Bash, crate::protocol::wire::registry::METHOD_BASH),
    (
        Method::Compact,
        crate::protocol::wire::registry::METHOD_COMPACT,
    ),
    (
        Method::GetSessionStats,
        crate::protocol::wire::registry::METHOD_GET_SESSION_STATS,
    ),
    (
        Method::ExportHtml,
        crate::protocol::wire::registry::METHOD_EXPORT_HTML,
    ),
    (
        Method::ExportJsonl,
        crate::protocol::wire::registry::METHOD_EXPORT_JSONL,
    ),
    (
        Method::ImportJsonl,
        crate::protocol::wire::registry::METHOD_IMPORT_JSONL,
    ),
    (
        Method::SwitchSession,
        crate::protocol::wire::registry::METHOD_SWITCH_SESSION,
    ),
    (Method::Fork, crate::protocol::wire::registry::METHOD_FORK),
    (
        Method::GetMessages,
        crate::protocol::wire::registry::METHOD_GET_MESSAGES,
    ),
    (
        Method::EstimateContext,
        crate::protocol::wire::registry::METHOD_ESTIMATE_CONTEXT,
    ),
    (
        Method::GetCommands,
        crate::protocol::wire::registry::METHOD_GET_COMMANDS,
    ),
    (
        Method::SessionTree,
        crate::protocol::wire::registry::METHOD_SESSION_TREE,
    ),
    (
        Method::TravelSessionTree,
        crate::protocol::wire::registry::METHOD_TRAVEL_SESSION_TREE,
    ),
    (
        Method::AppendEntryLabel,
        crate::protocol::wire::registry::METHOD_APPEND_ENTRY_LABEL,
    ),
    (
        Method::ListSessions,
        crate::protocol::wire::registry::METHOD_LIST_SESSIONS,
    ),
    (
        Method::LoadSessionEntries,
        crate::protocol::wire::registry::METHOD_LOAD_SESSION_ENTRIES,
    ),
    (
        Method::NewSession,
        crate::protocol::wire::registry::METHOD_NEW_SESSION,
    ),
    (
        Method::GetSessionName,
        crate::protocol::wire::registry::METHOD_GET_SESSION_NAME,
    ),
    (
        Method::SetSessionName,
        crate::protocol::wire::registry::METHOD_SET_SESSION_NAME,
    ),
    (
        Method::SetSessionNameFor,
        crate::protocol::wire::registry::METHOD_SET_SESSION_NAME_FOR,
    ),
    (
        Method::DeleteSession,
        crate::protocol::wire::registry::METHOD_DELETE_SESSION,
    ),
    (
        Method::Reload,
        crate::protocol::wire::registry::METHOD_RELOAD,
    ),
    (
        Method::LoadedResources,
        crate::protocol::wire::registry::METHOD_LOADED_RESOURCES,
    ),
    (
        Method::GetQueueStats,
        crate::protocol::wire::registry::METHOD_QUEUE_STATS,
    ),
    (Method::Steer, crate::protocol::wire::registry::METHOD_STEER),
    (
        Method::FollowUp,
        crate::protocol::wire::registry::METHOD_FOLLOW_UP,
    ),
    (
        Method::ClearQueue,
        crate::protocol::wire::registry::METHOD_CLEAR_QUEUE,
    ),
    (
        Method::Subscribe,
        crate::protocol::wire::registry::METHOD_SUBSCRIBE,
    ),
    (Method::ApproveTool, "approve_tool"),
    (Method::AnswerQuestion, "answer_question"),
    (Method::Quit, "quit"),
    (
        Method::ArmToolFreeze,
        crate::protocol::wire::registry::METHOD_ARM_TOOL_FREEZE,
    ),
    (
        Method::PersistTrust,
        crate::protocol::wire::registry::METHOD_PERSIST_TRUST,
    ),
];

/// `Method` 的产品方法名(找不到时 panic——表由对齐测试守卫,不会发生)。
pub fn method_name(method: &Method) -> &'static str {
    METHOD_NAMES
        .iter()
        .find(|(m, _)| m == method)
        .map(|(_, name)| *name)
        .unwrap_or_else(|| panic!("unmapped v3 Method discriminant {method:?}"))
}

/// 帧 dump 调试工具(task 5.4):二进制帧 → 可读文本(生成物 `Debug`)。
///
/// xlang compatible 模式自描述,任何合法帧都可离线解析;补偿二进制化
/// 损失的 curl 调试面。调试入口(rust 调用 / 未来 CLI 子命令):
///
/// ```ignore
/// let text = xylitol::protocol::wire::v3::dump_frame(&bytes)?;
/// ```
pub fn dump_frame(bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let frame = Frame::from_bytes(bytes)?;
    Ok(format!("{frame:#?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg_entry(id: &str, parent: Option<&str>, ts: u64, json: &str) -> SessionEntry {
        SessionEntry::MessageEntry(MessageEntry {
            base: Some(EntryBase {
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: ts,
            }),
            message_json: json.into(),
        })
    }

    /// task 5.4:dump 工具输出可读且失败有界(非法字节报错不 panic)。
    #[test]
    fn dump_frame_readable_and_bounded() {
        let f = Frame::ClientRequest(ClientRequest {
            rpc_id: 1,
            request: Request::Describe(Describe {}),
            writer_token: None,
        });
        let text = dump_frame(&f.to_bytes().unwrap()).expect("dump");
        assert!(text.contains("ClientRequest"), "{text}");
        assert!(text.contains("rpc_id: 1"), "{text}");
        assert!(dump_frame(&[0x01, 0x00]).is_err(), "非法字节稳定报错");
    }

    /// spec r1904:v3 方法 ID 表与 registry SSOT 双向对齐。
    /// registry 全部方法必须可寻址;Method 表除三个非 registry 命令
    /// (approve_tool / answer_question / quit)外不得混入未登记方法。
    #[test]
    fn method_table_aligns_with_registry() {
        let v3_names: std::collections::BTreeSet<&str> =
            METHOD_NAMES.iter().map(|(_, n)| *n).collect();
        let registry_names: std::collections::BTreeSet<&str> =
            crate::protocol::wire::registry::names().collect();
        let non_registry = ["approve_tool", "answer_question", "quit"];

        let missing: Vec<_> = registry_names.difference(&v3_names).collect();
        assert!(
            missing.is_empty(),
            "registry 方法 {missing:?} 缺少 v3 Method 判别值"
        );
        let extra: Vec<_> = v3_names
            .difference(&registry_names)
            .filter(|n| !non_registry.contains(n))
            .collect();
        assert!(extra.is_empty(), "v3 Method 表混入未登记方法: {extra:?}");
        assert_eq!(METHOD_NAMES.len(), 41, "方法判别值集合大小漂移");
    }

    /// spec r1910/codegen 纪律:字节级 conformance(锁 fory 1.7.5 codec 与
    /// 真源形状)。此断言失败 = codec 或 schema 变更,须有意识地重录并评审
    /// (fory 升级 / fbs 字段增删),不允许顺手改常量蒙混。
    #[test]
    fn codec_conformance_bytes_locked() {
        let toolstart = Frame::ServerNotification(ServerNotification {
            seq: 43,
            notification: Notification::Event(Event::ToolStart(ToolStart {
                id: "t1".into(),
                name: "grep".into(),
                args_json: r#"{"pattern":"TODO","max":100}"#.into(),
            })),
        });
        let steer = Frame::ClientRequest(ClientRequest {
            rpc_id: 7,
            request: Request::Command(Command::Steer(Steer {
                message: "hi".into(),
            })),
            writer_token: None,
        });
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(
            hex(&toolstart.to_bytes().unwrap()),
            "01ff22bbbea2d00603001c000a80951e81276433c2ec8dc19905c40ec8212b01ff22d49e99b60304001c020cf040624b9be125c389affcb603c415c815cc150a74311267726570727b227061747465726e223a22544f444f222c226d6178223a3130307d"
        );
        assert_eq!(
            hex(&steer.to_bytes().unwrap()),
            "01ff22bbbea2d00601001c000cd02af27ee5bd45c3edd6aabd07c40ec821ce150701ff2283d98bc70420001c0207404cb24a470d4dc1fbabbc16c4150a6869fd"
        );
    }

    /// 体积基准(c2834 tasks 1.5):大 transcript 快照 v3 vs 今日 JSON 形态。
    /// 断言宽松(0.95)防脆弱;比例变化显著时人工复核体积收益叙事。
    ///
    /// 两个比例分属不同通路，引用时别串位：
    /// - 0.73（本测试）= 强 schema union（`ResponsePayload::MessagesResult`），
    ///   即 2.5b 接完后的形态；
    /// - ≈ 1.0006 = 当前产品通路（非 describe 应答一律 `RawOk`，JSON 原文入
    ///   string，100KB 载荷仅 +64B 信封开销）。
    #[test]
    fn size_baseline_vs_json_large_transcript() {
        let entries: Vec<SessionEntry> = (0..500)
            .map(|i| {
                SessionEntry::MessageEntry(MessageEntry {
                    base: Some(EntryBase {
                        id: format!("entry-{i:04}"),
                        parent_id: (i > 0).then(|| format!("entry-{:04}", i - 1)),
                        timestamp: 1_700_000_000 + i as u64,
                    }),
                    message_json: format!(
                        r#"{{"type":"llm","role":"assistant","content":[{{"type":"text","text":"reply chunk {i} with some realistic length body text"}}]}}"#
                    ),
                })
            })
            .collect();
        let frame = Frame::ServerResponse(ServerResponse {
            rpc_id: 1,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::MessagesResult(MessagesResult { entries })),
            writer_token: None,
        });
        let v3 = frame.to_bytes().unwrap().len();

        let json_equiv = {
            let items: Vec<serde_json::Value> = (0..500)
                .map(|i| {
                    serde_json::json!({
                        "type": "message",
                        "id": format!("entry-{i:04}"),
                        "parentId": if i > 0 { serde_json::json!(format!("entry-{:04}", i-1)) } else { serde_json::Value::Null },
                        "timestamp": 1_700_000_000 + i,
                        "message": {
                            "type": "llm", "role": "assistant",
                            "content": [{"type": "text", "text": format!("reply chunk {i} with some realistic length body text")}]
                        }
                    })
                })
                .collect();
            serde_json::to_string(&serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": { "entries": items }
            }))
            .unwrap()
            .len()
        };

        let ratio = v3 as f64 / json_equiv as f64;
        println!("large transcript(500 entries): v3={v3}B json-rpc={json_equiv}B ratio={ratio:.2}");
        assert!(ratio < 0.95, "v3 体积收益消失: ratio={ratio:.2}");
    }

    fn roundtrip(frame: &Frame) -> Frame {
        let bytes = frame.to_bytes().expect("encode");
        Frame::from_bytes(&bytes).expect("decode")
    }

    /// spec r1902:v3 帧编解码往返(上行 Command union + 幂等键)。
    #[test]
    fn client_request_roundtrip() {
        let f = Frame::ClientRequest(ClientRequest {
            rpc_id: 7,
            request: Request::Command(Command::Steer(Steer {
                message: "hi".into(),
            })),
            writer_token: None,
        });
        assert_eq!(roundtrip(&f), f);
    }

    /// spec r1905:下行通知(seq + Event + nullable 载荷)。
    #[test]
    fn notification_with_compaction_end_roundtrip() {
        let f = Frame::ServerNotification(ServerNotification {
            seq: 42,
            notification: Notification::Event(Event::CompactionEnd(CompactionEnd {
                result: None,
                aborted: false,
                reason: "manual".into(),
                will_retry: false,
                error_message: None,
                summary: Some("sum".into()),
                tokens_before: Some(1000),
                tokens_after: None,
                notice: None,
            })),
        });
        assert_eq!(roundtrip(&f), f);
    }

    /// get_messages 大载荷(嵌套 SessionEntry 全变体抽样)。
    #[test]
    fn messages_result_roundtrip() {
        let f = Frame::ServerResponse(ServerResponse {
            rpc_id: 9,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::MessagesResult(MessagesResult {
                entries: vec![
                    SessionEntry::SessionHeader(SessionHeader {
                        version: 7,
                        id: "s0".into(),
                        timestamp: 1,
                        cwd: "/tmp".into(),
                        parent_session: None,
                        fork_at_entry_id: None,
                    }),
                    msg_entry("e1", None, 2, r#"{"role":"user"}"#),
                ],
            })),
            writer_token: None,
        });
        assert_eq!(roundtrip(&f), f);
    }

    /// session_tree 递归节点(生成器递归类型 trait 补充的守护测试)。
    #[test]
    fn recursive_tree_roundtrip() {
        let f = Frame::ServerResponse(ServerResponse {
            rpc_id: 10,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::TreeResult(TreeResult {
                nodes: vec![SessionTreeNode {
                    entry: msg_entry("n1", Some("e1"), 3, "x"),
                    children: vec![SessionTreeNode {
                        entry: SessionEntry::LabelEntry(LabelEntry {
                            base: Some(EntryBase {
                                id: "l1".into(),
                                parent_id: None,
                                timestamp: 4,
                            }),
                            target_id: "n1".into(),
                            label: Some("L".into()),
                        }),
                        children: vec![],
                        label: None,
                    }],
                    label: None,
                }],
            })),
            writer_token: None,
        });
        let back = roundtrip(&f);
        assert_eq!(back, f, "recursive tree with manual trait impls");
    }

    /// 错误应答(payload 缺省,spec r1904 产品码语义载体)。
    #[test]
    fn error_response_roundtrip() {
        let f = Frame::ServerResponse(ServerResponse {
            rpc_id: 11,
            ok: false,
            error: Some(RpcError {
                code: "writer_conflict".into(),
                details: "d".into(),
            }),
            payload: None,
            writer_token: None,
        });
        assert_eq!(roundtrip(&f), f);
    }

    /// spec r1906:动态块(工具参数)JSON 原文逐字节保真。
    #[test]
    fn tool_start_args_json_survives_wire() {
        let raw = r#"{"pattern":"TODO","max":100}"#;
        let f = Frame::ServerNotification(ServerNotification {
            seq: 43,
            notification: Notification::Event(Event::ToolStart(ToolStart {
                id: "t1".into(),
                name: "grep".into(),
                args_json: raw.into(),
            })),
        });
        let back = roundtrip(&f);
        let Frame::ServerNotification(n) = back else {
            unreachable!()
        };
        match &n.notification {
            Notification::Event(Event::ToolStart(ts)) => {
                assert_eq!(ts.args_json, raw, "args JSON 原文逐字节保真")
            }
            _ => unreachable!(),
        }
    }
}
