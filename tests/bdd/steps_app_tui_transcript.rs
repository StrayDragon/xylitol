//! Steps for `app-tui-transcript` — 无头帧与 rebuild seam 族。
//!
//! P1：纯字形合约（att19）。P2：场景构建器帧（att10/24/33/34）。
//! P4：live scrollback（att1）、Thought 结算（att8）、travel 重建 seam
//! （att12/18，走产品 `rebuild_scrollback_from_travel` 正常路径）。

use crate::app::tui::GlyphSet;
use crate::app::tui::SceneBuilder;
use crate::app::tui::UiEntry;
use crate::app::tui::UiModel;
use crate::app::tui::apply_xy_event;
use crate::app::tui::rebuild_scrollback_from_travel;
use crate::app::tui::travel_history_note;
use crate::protocol::session::SessionTreeKind;
use crate::protocol::session::SessionTreeTravel;
use crate::tests::bdd::prelude::*;
use crate::tests::bdd::steps_app_tui_interaction::TuiInteraction;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

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
    /// c2510/att35：跨步推进的场景构建器（流式计数更新断言）。
    pub scene: RefCell<Option<SceneBuilder>>,
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
        scene: RefCell::new(None),
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
    use crate::app::tui::human_tool_args_preview;
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
    use crate::app::tui::human_tool_args_preview;
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
    use crate::app::tui::SceneBuilder;
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
    // c2540: the cluster head is count-only now — locate the block row by its
    // head form (the kid stays collapsed behind it in this scene).
    let tool = find_last("Exploring 1 file");
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

#[then("只读前簇封口为 Explored 1 file 且改写簇头保持 Editing 1 file")]
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
    let sealed = pos("Explored 1 file");
    let body = pos("mid-body");
    let editing = pos("Editing 1 file");
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
        plain.contains("Editing 1 file") && !plain.contains("Edited 1 file"),
        "post-end cluster head must stay Editing within live turn:\n{plain}"
    );
    assert!(
        !(plain.contains("Edited") && plain.contains("Explored 1 file") && {
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
    let sealed = pos("Explored 1 file");
    let body = pos("mid-body");
    let next = pos("Editing 1 file");
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
    use crate::app::tui::InteractionBdd;
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
    use crate::app::tui::InteractionBdd;
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

// ---- att4：rail 皮肤状态轨（accent / success / error，轨+gutter 结构） ----

use crate::app::tui::FoldTarget;
use crate::app::tui::InteractionBdd;

/// rail 前缀 = 1 列底色 + `49m` 复位 + 1 列无底色 gutter（paint_left_rail_line 同构）。
pub(crate) fn rail_prefix(rgb: xylitol_tui::terminal_colors::RgbColor) -> String {
    format!("\x1b[48;2;{};{};{}m \x1b[49m ", rgb.r, rgb.g, rgb.b)
}

/// 封轮 + 打开命中视口 + 定点展开簇头（簇头默认折叠，内层块须展开才进帧）。
fn seal_and_open_cluster(fx: &mut InteractionBdd) {
    fx.push_xy(XyEvent::AgentEnd {
        messages: Vec::new(),
    });
    fx.open_hit_viewport(200);
    let _ = fx.render_plain(80); // 注册折叠命中区
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::Cluster(_)))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("a registered cluster header region");
    assert!(fx.left_click(hit.0, hit.1), "cluster click must consume");
}

#[when("以场景构建器回放 pending、成功与失败三种工具并取 ANSI 帧")]
fn when_mount_three_status_tools(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    sb.tool_start("t-pend", "read", "pending.rs");
    sb.tool_start("t-ok", "read", "done.rs")
        .tool_end("t-ok", "read");
    sb.tool_start("t-err", "read", "bad.rs");
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "t-err".into(),
        name: "read".into(),
        result: "boom".into(),
        is_error: true,
    });
    seal_and_open_cluster(&mut fx);
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[then("pending 轨用 accent 而成功轨用 success 且失败轨用 error")]
fn then_rail_status_colors(tui_interaction: &TuiInteraction) {
    use xylitol_tui::mix_rgb;
    let ansi = {
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        fx.render_lines(80).join("\n")
    };
    let theme = crate::app::tui::LayoutTheme::product_dark();
    let p = theme.palette();
    // att4 MAY soft-mix surface：产品轨色为 surface 与 vivid 的 0.72 混合。
    let cases = [
        ("pending.rs", mix_rgb(p.surface, p.accent, 0.72), "pending"),
        ("done.rs", mix_rgb(p.surface, p.success, 0.72), "success"),
        ("bad.rs", mix_rgb(p.surface, p.error, 0.72), "error"),
    ];
    for (needle, rgb, label) in cases {
        let prefix = rail_prefix(rgb);
        assert!(
            ansi.lines()
                .filter(|l| l.contains(needle))
                .any(|l| l.starts_with(&prefix)),
            "att4: {label} tool line must carry its status rail:\n{ansi:?}"
        );
    }
}

