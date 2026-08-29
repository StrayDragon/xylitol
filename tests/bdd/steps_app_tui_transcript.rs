//! Steps for `app-tui-transcript` — 无头帧与 rebuild seam 族。
//!
//! P1：纯字形合约（att19）。P2：场景构建器帧（att10/24/33/34）。
//! P4：live scrollback（att1）、Thought 结算（att8）、travel 重建 seam
//! （att12/18，走产品 `rebuild_scrollback_from_travel` 正常路径）。

use crate::prelude::*;
use crate::steps_app_tui_interaction::TuiInteraction;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol::app::tui::GlyphSet;
use xylitol::app::tui::SceneBuilder;
use xylitol::app::tui::UiEntry;
use xylitol::app::tui::UiModel;
use xylitol::app::tui::apply_xy_event;
use xylitol::app::tui::rebuild_scrollback_from_travel;
use xylitol::app::tui::travel_history_note;
use xylitol::protocol::session::SessionTreeKind;
use xylitol::protocol::session::SessionTreeTravel;

/// Shared state for transcript glyph scenarios.
pub struct TranscriptBdd {
    pub glyph_pair: RefCell<Option<(&'static str, &'static str)>>,
    pub previews: RefCell<Vec<String>>,
    pub plain_frame: RefCell<Option<String>>,
    /// P2 批量：场景内累计的纯文本帧（顺序即渲染序）。
    pub frames: RefCell<Vec<String>>,
    /// P4：最近一次产品 paint 的 ANSI 帧（颜色/样式断言）。
    pub ansi_frame: RefCell<Option<String>>,
    /// P4：rebuild 场景的模型快照（重建 / 直播 / 追加通告等顺序留痕）。
    pub models: RefCell<Vec<UiModel>>,
}

#[fixture]
pub fn transcript_bdd() -> TranscriptBdd {
    TranscriptBdd {
        glyph_pair: RefCell::new(None),
        previews: RefCell::new(Vec::new()),
        plain_frame: RefCell::new(None),
        frames: RefCell::new(Vec::new()),
        ansi_frame: RefCell::new(None),
        models: RefCell::new(Vec::new()),
    }
}

/// P2 管线公共入口：脚本事件 → 真实 UiRoot 渲染 → 剥离 ANSI 纯文本帧。
fn render_plain(script: impl FnOnce(&mut SceneBuilder)) -> String {
    let mut sb = SceneBuilder::begin();
    script(&mut sb);
    let (plain, _) = sb.render(80);
    plain
}

fn read_glyph_pair() -> (&'static str, &'static str) {
    let gs = GlyphSet::from_env();
    (gs.fold(), gs.unfold())
}

#[given("折叠字形环境未指定（默认 Unicode 集）")]
fn given_glyph_env_unset() {
    // SAFETY: 本变量仅本步骤文件读写；场景内串行翻转，无并发窗口。
    unsafe { std::env::remove_var("XYLITOL_TUI_GLYPH_SET") };
}

#[when("读取折叠与展开字形")]
fn when_read_glyphs(transcript_bdd: &TranscriptBdd) {
    transcript_bdd.glyph_pair.replace(Some(read_glyph_pair()));
}

#[then("折叠为 ▸ 展开为 ▾ 且各占单列")]
fn then_unicode_glyphs(transcript_bdd: &TranscriptBdd) {
    let pair = transcript_bdd.glyph_pair.borrow().expect("glyphs");
    assert_eq!(pair, ("▸", "▾"), "unicode fold/unfold glyphs");
    // att19：可视宽 1 列、无 ANSI 转义
    for g in [pair.0, pair.1] {
        assert_eq!(g.chars().count(), 1, "glyph must occupy 1 column: {g}");
        assert!(!g.contains('\x1b'), "glyph must not carry ANSI: {g}");
    }
}

#[then("切换环境变量 XYLITOL_TUI_GLYPH_SET=ascii 并重新读取")]
fn when_flip_ascii_reread(transcript_bdd: &TranscriptBdd) {
    // SAFETY: 同上；读后立即恢复删除，避免污染同进程其它用例。
    unsafe {
        std::env::set_var("XYLITOL_TUI_GLYPH_SET", "ascii");
    }
    let pair = read_glyph_pair();
    unsafe {
        std::env::remove_var("XYLITOL_TUI_GLYPH_SET");
    }
    transcript_bdd.glyph_pair.replace(Some(pair));
}

