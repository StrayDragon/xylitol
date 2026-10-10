//! c2848/r1924 BDD：切点度量与统一估算**同源不变量**（防回归独立字符口径）。
//! 步骤为自包含属性断言（无 store/agent 依赖）：when 在同一线程内构造夹具并
//! 计算两个数字，then 断言同源等式与「非独立字符计数」双重锁定。

use std::cell::RefCell;

use rstest_bdd_macros::{given, then, when};
use xylitol::agent::compaction::cut_detector::estimate_tokens_entry_for_cut;
use xylitol::agent::llm_project::project_for_llm;
use xylitol::protocol::message::{AgentMessage, AgentPart};
use xylitol::protocol::session::{EntryBase, MessageEntry, SessionEntry};

type Measure = (u64, u64);
thread_local! {
    static MEASURE: RefCell<Option<Measure>> = const { RefCell::new(None) };
}

fn message_entry(id: &str, msg: AgentMessage) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: None,
            timestamp: 1_700_000_000,
        },
        message: serde_json::to_value(msg).unwrap(),
    })
}

/// 与实现同源的逐条分解：`project_for_llm` 投影后序列化字节 /3 + pi 图片成本。
/// （`when` 用公共 API 重算，任何对切点度量的独立化回归都会破坏 `then` 等式。）
fn unified_decomposition(entries: &[SessionEntry]) -> u64 {
    use xylitol_ai_bridge::accounting::heuristic_token_count;
    const ESTIMATED_IMAGE_CHARS: u64 = 4800;
    let mut total = 0u64;
    for e in entries {
        let Some(msg) = e.as_agent_message() else {
            continue;
        };
        let image_tokens = match &msg {
            AgentMessage::Llm(xylitol::protocol::message::LlmMessage::UserMessage {
                content,
                ..
            })
            | AgentMessage::Llm(xylitol::protocol::message::LlmMessage::ToolResultMessage {
                content,
                ..
            }) => {
                content
                    .iter()
                    .filter(|p| matches!(p, AgentPart::Image(_)))
                    .count() as u64
                    * heuristic_token_count(ESTIMATED_IMAGE_CHARS)
            }
            _ => 0,
        };
        let projected = project_for_llm(std::slice::from_ref(&msg));
        let serialized: u64 = projected
            .iter()
            .map(|m| {
                heuristic_token_count(
                    serde_json::to_string(m)
                        .map(|s| s.len() as u64)
                        .unwrap_or(0),
                )
            })
            .sum();
        total += serialized.saturating_add(image_tokens);
    }
    total
}

#[given("会话含一批可投影的 user 与 assistant 消息（含长文本与图片）")]
fn g_metric_fixture() {}

#[when("对该上下文计算切点度量与统一估算的逐条组成")]
fn w_metric_compute() {
    let entries = vec![
        message_entry(
            "u1",
            AgentMessage::user(
                "你好，这个会话很长的中文文本，用来检验切点度量口径是否与统一估算同源".repeat(3),
            ),
        ),
        message_entry("a1", AgentMessage::assistant("好的，收到。".repeat(10))),
        message_entry(
            "u2",
            AgentMessage::user_parts(vec![
                AgentPart::text("请查看这张图"),
                AgentPart::image("image/png", "AAAA"),
            ]),
        ),
    ];
    let cut_sum: u64 = entries.iter().map(estimate_tokens_entry_for_cut).sum();
    let unified = unified_decomposition(&entries);
    MEASURE.with(|m| *m.borrow_mut() = Some((cut_sum, unified)));
}

#[then("切点累计等于统一估算逐条分解且非常规字符计数")]
fn t_metric_same_source() {
    let (cut_sum, unified) = MEASURE.with(|m| m.borrow().expect("when 先于 then"));
    assert_eq!(
        cut_sum, unified,
        "切点度量 MUST 等于统一估算的逐条分解（同源非独立）: cut={cut_sum} unified={unified}"
    );
    assert!(
        cut_sum != (3 + 4800u64).div_ceil(4),
        "切点度量 MUST NOT 回到独立内容字符口径（图片 1201 特征值）: cut={cut_sum}"
    );
    assert!(
        cut_sum >= 1600,
        "同源度量 MUST 保留 pi 图片视觉成本（≥1600 token）: cut={cut_sum}"
    );
}