#[then("轨为单列加无底色 gutter 且外层背景以复位码收束")]
fn then_rail_shape_and_reset(tui_interaction: &TuiInteraction) {
    let (done, prefix) = {
        use xylitol_tui::mix_rgb;
        let theme = crate::app::tui::LayoutTheme::product_dark();
        let p = theme.palette();
        let prefix = rail_prefix(mix_rgb(p.surface, p.success, 0.72));
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        let ansi = fx.render_lines(80).join("\n");
        let done = ansi
            .lines()
            .find(|l| l.contains("done.rs"))
            .expect("success tool line")
            .to_string();
        (done, prefix)
    };
    // rail_prefix 末位即 \x1b[49m 复位 + 无底色 gutter 空格（结构即断言）。
    assert!(
        done.starts_with(&prefix),
        "att4: rail is one bg cell + 49m reset + bare gutter:\n{done:?}"
    );
}

#[then("内容行除轨外无整行洗底")]
fn then_no_full_row_wash(tui_interaction: &TuiInteraction) {
    let (done, prefix) = {
        use xylitol_tui::mix_rgb;
        let theme = crate::app::tui::LayoutTheme::product_dark();
        let p = theme.palette();
        let prefix = rail_prefix(mix_rgb(p.surface, p.success, 0.72));
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        let ansi = fx.render_lines(80).join("\n");
        let done = ansi
            .lines()
            .find(|l| l.contains("done.rs"))
            .expect("success tool line")
            .to_string();
        (done, prefix)
    };
    let rest = &done[prefix.len()..];
    assert!(
        !rest.contains("48;2;"),
        "att4: content area must not be washed with tool bg:\n{done:?}"
    );
}

// ---- att14：write 正文视口 + edit diff 默认可见 ----

fn long_body(lines: usize) -> String {
    (1..=lines)
        .map(|i| format!("body-line-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[when("以场景构建器回放超长 write 正文并挂载交互面")]
fn when_mount_long_write(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionStart {
        id: "w1".into(),
        name: "write".into(),
        args: serde_json::json!({ "path": "docs/x.md", "content": long_body(30) }),
    });
    seal_and_open_cluster(&mut fx);
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[then("正文视口至多 10 行尾且以 total 加 ctrl+o 提示省略")]
fn then_write_viewport_tail(tui_interaction: &TuiInteraction) {
    let plain = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(80)
    };
    assert!(
        plain.contains("body-line-30"),
        "tail window keeps the last line:\n{plain}"
    );
    assert!(
        !plain.contains("body-line-01"),
        "early lines must stay outside the 10-line tail viewport:\n{plain}"
    );
    assert!(
        plain.contains("30 total") && plain.contains("ctrl+o to expand"),
        "att14: omitted lines are announced with count + ctrl+o hint:\n{plain}"
    );
}

#[then("write 头行与正文共用同一状态轨")]
fn then_write_shared_rail(tui_interaction: &TuiInteraction) {
    use xylitol_tui::mix_rgb;
    let (header, body, prefix) = {
        let theme = crate::app::tui::LayoutTheme::product_dark();
        let p = theme.palette();
        let prefix = rail_prefix(mix_rgb(p.surface, p.accent, 0.72));
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        let ansi = fx.render_lines(80).join("\n");
        let header = ansi
            .lines()
            .find(|l| l.contains("docs/x.md"))
            .expect("write header line")
            .to_string();
        let body = ansi
            .lines()
            .find(|l| l.contains("body-line-30"))
            .expect("write body line")
            .to_string();
        (header, body, prefix)
    };
    assert!(
        header.starts_with(&prefix) && body.starts_with(&prefix),
        "att14: header and body must share one status rail:\n{header:?}\n{body:?}"
    );
}

#[when("以场景构建器回放 edit 成功并挂载交互面")]
fn when_mount_edit_success(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    sb.tool_start("e1", "edit", "a.rs");
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "e1".into(),
        name: "edit".into(),
        result: serde_json::json!({
            "display_diff": "@@ -1,2 +1,3 @@\n ctx\n-removed\n+added"
        })
        .to_string(),
        is_error: false,
    });
    seal_and_open_cluster(&mut fx);
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[then("diff 正文默认可见且头行无状态字面标签")]
fn then_edit_diff_default_visible(tui_interaction: &TuiInteraction) {
    let plain = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(80)
    };
    assert!(
        plain.contains("+added") && plain.contains("-removed"),
        "att14: edit diff must be visible by default (no Alt+E needed):\n{plain}"
    );
    for label in ["[ok]", "[err]", "[…]"] {
        assert!(
            !plain.contains(label),
            "att14: headers must not embed literal status labels: {label}"
        );
    }
}