#[then("折叠回退为 > 展开回退为 v 且各占单列")]
fn then_ascii_fallback(transcript_bdd: &TranscriptBdd) {
    let pair = transcript_bdd.glyph_pair.borrow().expect("glyphs");
    assert_eq!(pair, (">", "v"), "ascii fold/unfold fallback glyphs");
}

// ---- att13：折叠态工具人话摘要（位置摘要，名字由 header 单独绘制）----

#[when("折叠态读取 bash、read、write 三类参数人话摘要")]
fn then_read_tool_previews(transcript_bdd: &TranscriptBdd) {
    use xylitol::app::tui::human_tool_args_preview;
    let bash = human_tool_args_preview("bash", &serde_json::json!({"command": "cargo test"}), 200);
    let read = human_tool_args_preview(
        "read",
        &serde_json::json!({"path": "src/lib.rs", "offset": 10, "limit": 5}),
        200,
    );
    let write = human_tool_args_preview("write", &serde_json::json!({"path": "docs/x.md"}), 200);
    *transcript_bdd.previews.borrow_mut() = vec![bash, read, write];
}

#[then("bash 前缀 $ 且 read 附行号区间且 write 为纯路径不带名前缀")]
fn then_tool_preview_shapes(transcript_bdd: &TranscriptBdd) {
    let previews = transcript_bdd.previews.borrow();
    assert_eq!(previews[0], "$ cargo test", "bash → $ {{command}}");
    assert_eq!(
        previews[1], "src/lib.rs:10-14",
        "read offset/limit → :start-end"
    );
    assert_eq!(previews[2], "docs/x.md", "write → 纯路径");
    for p in previews.iter() {
        assert!(
            !p.starts_with("bash ") && !p.starts_with("read ") && !p.starts_with("write "),
            "summary must be location-only (name painted separately): {p}"
        );
        assert!(!p.contains('{'), "must not leak raw args JSON: {p}");
    }
}

#[then("缺 path 时用三点占位且不回退完整 args JSON")]
fn then_missing_path_placeholder(transcript_bdd: &TranscriptBdd) {
    let _ = transcript_bdd; // 断言本地自足，夹具仅用于步骤分组
    use xylitol::app::tui::human_tool_args_preview;
    // 前一步存的是三类正常摘要；这里直接补算 edit 无 path 场景
    let edit_no_path = human_tool_args_preview(
        "edit",
        &serde_json::json!({"edits": [{"oldText": "a", "newText": "b"}]}),
        200,
    );
    assert_eq!(edit_no_path, "...", "missing path must fall back to ...");
    assert!(
        !edit_no_path.contains("oldText"),
        "must not embed full args JSON: {edit_no_path}"
    );
}

// ---- att10：相邻块空行分隔（P2 无头帧挂载面首证）----

#[when("以场景构建器渲染相邻的助手块与工具块（宽 80）")]
fn then_render_adjacent_blocks(transcript_bdd: &TranscriptBdd) {
    use xylitol::app::tui::SceneBuilder;
    let mut sb = SceneBuilder::begin();
    sb.assistant("alpha body").message_end();
    sb.tool_start("t1", "read", "src/lib.rs")
        .tool_end("t1", "read");
    let (plain, _dump) = sb.render(80);
    *transcript_bdd.plain_frame.borrow_mut() = Some(plain);
}

#[then("相邻块之间至少一行空行分隔且不粘连成墙")]
fn then_block_gap_present(transcript_bdd: &TranscriptBdd) {
    let plain = transcript_bdd.plain_frame.borrow().clone().expect("frame");
    let lines: Vec<&str> = plain.lines().collect();
    let find_last = |needle: &str| {
        lines
            .iter()
            .rposition(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("frame must contain {needle:?}:\n{plain}"))
    };
    let alpha = find_last("alpha body");
    let tool = find_last("lib.rs");
    assert!(
        tool > alpha,
        "tool block must render below assistant block:\n{plain}"
    );
    let gap_blank = lines[alpha + 1..tool].iter().any(|l| l.trim().is_empty());
    assert!(
        gap_blank,
        "att10: adjacent blocks need >=1 blank line between:\n{plain}"
    );
}

// ---- att24 / att34 / att33：簇语义帧断言（P2 管线批量，真值对齐 live_tape）----

#[when("以场景构建器回放读后改写序列（read old.rs 然后 edit a.rs）")]
fn when_replay_read_then_edit(transcript_bdd: &TranscriptBdd) {
    let plain = render_plain(|sb| {
        sb.tool_start("r1", "read", "old.rs")
            .tool_end("r1", "read")
            .assistant("mid-body")
            .tool_start("e1", "edit", "a.rs")
            .tool_end("e1", "edit");
    });
    *transcript_bdd.frames.borrow_mut() = vec![plain];
}