// ---- att15：[Full output: 脚注 warning 前景；未截断不伪造 ----

const FULL_OUTPUT_FOOTER: &str =
    "[Full output: /tmp/x.log. Truncated: 20 lines shown (50.0KB limit)]";

fn bash_output(lines: usize, footer: Option<&str>) -> String {
    let mut out = (1..=lines)
        .map(|i| format!("line-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(f) = footer {
        out.push('\n');
        out.push_str(f);
    }
    out
}

/// att16 产品形状：硬截断 bash 结果为 JSON（truncated + combined 内含脚注）。
fn truncated_bash_result(lines: usize) -> String {
    serde_json::json!({
        "exit_code": 0,
        "truncated": true,
        "combined": bash_output(lines, Some(FULL_OUTPUT_FOOTER))
    })
    .to_string()
}

fn mount_bash_tool(tui_interaction: &TuiInteraction, result: String) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    sb.tool_start("b1", "bash", "/tmp/app");
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "b1".into(),
        name: "bash".into(),
        result,
        is_error: false,
    });
    seal_and_open_cluster(&mut fx);
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[when("以场景构建器回放带 Full output 脚注的 bash 工具并取 ANSI 帧")]
fn when_mount_truncated_bash(tui_interaction: &TuiInteraction) {
    mount_bash_tool(tui_interaction, truncated_bash_result(20));
}

#[then("脚注行以 warning 前景绘制且帧内可见脚注")]
fn then_footer_warning_fg(tui_interaction: &TuiInteraction) {
    use xylitol_tui::{bold, fg_rgb};
    let ansi = {
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        fx.render_lines(120).join("\n")
    };
    let expect = bold(&fg_rgb(
        crate::app::tui::LayoutTheme::product_dark()
            .palette()
            .warning,
        FULL_OUTPUT_FOOTER,
    ));
    assert!(
        ansi.contains(&expect),
        "att15: footer must paint warning fg (bold allowed):\n{ansi:?}"
    );
}

#[when("以场景构建器回放未截断的正常输出")]
fn when_mount_normal_bash(tui_interaction: &TuiInteraction) {
    mount_bash_tool(tui_interaction, bash_output(4, None));
}

#[then("帧内不出现伪造的 Full output 脚注")]
fn then_no_fabricated_footer(tui_interaction: &TuiInteraction) {
    let plain = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(120)
    };
    assert!(
        !plain.contains("[Full output:"),
        "att15: untruncated output must not fabricate the footer:\n{plain}"
    );
}

// ---- att16：硬截断禁视口展开；write 正文仍可 Ctrl+O ----

fn ctrl_key(ch: char) -> crossterm::event::KeyEvent {
    crossterm::event::KeyEvent {
        code: crossterm::event::KeyCode::Char(ch),
        modifiers: crossterm::event::KeyModifiers::CONTROL,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
}

#[when("以场景构建器回放硬截断 bash 工具并按下 Ctrl+O")]
fn when_mount_hard_truncated_bash(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    sb.tool_start("b1", "bash", "/tmp/app");
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "b1".into(),
        name: "bash".into(),
        result: truncated_bash_result(24),
        is_error: false,
    });
    seal_and_open_cluster(&mut fx);
    fx.handle_key(ctrl_key('o'));
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[then("视口保持尾窗且提示 expand disabled 且不出全文")]
fn then_hard_truncation_guard(tui_interaction: &TuiInteraction) {
    let plain = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(120)
    };
    assert!(
        plain.contains("expand disabled"),
        "att16: hard truncation must announce expand disabled:\n{plain}"
    );
    assert!(
        plain.contains("line-24"),
        "tail preview stays visible:\n{plain}"
    );
    assert!(
        !plain.contains("line-01"),
        "att16: Ctrl+O must NOT expand hard-truncated output to full text:\n{plain}"
    );
}

#[when("回放超长 write 正文并按下 Ctrl+O")]
fn when_mount_long_write_ctrl_o(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工").message_end();
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionStart {
        id: "w1".into(),
        name: "write".into(),
        args: serde_json::json!({ "path": "docs/x.md", "content": long_body(30) }),
    });
    seal_and_open_cluster(&mut fx);
    fx.handle_key(ctrl_key('o'));
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[then("write 正文可展开为全文")]
fn then_write_body_expands(tui_interaction: &TuiInteraction) {
    let plain = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(120)
    };
    assert!(
        plain.contains("body-line-01") && plain.contains("body-line-30"),
        "att16: write body viewport MUST still allow Ctrl+O expansion:\n{plain}"
    );
}

// ---- att23 / att26 / att27 / att28：信封嵌套与 ActivityFold 自动收纳 ----

/// 一轮会话条目：user → read 工具 → 中段正文 → edit 工具 → 末段正文。
/// 每轮投影 5 个 UiEntry（User/Tool/Assistant/Tool/Assistant），seg id = `seg-{5n}`。
fn activity_turns_fixture(turns: usize) -> (Vec<SessionEntry>, SessionTreeTravel) {
    fn add(
        entries: &mut Vec<SessionEntry>,
        parent: &mut Option<String>,
        last_id: &mut String,
        id: &str,
        message: serde_json::Value,
    ) {
        entries.push(SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.clone(),
                timestamp: 0,
            },
            message,
        }));
        *parent = Some(id.to_string());
        *last_id = id.to_string();
    }
    let mut entries = Vec::new();
    let mut parent = None;
    let mut last_id = String::new();
    for n in 0..turns {
        add(
            &mut entries,
            &mut parent,
            &mut last_id,
            &format!("u{n}"),
            serde_json::json!({
                "role": "user",
                "content": [{ "type": "text", "text": format!("u-turn-{n}") }],
                "timestamp": 0u64,
            }),
        );
        add(
            &mut entries,
            &mut parent,
            &mut last_id,
            &format!("c{n}-1"),
            serde_json::json!({
                "role": "assistant",
                "content": [{ "type": "toolCall", "id": format!("tc{n}-1"),
                              "name": "read", "arguments": { "path": format!("f{n}-1.rs") } }],
                "timestamp": 0u64,
            }),
        );
        add(
            &mut entries,
            &mut parent,
            &mut last_id,
            &format!("r{n}-1"),
            serde_json::json!({
                "role": "toolResult", "toolCallId": format!("tc{n}-1"), "toolName": "read",
                "content": [{ "type": "text", "text": "ok" }], "isError": false,
                "timestamp": 0u64,
            }),
        );
        add(
            &mut entries,
            &mut parent,
            &mut last_id,
            &format!("a{n}"),
            serde_json::json!({
                "role": "assistant",
                "content": [
                    { "type": "text", "text": format!("mid-turn-{n}") },
                    { "type": "toolCall", "id": format!("tc{n}-2"),
                      "name": "edit", "arguments": { "path": format!("f{n}-2.rs") } }
                ],
                "timestamp": 0u64,
            }),
        );
        add(
            &mut entries,
            &mut parent,
            &mut last_id,
            &format!("r{n}-2"),
            serde_json::json!({
                "role": "toolResult", "toolCallId": format!("tc{n}-2"), "toolName": "edit",
                "content": [{ "type": "text", "text": "patched" }], "isError": false,
                "timestamp": 0u64,
            }),
        );
        add(
            &mut entries,
            &mut parent,
            &mut last_id,
            &format!("e{n}"),
            serde_json::json!({
                "role": "assistant",
                "content": [{ "type": "text", "text": format!("end-turn-{n}") }],
                "timestamp": 0u64,
            }),
        );
    }
    let travel = SessionTreeTravel {
        kind: SessionTreeKind::MessageHistory,
        selected_id: last_id.clone(),
        leaf_id: Some(last_id),
        editor_text: None,
    };
    (entries, travel)
}

fn mount_activity_turns(tui_interaction: &TuiInteraction, turns: usize) {
    let (entries, travel) = activity_turns_fixture(turns);
    let mut model = UiModel::default();
    rebuild_scrollback_from_travel(&mut model, &entries, &travel);
    let mut fx = InteractionBdd::from_model(model);
    fx.open_hit_viewport(400);
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

fn frame_of(tui_interaction: &TuiInteraction, width: usize) -> String {
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut().expect("fixture mounted").render_plain(width)
}

fn worked_for_lines(frame: &str) -> usize {
    frame.lines().filter(|l| l.contains("Worked for")).count()
}

#[when("以场景构建器回放四轮活动并按回合结束收纳")]
fn when_mount_four_turns_turn_end(tui_interaction: &TuiInteraction) {
    mount_activity_turns(tui_interaction, 4);
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut().expect("fixture mounted").turn_end_activity();
}

#[when("左键单击折叠命中表中的信封三角列")]
fn when_click_envelope_triangle(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(120); // 重新注册命中区
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::Segment(_)))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("a registered envelope triangle");
    assert!(fx.left_click(hit.0, hit.1), "envelope click must consume");
}