#[then("只读前簇封口为 Explored old.rs 且改写簇头保持 Editing a.rs")]
fn then_explored_and_editing_heads(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    let lines: Vec<&str> = plain.lines().collect();
    let pos = |needle: &str| {
        lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("frame must contain {needle:?}:\n{plain}"))
    };
    let sealed = pos("Explored old.rs");
    let body = pos("mid-body");
    let editing = pos("Editing a.rs");
    assert!(
        sealed < body && body < editing,
        "att24: sealed head, body divider, editing head must co-exist in order:\n{plain}"
    );
}

#[then("改写结束后无 Edited 错时态")]
fn then_no_wrong_tense_after_end(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    // 开启中的回合内改写簇头保持现在时 Editing；MUST NOT 翻成 Edited 或与 Explored 并列同头
    assert!(
        plain.contains("Editing a.rs") && !plain.contains("Edited a.rs"),
        "post-end cluster head must stay Editing within live turn:\n{plain}"
    );
    assert!(
        !(plain.contains("Edited") && plain.contains("Explored old.rs") && {
            let edited_line = plain.lines().find(|l| l.contains("Edited")).unwrap();
            edited_line.contains("Explored")
        }),
        "att24 exclusivity: Edited and Explored must not share one head:\n{plain}"
    );
}

#[then("全帧不出现 Worked for 与 Planning next moves")]
fn then_no_envelope_no_planning(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    for (i, plain) in frames.iter().enumerate() {
        assert!(
            !plain.contains("Worked for"),
            "frame {i}: live window must not seal envelope\n{plain}"
        );
        assert!(
            !plain.contains("Planning next moves"),
            "frame {i}: no placeholder row allowed\n{plain}"
        );
    }
}

#[when("以场景构建器在两个工具活动之间插入助手正文")]
fn when_insert_body_between_tools(transcript_bdd: &TranscriptBdd) {
    let plain = render_plain(|sb| {
        sb.tool_start("r1", "read", "old.rs")
            .tool_end("r1", "read")
            .assistant("mid-body")
            .tool_start("e1", "edit", "a.rs");
    });
    *transcript_bdd.frames.borrow_mut() = vec![plain];
}

#[then("正文封口前簇且新簇在其下方独立开口")]
fn then_body_seals_previous_cluster(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    let lines: Vec<&str> = plain.lines().collect();
    let pos = |needle: &str| {
        lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("frame must contain {needle:?}:\n{plain}"))
    };
    let sealed = pos("Explored old.rs");
    let body = pos("mid-body");
    let next = pos("Editing a.rs");
    assert!(
        sealed < body && body < next,
        "att34: body must seal previous cluster and precede new cluster head:\n{plain}"
    );
}

#[when("以场景构建器渲染流式思考中的 live window")]
fn when_render_streaming_thinking(transcript_bdd: &TranscriptBdd) {
    let plain = render_plain(|sb| {
        sb.thinking("consider next edit");
    });
    transcript_bdd.frames.borrow_mut().push(plain);
}

#[then("出现 Thinking 簇头且无 Thought 与 Ctrl+T 旁注")]
fn then_thinking_stream_head(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    assert!(
        plain.contains("Thinking"),
        "streaming thinking must show Thinking cluster head:\n{plain}"
    );
    assert!(
        !plain.contains("Thought"),
        "unsealed thinking must not show Thought:\n{plain}"
    );
    assert!(
        !plain.contains("Ctrl+T"),
        "inflight thinking carries no fold chord hint:\n{plain}"
    );
}

#[when("以场景构建器渲染含助手正文的 live window")]
fn when_render_assistant_live(transcript_bdd: &TranscriptBdd) {
    let plain = render_plain(|sb| {
        sb.assistant("alpha live body").message_end();
    });
    transcript_bdd.frames.borrow_mut().push(plain);
}

#[then("助手正文可见且仍无信封封套")]
fn then_assistant_live_unenveloped(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    assert!(
        plain.contains("alpha live body"),
        "live assistant body must be visible:\n{plain}"
    );
    assert!(
        !plain.contains("Worked for") && !plain.contains("Planning next moves"),
        "att33: live window stays open, no envelope/placeholder:\n{plain}"
    );
}

// ---- att1：live scrollback 多段正文不截断、包 Markdown 样式呈现 ----