#[then("最旧信封折叠为用户行加 Worked for 加末段正文且无内层块")]
fn then_oldest_envelope_collapsed(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert!(
        plain.contains("u-turn-0"),
        "att23: folded envelope keeps the user row:\n{plain}"
    );
    assert!(
        worked_for_lines(&plain) >= 1,
        "att23: folded envelope paints a Worked for head:\n{plain}"
    );
    assert!(
        plain.contains("end-turn-0"),
        "att23: folded envelope keeps the last assistant body:\n{plain}"
    );
    assert!(
        !plain.contains("mid-turn-0") && !plain.contains("Read f0-1.rs"),
        "att23: middle body and inner blocks must be stored away:\n{plain}"
    );
}

#[then("该信封定点展开且中间助手正文与簇头行重新可见")]
fn then_envelope_precise_expand(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    // att23 展开到簇头态（L2）：中间正文与簇头行可见；内层块仍由簇级折叠
    // 管辖（att25/att31），不在此处要求。
    assert!(
        plain.contains("mid-turn-0")
            && plain.contains("Explored 1 file")
            && plain.contains("Edited 1 file"),
        "att23: expanding the envelope reveals the middle body and cluster heads:\n{plain}"
    );
    assert!(
        worked_for_lines(&plain) >= 1,
        "att23: the Worked for head row stays for re-folding:\n{plain}"
    );
}

#[then("仅最旧两轮折叠为 Worked for 且近窗轮正文与簇头保持")]
fn then_turn_end_window(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert_eq!(
        worked_for_lines(&plain),
        2,
        "att26: keep_recent_turns=2 folds only turns outside the window:\n{plain}"
    );
    // 近窗轮 = 信封展开的簇头态：正文与簇头行可见（内层块归簇级折叠管辖）。
    assert!(
        plain.contains("mid-turn-2")
            && plain.contains("end-turn-2")
            && plain.contains("Explored 1 file"),
        "att26: near-window turns keep bodies and cluster heads visible:\n{plain}"
    );
}

#[when("以重建路径应用重建收纳")]
fn when_apply_rebuild_crush(tui_interaction: &TuiInteraction) {
    let (_, travel) = activity_turns_fixture(4);
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut()
        .expect("fixture mounted")
        .rebuild_activity(&[], &travel);
}

#[then("全部四轮折叠为 Worked for")]
fn then_rebuild_folds_all(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert_eq!(
        worked_for_lines(&plain),
        4,
        "att26: auto_on_rebuild folds every ended turn:\n{plain}"
    );
}

#[when("关闭 ActivityFold 重挂并按回合结束收纳")]
fn when_mount_disabled_fold(tui_interaction: &TuiInteraction) {
    use crate::app::tui::ActivityFoldSettings;
    mount_activity_turns(tui_interaction, 4);
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    fx.set_activity_settings(ActivityFoldSettings {
        enabled: false,
        ..ActivityFoldSettings::default()
    });
    fx.turn_end_activity();
}

#[then("全帧无 Worked for")]
fn then_no_envelope_when_disabled(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert!(
        !plain.contains("Worked for"),
        "att26: disabled ActivityFold keeps the full ledger:\n{plain}"
    );
    // 关闭收纳 ≠ 强制展开：密封簇保持默认收起（att25），簇头行可见即可再展开。
    assert!(
        plain.contains("Explored 1 file"),
        "att26: cluster heads stay visible when disabled:\n{plain}"
    );
}

#[then("折叠信封与收起簇头旁注均为 Alt+Shift+E")]
fn then_marker_chords(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    // 折叠信封（L3）与收起簇头（L2/virgin heads）的旁注都是「展开最近」和弦。
    for line in plain.lines().filter(|l| l.contains("Worked for")) {
        assert!(
            line.contains("(Alt+Shift+E)"),
            "att27: folded envelope head advertises the expand chord: {line:?}"
        );
    }
    assert!(
        plain.contains("▸ Explored 1 file · 1 read  (Alt+Shift+E)"),
        "att27: collapsed cluster heads advertise the same expand chord:\n{plain}"
    );
    assert!(
        plain.contains('▸'),
        "att27: fold glyphs follow att19:\n{plain}"
    );
}

#[then("展开簇头旁注为 Ctrl+Alt+Shift+E 且信封簇头行不带 (Alt+E)")]
fn then_expanded_head_chord(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    let expanded_head = plain
        .lines()
        .find(|l| l.contains("▾") && l.contains("Explored"))
        .expect("a cluster head was opened by the click");
    assert!(
        expanded_head.contains("(Ctrl+Alt+Shift+E)"),
        "att27: expanded cluster head advertises the collapse chord: {expanded_head:?}"
    );
    for line in plain.lines() {
        let is_head = line.contains("Worked for")
            || line.contains("Explored")
            || line.contains("Edited")
            || line.contains("Ran ");
        assert!(
            !is_head || !line.contains("(Alt+E)"),
            "att27: envelope/cluster heads must not carry the block chord: {line:?}"
        );
    }
}

#[then("全帧无 Planning next moves")]
fn then_no_planning_placeholder(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert!(
        !plain.contains("Planning next moves"),
        "att27: no placeholder rows:\n{plain}"
    );
}

// ---- att28：跨面动作 expandNearest / collapseNearest（键位路径） ----

#[when("以场景构建器回放四轮活动并按重建收纳")]
fn when_mount_four_turns_rebuild(tui_interaction: &TuiInteraction) {
    mount_activity_turns(tui_interaction, 4);
    when_apply_rebuild_crush(tui_interaction);
}