#[when("以场景构建器渲染多段助手正文（宽 80）")]
fn when_render_multi_assistant(transcript_bdd: &TranscriptBdd) {
    use xylitol::app::tui::InteractionBdd;
    let mut sb = SceneBuilder::begin();
    sb.assistant("first **loud** tail").message_end();
    sb.assistant("second para").message_end();
    sb.assistant("third para").message_end();
    let mut fx = InteractionBdd::from_model(sb.into_model());
    *transcript_bdd.ansi_frame.borrow_mut() = Some(fx.render_lines(80).join("\n"));
    *transcript_bdd.frames.borrow_mut() = vec![fx.render_plain(80)];
}

#[then("各段正文均在帧内且早段未被挤出")]
fn then_all_paragraphs_present(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    for seg in ["first", "second para", "third para"] {
        assert!(
            plain.contains(seg),
            "att1: `{seg}` must stay in scrollback:\n{plain}"
        );
    }
    let first = plain.find("first").expect("first seg");
    let third = plain.find("third para").expect("third seg");
    assert!(
        first < third,
        "att1: early content must not be truncated away:\n{plain}"
    );
}

#[then("助手正文行携带样式转义")]
fn then_assistant_line_styled(transcript_bdd: &TranscriptBdd) {
    let ansi = transcript_bdd
        .ansi_frame
        .borrow()
        .clone()
        .expect("ansi frame");
    assert!(
        ansi.lines()
            .filter(|l| l.contains("loud"))
            .any(|l| l.contains('\x1b')),
        "att1: package markdown render must style the assistant line:\n{ansi:?}"
    );
}

// ---- att8：思考结算后 Thought {Ns} 外显 + 完整和弦旁注 ----

#[when("以场景构建器回放思考加工具并结算 7 秒封轮挂载交互面")]
fn when_mount_settled_thinking(transcript_bdd: &TranscriptBdd, tui_interaction: &TuiInteraction) {
    use xylitol::app::tui::InteractionBdd;
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    sb.thinking_flushed("慢慢想", 7);
    sb.tool_start("t-grep", "grep", "src");
    sb.tool_end("t-grep", "grep");
    sb.message_end();
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::AgentEnd {
        messages: Vec::new(),
    });
    fx.open_hit_viewport(200);
    *tui_interaction.fx.borrow_mut() = Some(fx);
    let _ = transcript_bdd; // 帧断言走交互夹具渲染，占位保持步骤同组
}

#[then("结算行外显 Thought 7s 且不再出现流式 Thinking 头")]
fn then_thought_label_and_duration(tui_interaction: &TuiInteraction) {
    let plain = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(80)
    };
    assert!(
        plain.contains("Thought 7s"),
        "settled thinking must show Thought with persisted duration:\n{plain}"
    );
    assert!(
        !plain.contains("Thinking"),
        "streaming label must be replaced after the thinking channel ends:\n{plain}"
    );
}

#[then("思考块旁注为括号完整和弦 Ctrl+T")]
fn then_thinking_block_chord_hint(tui_interaction: &TuiInteraction) {
    let plain = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(80)
    };
    assert!(
        plain.contains("(Ctrl+T)"),
        "att8: thinking fold hint must be a full parenthesised chord:\n{plain}"
    );
}

// ---- att12 / att18：travel 重建 seam（产品正常路径） ----

/// 一轮 live 条目：user → assistant(thinking + text + toolCall) → toolResult。
fn rebuild_fixture_entries() -> (Vec<SessionEntry>, SessionTreeTravel) {
    let entries = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::json!({
                "role": "user",
                "content": [{ "type": "text", "text": "hi" }],
                "timestamp": 1_700_000_000_000u64,
            }),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: Some("u1".into()),
                timestamp: 0,
            },
            message: serde_json::json!({
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "step 1" },
                    { "type": "text", "text": "hello" },
                    { "type": "toolCall", "id": "tc1", "name": "read",
                      "arguments": { "path": "old.rs" } }
                ],
                "timestamp": 1_700_000_005_000u64,
            }),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "tr1".into(),
                parent_id: Some("a1".into()),
                timestamp: 0,
            },
            message: serde_json::json!({
                "role": "toolResult",
                "toolCallId": "tc1",
                "toolName": "read",
                "content": [{ "type": "text", "text": "contents" }],
                "isError": false,
                "timestamp": 1_700_000_006_000u64,
            }),
        }),
    ];
    let travel = SessionTreeTravel {
        kind: SessionTreeKind::MessageHistory,
        selected_id: "tr1".into(),
        leaf_id: Some("tr1".into()),
        editor_text: None,
    };
    (entries, travel)
}