fn chord_key(ch: char, modifiers: crossterm::event::KeyModifiers) -> crossterm::event::KeyEvent {
    crossterm::event::KeyEvent {
        code: crossterm::event::KeyCode::Char(ch),
        modifiers,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
}

#[when("按下和弦 Alt+Shift+E")]
fn when_press_alt_shift_e(tui_interaction: &TuiInteraction) {
    use crossterm::event::KeyModifiers;
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut()
        .expect("fixture mounted")
        .handle_key(chord_key('e', KeyModifiers::ALT | KeyModifiers::SHIFT));
}

#[when("再按下和弦 Alt+Shift+E")]
fn when_press_alt_shift_e_again(tui_interaction: &TuiInteraction) {
    when_press_alt_shift_e(tui_interaction);
}

#[when("按下和弦 Ctrl+Alt+Shift+E")]
fn when_press_ctrl_alt_shift_e(tui_interaction: &TuiInteraction) {
    use crossterm::event::KeyModifiers;
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut().expect("fixture mounted").handle_key(chord_key(
        'e',
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
}

#[when("再按下和弦 Ctrl+Alt+Shift+E")]
fn when_press_ctrl_alt_shift_e_again(tui_interaction: &TuiInteraction) {
    when_press_ctrl_alt_shift_e(tui_interaction);
}

#[then("最近信封降为簇头态且 Worked for 保持")]
fn then_nearest_expanded_to_heads(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert!(
        plain.contains("Explored 1 file") && plain.contains("Edited 1 file"),
        "att28: expandNearest opens the nearest folded envelope to cluster heads:\n{plain}"
    );
    assert!(
        !plain.contains("Read f3-1.rs"),
        "att28: cluster heads only — inner blocks stay hidden:\n{plain}"
    );
    assert_eq!(
        worked_for_lines(&plain),
        4,
        "att28: the envelope head stays while at cluster level:\n{plain}"
    );
}

#[then("次近折叠信封降为簇头态且最近簇头保持")]
fn then_next_envelope_to_heads(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    // c2540: heads are count-only now — distinguish envelopes by occurrence.
    assert_eq!(
        plain.matches("Explored 1 file").count(),
        2,
        "att28: expandNearest proceeds to the next nearest folded envelope:\n{plain}"
    );
}

#[then("最近展开信封收回为 Worked for 折叠态")]
fn then_nearest_envelope_recollapsed(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert_eq!(
        plain.matches("Explored 1 file").count(),
        1,
        "att28: collapseNearest re-folds the nearest non-L3 envelope:\n{plain}"
    );
}

#[then("全部信封收回为 Worked for 折叠态")]
fn then_all_envelopes_recollapsed(tui_interaction: &TuiInteraction) {
    let plain = frame_of(tui_interaction, 120);
    assert_eq!(
        plain.matches("Explored 1 file").count(),
        0,
        "att28: second collapseNearest re-folds the remaining envelope:\n{plain}"
    );
    assert_eq!(
        worked_for_lines(&plain),
        4,
        "att28: every turn is back under a Worked for head:\n{plain}"
    );
}

// ── c2510/att35: explore cluster head category-count suffix ────────

#[when("以场景构建器回放多读多检索序列（三读两检索且检索无路径）")]
fn when_replay_multi_read_search(transcript_bdd: &TranscriptBdd) {
    let plain = render_plain(|sb| {
        sb.tool_start("r1", "read", "a.rs")
            .tool_end("r1", "read")
            .tool_start("r2", "read", "b.rs")
            .tool_end("r2", "read")
            .tool_start("r3", "read", "c.rs")
            .tool_end("r3", "read")
            .tool_start("g1", "grep", "")
            .tool_end("g1", "grep")
            .tool_start("g2", "grep", "")
            .tool_end("g2", "grep")
            .assistant("seal");
    });
    *transcript_bdd.frames.borrow_mut() = vec![plain];
}

#[when("以场景构建器回放同类别单序列（三读）")]
fn when_replay_reads_only(transcript_bdd: &TranscriptBdd) {
    let plain = render_plain(|sb| {
        sb.tool_start("r1", "read", "a.rs")
            .tool_end("r1", "read")
            .tool_start("r2", "read", "b.rs")
            .tool_end("r2", "read")
            .tool_start("r3", "read", "c.rs")
            .tool_end("r3", "read")
            .assistant("seal");
    });
    *transcript_bdd.frames.borrow_mut() = vec![plain];
}

#[then("封口簇头 MUST 同时含文件计数与类目计数后缀（3 files · 3 reads · 2 searches）")]
fn then_head_file_and_call_counts(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    assert!(
        plain.contains("Explored 3 files · 3 reads · 2 searches"),
        "head must carry file count then invocation counts:\n{plain}"
    );
}

#[then("后缀 MUST 只列该非零类目")]
fn then_suffix_single_category(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    let plain = frames.last().expect("frame");
    assert!(plain.contains("· 3 reads"), "{plain}");
    assert!(
        !plain.contains("search"),
        "single-category suffix must omit searches:\n{plain}"
    );
}

#[given("探索簇流式进行中")]
fn given_live_explore_cluster(transcript_bdd: &TranscriptBdd) {
    let mut sb = SceneBuilder::begin();
    sb.tool_start("r1", "read", "a.rs");
    let (plain, _) = sb.render(80);
    *transcript_bdd.frames.borrow_mut() = vec![plain];
    *transcript_bdd.scene.borrow_mut() = Some(sb);
}

#[when("新的读段或检索段开始")]
fn when_new_tool_starts(transcript_bdd: &TranscriptBdd) {
    let mut scene = transcript_bdd.scene.borrow_mut();
    let sb = scene.as_mut().expect("live scene");
    sb.tool_end("r1", "read");
    sb.tool_start("r2", "read", "b.rs");
    let (plain, _) = sb.render(80);
    transcript_bdd.frames.borrow_mut().push(plain);
}

#[then("簇头后缀计数 MUST 随工具开始更新且进行时词形 MUST 为 Exploring")]
fn then_suffix_updates_live(transcript_bdd: &TranscriptBdd) {
    let frames = transcript_bdd.frames.borrow();
    assert_eq!(frames.len(), 2, "two streaming frames");
    let (first, second) = (&frames[0], &frames[1]);
    assert!(
        first.contains("Exploring") && first.contains("· 1 read"),
        "first frame must be Exploring with 1 read:\n{first}"
    );
    assert!(
        second.contains("Exploring") && second.contains("· 2 reads"),
        "suffix count MUST update as tools start:\n{second}"
    );
    assert!(
        !second.contains("· 1 read"),
        "stale count must not linger:\n{second}"
    );
}

// ---- att36：todo_* 块 body 清单渲染（直播 vs travel 重建同构） ----

/// 一轮 todo 会话条目：user → assistant(toolCall todo_rewrite) → toolResult 全表快照
/// + 一条 agent_todo Custom 快照（SSOT，resume 侧 checklist 来源）。
fn todo_block_fixture_entries() -> (Vec<SessionEntry>, SessionTreeTravel) {
    use crate::protocol::session::CustomEntry;
    let todo_json = serde_json::json!({
        "items": [
            { "id": "a", "content": "检查环境", "status": "in_progress" },
            { "id": "b", "content": "写清单", "status": "pending" }
        ]
    });
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
                "content": [{ "type": "text", "text": "plan it" }],
                "timestamp": 0u64,
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
                    { "type": "toolCall", "id": "tc-todo", "name": "todo_rewrite",
                      "arguments": { "items": [
                          { "id": "a", "content": "检查环境", "status": "in_progress" },
                          { "id": "b", "content": "写清单" }
                      ] } }
                ],
                "timestamp": 0u64,
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
                "toolCallId": "tc-todo",
                "toolName": "todo_rewrite",
                "content": [{ "type": "text", "text": todo_json.to_string() }],
                "isError": false,
                "timestamp": 0u64,
            }),
        }),
        SessionEntry::Custom(CustomEntry {
            base: crate::protocol::session::EntryBase {
                entry_type: "custom".into(),
                id: "snap1".into(),
                parent_id: Some("tr1".into()),
                timestamp: 0,
            },
            custom_type: crate::protocol::session::CUSTOM_TYPE_AGENT_TODO.into(),
            data: todo_json.clone(),
        }),
    ];
    let travel = SessionTreeTravel {
        kind: SessionTreeKind::MessageHistory,
        selected_id: "snap1".into(),
        leaf_id: Some("snap1".into()),
        editor_text: None,
    };
    (entries, travel)
}