#[when("以含思考正文与同 id 工具调用加结果的条目重建 transcript")]
fn when_rebuild_from_parts(transcript_bdd: &TranscriptBdd) {
    let (entries, travel) = rebuild_fixture_entries();
    let mut rebuilt = UiModel::default();
    rebuild_scrollback_from_travel(&mut rebuilt, &entries, &travel);

    // 同一轮的直播事件流（相同 thinking / 正文 / 工具调用与结果）。
    let mut live = UiModel::default();
    apply_xy_event(&mut live, &XyEvent::ThinkingDelta("step 1".into()));
    apply_xy_event(
        &mut live,
        &XyEvent::MessageUpdate {
            text: "hello".into(),
            thinking: None,
            message: None,
        },
    );
    apply_xy_event(
        &mut live,
        &XyEvent::ToolExecutionStart {
            id: "tc1".into(),
            name: "read".into(),
            args: serde_json::json!({ "path": "old.rs" }),
        },
    );
    apply_xy_event(
        &mut live,
        &XyEvent::ToolExecutionEnd {
            id: "tc1".into(),
            name: "read".into(),
            result: "contents".into(),
            is_error: false,
        },
    );
    *transcript_bdd.models.borrow_mut() = vec![rebuilt, live];
}

#[then("思考正文工具各成一块且工具恰一行不另起第二工具")]
fn then_rebuild_block_shapes(transcript_bdd: &TranscriptBdd) {
    let models = transcript_bdd.models.borrow();
    let rebuilt = &models[0];
    assert!(
        matches!(
            rebuilt.entries.as_slice(),
            [
                UiEntry::User { .. },
                UiEntry::Thinking { .. },
                UiEntry::Assistant { .. },
                UiEntry::Tool { .. }
            ]
        ),
        "att12: typed parts must project to separate blocks: {:?}",
        rebuilt.entries
    );
    let tools: Vec<_> = rebuilt
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Tool { .. }))
        .collect();
    assert_eq!(
        tools.len(),
        1,
        "att12: call + result must merge into exactly one tool row: {:?}",
        rebuilt.entries
    );
    assert!(
        matches!(
            tools[0],
            UiEntry::Tool {
                id,
                done: true,
                is_error: false,
                ..
            } if id == "tc1"
        ),
        "merged row keeps the toolCallId (never a session-entry id): {:?}",
        tools[0]
    );
}

#[then("重建工具行与同轮直播工具行逐字段一致")]
fn then_rebuild_tool_matches_live(transcript_bdd: &TranscriptBdd) {
    let models = transcript_bdd.models.borrow();
    let tool_of = |m: &UiModel| {
        m.entries.iter().find_map(|e| match e {
            UiEntry::Tool { .. } => Some(e.clone()),
            _ => None,
        })
    };
    let rebuilt_tool = tool_of(&models[0]).expect("rebuilt tool row");
    let live_tool = tool_of(&models[1]).expect("live tool row");
    assert_eq!(
        rebuilt_tool, live_tool,
        "att12: rebuild must be idempotent with the live flush shape"
    );
}

#[when("重建到叶节点并按产品路径追加路径通告")]
fn when_rebuild_and_append_notice(transcript_bdd: &TranscriptBdd) {
    let (entries, travel) = rebuild_fixture_entries();
    let mut model = UiModel::default();
    rebuild_scrollback_from_travel(&mut model, &entries, &travel);
    let before = model.clone();
    let note = travel_history_note(&entries, &travel);
    model.entries.push(UiEntry::ScrollNotice { text: note });
    *transcript_bdd.models.borrow_mut() = vec![before, model];
}

#[then("通告条目位于 entries 末尾且重建内容次序保持原样")]
fn then_notice_trailing_not_prepended(transcript_bdd: &TranscriptBdd) {
    let models = transcript_bdd.models.borrow();
    let before = &models[0];
    let after = &models[1];
    let last = after.entries.last().expect("non-empty entries");
    assert!(
        matches!(last, UiEntry::ScrollNotice { text } if text.contains("history @")),
        "att18: travel note must be the appended last entry: {last:?}"
    );
    assert_eq!(
        &after.entries[..after.entries.len() - 1],
        &before.entries[..],
        "att18: prepend is forbidden — rebuilt content keeps its order untouched"
    );
}