#[when("以场景构建器回放 todo_rewrite 成功调用的直播与 travel 重建")]
fn when_todo_block_live_and_rebuild(transcript_bdd: &TranscriptBdd) {
    let (entries, travel) = todo_block_fixture_entries();
    let mut rebuilt = UiModel::default();
    rebuild_scrollback_from_travel(&mut rebuilt, &entries, &travel);

    // 同一轮的直播事件流：Start → TodoUpdated（typed，atd13）→ End。
    let todo_args = serde_json::json!({ "items": [
        { "id": "a", "content": "检查环境", "status": "in_progress" },
        { "id": "b", "content": "写清单" }
    ] });
    let todo_result = serde_json::json!({ "items": [
        { "id": "a", "content": "检查环境", "status": "in_progress" },
        { "id": "b", "content": "写清单", "status": "pending" }
    ] })
    .to_string();
    let mut live = UiModel::default();
    apply_xy_event(
        &mut live,
        &XyEvent::ToolExecutionStart {
            id: "tc-todo".into(),
            name: "todo_rewrite".into(),
            args: todo_args,
        },
    );
    let list = crate::protocol::session::TodoList::from_data_value(
        &serde_json::from_str::<serde_json::Value>(&todo_result).expect("todo json"),
    )
    .expect("typed list");
    apply_xy_event(&mut live, &XyEvent::TodoUpdated { list });
    apply_xy_event(
        &mut live,
        &XyEvent::ToolExecutionEnd {
            id: "tc-todo".into(),
            name: "todo_rewrite".into(),
            result: todo_result,
            is_error: false,
        },
    );

    *transcript_bdd.models.borrow_mut() = vec![rebuilt, live];
}

#[then("两种路径的块 body MUST 均为清单行形态且逐行一致，MUST NOT 出现原始 items JSON")]
fn then_todo_block_bodies_agree(transcript_bdd: &TranscriptBdd) {
    let models = transcript_bdd.models.borrow();

    fn todo_tool(model: &UiModel) -> (String, String) {
        model
            .entries
            .iter()
            .find_map(|e| match e {
                UiEntry::Tool {
                    args_preview,
                    output,
                    ..
                } if !output.is_empty() || !args_preview.is_empty() => {
                    Some((args_preview.clone(), output.clone()))
                }
                _ => None,
            })
            .expect("todo tool row")
    }
    fn checklist(model: &UiModel) -> (String, Vec<String>) {
        model
            .entries
            .iter()
            .find_map(|e| match e {
                UiEntry::Todo {
                    summary,
                    detail_lines,
                } => Some((summary.clone(), detail_lines.clone())),
                _ => None,
            })
            .expect("checklist projection row")
    }

    let (rebuilt_preview, rebuilt_body) = todo_tool(&models[0]);
    let (live_preview, live_body) = todo_tool(&models[1]);
    assert_eq!(
        rebuilt_preview, live_preview,
        "att13: header preview must be identical across paths"
    );
    assert_eq!(live_preview, "2 items · 1 in progress");
    assert_eq!(
        rebuilt_body, live_body,
        "att36: block body must be line-by-line identical across paths"
    );
    assert_eq!(live_body, "[~] 检查环境\n[ ] 写清单");
    for body in [&rebuilt_body, &live_body] {
        assert!(
            !body.contains("{\"items\""),
            "att36: raw items JSON must not be the block body: {body}"
        );
    }

    // checklist projection row：latest-wins、同字形、两路径一致。
    let (rebuilt_summary, rebuilt_lines) = checklist(&models[0]);
    let (live_summary, live_lines) = checklist(&models[1]);
    assert_eq!(rebuilt_summary, live_summary);
    assert_eq!(rebuilt_summary, "Todo · 0/2");
    assert_eq!(rebuilt_lines, live_lines);
    assert_eq!(rebuilt_lines, vec!["[~] 检查环境", "[ ] 写清单"]);
}
