//! Steps for `app-tui-transcript` P3 — headless keyboard/mouse interaction
//! over the product `UiRoot` via [`InteractionBdd`] (att20–att32).
//!
//! Paths exercised are the product's own: chord routing (`UiRoot::handle_key`),
//! the shared fold-toggle pipeline (`UiRoot::click_fold_at`, also called by the
//! host hit-priority wiring), and product paint for frames.

use crate::app::tui::{FoldTarget, InteractionBdd, SceneBuilder, UiModel};
use crate::tests::bdd::prelude::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol_tui::TreeNode;

/// Shared state for interaction scenes.
pub struct TuiInteraction {
    pub fx: RefCell<Option<InteractionBdd>>,
    /// Frame snapshot taken at mount time (layered-collapse comparison).
    pub mounted_frame: RefCell<String>,
    /// Target id clicked most recently ("cluster" or the block id).
    pub last_clicked: RefCell<Option<String>>,
}

#[fixture]
pub fn tui_interaction() -> TuiInteraction {
    TuiInteraction {
        fx: RefCell::new(None),
        mounted_frame: RefCell::new(String::new()),
        last_clicked: RefCell::new(None),
    }
}

fn ctrl(ch: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(ch),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn alt(ch: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(ch),
        modifiers: KeyModifiers::ALT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Read a sealed scene onto a fresh product root and take the first paint
/// (the paint registers the fold hit table).
fn mount(fx: &mut InteractionBdd) -> String {
    fx.push_xy(XyEvent::AgentEnd {
        messages: Vec::new(),
    });
    fx.open_hit_viewport(200);
    let plain = fx.render_plain(80);
    let _ = fx.fold_hits().regions.len();
    plain
}

/// read old.rs → edit a.rs, with an assistant body between, one closed turn.
fn replay_read_edit() -> InteractionBdd {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工");
    sb.tool_start("t-read", "read", "old.rs");
    sb.tool_end("t-read", "read");
    sb.message_end();
    sb.thinking_flushed("看看 diff", 3);
    sb.tool_start("t-edit", "edit", "a.rs");
    sb.message_end();
    InteractionBdd::from_model(sb.into_model())
}

#[when("以场景构建器回放读后改写序列并封轮挂载交互面")]
fn when_mount_read_edit(tui_interaction: &TuiInteraction) {
    let mut fx = replay_read_edit();
    let plain = mount(&mut fx);
    *tui_interaction.mounted_frame.borrow_mut() = plain;
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[when("以场景构建器回放思考加工具并封轮挂载交互面")]
fn when_mount_think_tool(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开工");
    sb.message_end();
    sb.thinking_flushed("先想一想", 4);
    sb.thinking_flushed("再看一眼", 6);
    sb.tool_start("t-grep", "grep", "src");
    sb.tool_end("t-grep", "grep");
    sb.message_end();
    let mut fx = InteractionBdd::from_model(sb.into_model());
    let plain = mount(&mut fx);
    *tui_interaction.mounted_frame.borrow_mut() = plain;
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[then("簇即为折叠收纳态且无内层块登记")]
fn then_storage_collapsed_no_inner_hits(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("interaction fixture mounted");
    assert!(
        !tui_interaction
            .mounted_frame
            .borrow()
            .contains("Read old.rs"),
        "collapsed storage must not render inner blocks"
    );
    let inner = fx.fold_hits().regions.iter().any(|r| {
        matches!(
            &r.target,
            FoldTarget::Tool(_) | FoldTarget::Diff(_) | FoldTarget::Thinking(_)
        )
    });
    assert!(!inner, "no inner triangle may be registered while folded");
}

#[when("左键单击折叠命中表中的簇头三角列")]
fn when_click_cluster_triangle(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::Cluster(_)))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("a registered cluster header region");
    assert!(fx.left_click(hit.0, hit.1), "cluster click must consume");
    *tui_interaction.last_clicked.borrow_mut() = Some("cluster".into());
}

#[when("再左键单击 \"{id}\" 的工具块三角列")]
fn when_click_tool_triangle(tui_interaction: &TuiInteraction, id: String) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    let target_id = id.clone();
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::Tool(tid) if *tid == target_id))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .unwrap_or_else(|| panic!("tool `{target_id}` must be registered"));
    assert!(fx.left_click(hit.0, hit.1));
    *tui_interaction.last_clicked.borrow_mut() = Some(id);
}

#[when("再左键单击第一条思考的折叠三角列")]
fn when_click_first_thinking(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    let (col, row, id) = fx
        .fold_hits()
        .regions
        .iter()
        .find_map(|r| match &r.target {
            FoldTarget::Thinking(id) => {
                Some((r.col_start as u16, r.content_row as u16, id.clone()))
            }
            _ => None,
        })
        .expect("first thinking triangle must be registered");
    assert!(fx.left_click(col, row));
    *tui_interaction.last_clicked.borrow_mut() = Some(id);
}

#[when("点击该行折叠命中区右边界之外的一列")]
fn when_click_outside_triangle_column(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    // First registered inner block row; aim one full column past its end.
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| {
            matches!(
                &r.target,
                FoldTarget::Tool(_) | FoldTarget::Thinking(_) | FoldTarget::Diff(_)
            )
        })
        .map(|r| (r.col_end as u16, r.content_row as u16))
        .expect("an inner block must be registered after expanding");
    let consumed = fx.left_click(hit.0, hit.1);
    assert!(!consumed, "non-triangle columns must not toggle folds");
    *tui_interaction.last_clicked.borrow_mut() = None;
}

#[then("折叠命中不消费且覆盖表为空")]
fn then_no_consumption_no_overrides(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(fx.fold().tools_overrides.is_empty());
    assert!(fx.fold().thinking_overrides.is_empty());
}

#[then("该块经覆盖表收起且覆盖表只有这一个条目")]
fn then_only_clicked_override(tui_interaction: &TuiInteraction) {
    let id = tui_interaction
        .last_clicked
        .borrow()
        .clone()
        .expect("a block was clicked");
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert_eq!(fx.fold().tools_overrides.len(), 1);
    assert_eq!(
        fx.tools_effective(&id),
        false,
        "clicked tool collapsed via override"
    );
}

#[then("仅该簇被定点展开且内层块行可见")]
fn then_cluster_precise_expand(tui_interaction: &TuiInteraction) {
    let frame = {
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        fx.render_plain(80)
    };
    assert!(
        frame.contains("Read old.rs"),
        "expanded cluster must reveal inner blocks: {frame}"
    );
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(
        fx.fold_hits()
            .regions
            .iter()
            .any(|r| matches!(&r.target, FoldTarget::Tool(_))),
        "inner triangles register once expanded"
    );
}

#[then("该条思考经覆盖展开且另一条不受影响")]
fn then_one_thinking_override(tui_interaction: &TuiInteraction) {
    let id = tui_interaction
        .last_clicked
        .borrow()
        .clone()
        .expect("a thinking entry was clicked");
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert_eq!(
        fx.fold().thinking_overrides.len(),
        1,
        "exactly one per-id override"
    );
    assert_eq!(fx.thinking_effective(&id), true);
}

#[when("按下和弦 Alt+E")]
fn when_press_alt_e(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut().expect("fixture mounted").handle_key(alt('e'));
}

#[when("按下和弦 Ctrl+T")]
fn when_press_ctrl_t(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut().expect("fixture mounted").handle_key(ctrl('t'));
}

#[when("依序按下和弦 Alt+E、Ctrl+T、Ctrl+O")]
fn when_press_alt_e_ctrl_t_ctrl_o(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    fx.handle_key(alt('e'))
        .handle_key(ctrl('t'))
        .handle_key(ctrl('o'));
}

#[when("以场景构建器回放压缩事件并封轮挂载交互面")]
fn when_mount_compaction(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("开始整理");
    sb.message_end();
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::CompactionStart {
        reason: "threshold".into(),
    });
    fx.push_xy(XyEvent::CompactionEnd {
        result: None,
        aborted: false,
        reason: "threshold".into(),
        will_retry: false,
        error_message: None,
        summary: Some("摘要完成".into()),
        tokens_before: Some(1200),
    });
    let plain = mount(&mut fx);
    *tui_interaction.mounted_frame.borrow_mut() = plain;
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[when("左键单击 Compaction 块的折叠三角列")]
fn when_click_compaction_triangle(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::Compaction))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("a registered compaction triangle");
    assert!(fx.left_click(hit.0, hit.1), "compaction click must consume");
}

#[then("Compaction 全局展开态翻转")]
fn then_compaction_global_toggled(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(
        fx.fold().compaction_expanded,
        "one click on the compaction triangle expands the global state"
    );
}

#[then("工具族覆盖表仍为空且工具族默认态未变")]
fn then_tools_family_untouched(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(fx.fold().tools_overrides.is_empty());
    assert!(fx.fold().tools_expanded, "tools default stays open");
}

fn long_output(lines: usize) -> String {
    (1..=lines)
        .map(|i| format!("line-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[when("以场景构建器回放多行输出的工具并封轮挂载交互面")]
fn when_mount_long_output_tool(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("看日志");
    sb.message_end();
    sb.tool_start("t-log", "bash", "/tmp/app");
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "t-log".into(),
        name: "bash".into(),
        result: long_output(24),
        is_error: false,
    });
    let plain = mount(&mut fx);
    *tui_interaction.mounted_frame.borrow_mut() = plain;
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[when("点击输出的 Ctrl+O 提示带")]
fn when_click_viewport_hint_band(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::OutputViewport(_)))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("a registered output viewport hint band");
    assert!(fx.left_click(hit.0, hit.1), "hint band click must consume");
}

#[then("提示带点击仅翻转该块输出视口且 Ctrl+O 翻全局并清按块覆盖")]
fn then_hint_click_per_block_ctrl_o_global(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    // Per-block override on the clicked block; the global default stays put.
    assert!(
        fx.fold().output_effective("t-log"),
        "hint-band click expands the clicked block"
    );
    assert!(
        !fx.fold().tools_output_expanded,
        "per-block click must not flip the global default"
    );
    // Keyboard Ctrl+O flips the global default and clears per-block overrides.
    fx.handle_key(ctrl('o'));
    assert!(
        fx.fold().tools_output_expanded,
        "Ctrl+O flips the global default"
    );
    assert!(
        fx.fold().output_overrides.is_empty(),
        "Ctrl+O clears per-block overrides, got {:?}",
        fx.fold().output_overrides
    );
    assert!(fx.fold().output_effective("t-log"));
    fx.handle_key(ctrl('o'));
    assert!(!fx.fold().tools_output_expanded);
    assert!(
        !fx.fold().output_effective("t-log"),
        "global collapse applies to the block again"
    );
}

#[when("以场景构建器回放两个多行输出工具并封轮挂载交互面")]
fn when_mount_two_long_output_tools(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("看两份日志");
    sb.message_end();
    sb.tool_start("t-a", "bash", "/tmp/a");
    sb.tool_start("t-b", "bash", "/tmp/b");
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "t-a".into(),
        name: "bash".into(),
        result: long_output(24),
        is_error: false,
    });
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "t-b".into(),
        name: "bash".into(),
        result: long_output(24),
        is_error: false,
    });
    let plain = mount(&mut fx);
    *tui_interaction.mounted_frame.borrow_mut() = plain;
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[when("点击第一个工具的 Ctrl+O 提示带")]
fn when_click_first_tool_hint_band(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::OutputViewport(id) if id.as_str() == "t-a"))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("first tool hint band registered");
    assert!(fx.left_click(hit.0, hit.1), "hint band click must consume");
}

#[then("仅该块展开且另一块保持折叠且折叠带可点回折")]
fn then_per_block_expand_and_fold_band(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    assert!(fx.fold().output_effective("t-a"), "clicked block expands");
    assert!(
        !fx.fold().output_effective("t-b"),
        "sibling block stays collapsed"
    );
    assert!(
        !fx.fold().tools_output_expanded,
        "global default untouched by per-block click"
    );
    // Expanded block paints a fold-back band; clicking it folds only that block.
    let _ = fx.render_plain(80);
    let fold_band = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::OutputViewport(id) if id.as_str() == "t-a"))
        .map(|r| (r.col_start as u16, r.content_row as u16))
        .expect("expanded fold band registered");
    assert!(
        fx.left_click(fold_band.0, fold_band.1),
        "fold band click must consume"
    );
    assert!(
        !fx.fold().output_effective("t-a"),
        "fold band collapses the clicked block"
    );
    assert!(
        !fx.fold().output_effective("t-b"),
        "sibling block unaffected by fold"
    );
}

#[when("以场景构建器回放压缩加多行输出并封轮挂载交互面")]
fn when_mount_compaction_and_long_output(tui_interaction: &TuiInteraction) {
    let mut sb = SceneBuilder::begin();
    sb.assistant("记录并清理");
    sb.message_end();
    sb.tool_start("t-read2", "read", "big.txt");
    let mut fx = InteractionBdd::from_model(sb.into_model());
    fx.push_xy(XyEvent::ToolExecutionEnd {
        id: "t-read2".into(),
        name: "read".into(),
        result: long_output(18),
        is_error: false,
    });
    fx.push_xy(XyEvent::CompactionStart {
        reason: "threshold".into(),
    });
    fx.push_xy(XyEvent::CompactionEnd {
        result: None,
        aborted: false,
        reason: "threshold".into(),
        will_retry: false,
        error_message: None,
        summary: Some("压缩完成".into()),
        tokens_before: Some(900),
    });
    let plain = mount(&mut fx);
    *tui_interaction.mounted_frame.borrow_mut() = plain;
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[then("Compaction、输出提示带与块级三角登记于同一命中表且定点可点")]
fn then_targets_share_one_hit_table(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    // After the `when` step opened the cluster, all three remaining target
    // kinds must live in the *same* registered table and be precisely
    // clickable. Each click repaints, so re-locate before every click — the
    // real engine does the same against fresh frames.
    let locate = |fx: &mut InteractionBdd, pred: &dyn Fn(&FoldTarget) -> bool| -> (u16, u16) {
        let _ = fx.render_plain(80);
        fx.fold_hits()
            .regions
            .iter()
            .find(|r| pred(&r.target))
            .map(|r| (r.col_start as u16, r.content_row as u16))
            .expect("target kind must be registered")
    };

    let compaction = locate(fx, &|t| matches!(t, FoldTarget::Compaction));
    assert!(fx.left_click(compaction.0, compaction.1));
    let band = locate(fx, &|t| matches!(t, FoldTarget::OutputViewport(_)));
    assert!(fx.left_click(band.0, band.1));
    let block = locate(fx, &|t| matches!(t, FoldTarget::Tool(_)));
    assert!(fx.left_click(block.0, block.1));
}

#[when("点击簇头行的摘要正文列而非三角列")]
fn when_click_cluster_body_column(tui_interaction: &TuiInteraction) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    let _ = fx.render_plain(80); // refresh the registered regions
    // Cluster header region is the triangle column `[0, 1)`; column 1 onward
    // is summary text / chord annotation — clicking it must not toggle.
    let hit = fx
        .fold_hits()
        .regions
        .iter()
        .find(|r| matches!(&r.target, FoldTarget::Cluster(_)))
        .map(|r| (r.col_end as u16, r.content_row as u16))
        .expect("a registered cluster header region");
    assert!(
        !fx.left_click(hit.0, hit.1),
        "summary-text columns must not consume fold hits"
    );
}

#[then("工具族默认展开态翻转且块级覆盖清空")]
fn then_tools_default_flipped_overrides_cleared(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    let fold = fx.fold();
    assert!(
        fold.tools_overrides.is_empty(),
        "Alt+E clears family overrides, got {:?}",
        fold.tools_overrides
    );
    assert_eq!(fold.tools_expanded, false);
    assert_eq!(fx.tools_effective("t-read"), false);
    assert_eq!(fx.tools_effective("t-edit"), false);
}

#[then("思考默认展开态翻转且全部 per-id 覆盖清空")]
fn then_thinking_default_flipped_overrides_cleared(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    let fold = fx.fold();
    assert!(
        fold.thinking_overrides.is_empty(),
        "Ctrl+T clears all thinking overrides, got {:?}",
        fold.thinking_overrides
    );
    assert_eq!(fold.thinking_expanded, true);
}

#[then("渲染帧与按键前逐字一致")]
fn then_frame_unchanged_after_chords(tui_interaction: &TuiInteraction) {
    let now = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(80)
    };
    let before = tui_interaction.mounted_frame.borrow();
    assert_eq!(
        *before, now,
        "chords must not alter a collapsed storage's appearance"
    );
}

#[then("收起块保留摘要行且展开旁注为括号完整和弦")]
fn then_collapsed_summary_and_chord_hints(tui_interaction: &TuiInteraction) {
    let frame = {
        let mut fx = tui_interaction.fx.borrow_mut();
        fx.as_mut().expect("fixture mounted").render_plain(80)
    };
    assert!(
        frame.contains("Read old.rs"),
        "collapsed block keeps a readable summary line: {frame}"
    );
    assert!(
        frame.contains("(Alt+E)"),
        "tool chord hint is a full parenthesised chord"
    );
}

// ── session-tree slot key family (ati22–ati27 / ati36) ──

/// Decode chord specs like `Ctrl+Shift+O` / `Alt+Right` / `Esc` / `Tab`.
fn parse_chord(spec: &str) -> KeyEvent {
    let mut modifiers = KeyModifiers::NONE;
    let mut code = KeyCode::Null;
    for part in spec.split('+') {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "esc" => code = KeyCode::Esc,
            "enter" => code = KeyCode::Enter,
            "left" => code = KeyCode::Left,
            "right" => code = KeyCode::Right,
            "up" => code = KeyCode::Up,
            "down" => code = KeyCode::Down,
            "tab" => code = KeyCode::Tab,
            other => code = KeyCode::Char(other.chars().next().expect("non-empty chord segment")),
        }
    }
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn tree_roots() -> Vec<TreeNode> {
    let parent = TreeNode::new("p1", "parent branch").with_child(TreeNode::new("c1", "child leaf"));
    vec![TreeNode::new("root", "main session"), parent]
}

#[when("打开样例会话树并挂载交互面")]
fn when_mount_tree(tui_interaction: &TuiInteraction) {
    let mut fx = InteractionBdd::from_model(UiModel::new());
    fx.mount_tree(tree_roots(), Some("root"));
    assert!(fx.is_tree_slot(), "tree slot must own the editor area");
    *tui_interaction.fx.borrow_mut() = Some(fx);
}

#[when("在树槽按下和弦 \"{chord}\"")]
fn when_press_chord_in_tree(tui_interaction: &TuiInteraction, chord: String) {
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut()
        .expect("fixture mounted")
        .handle_key(parse_chord(&chord));
}

#[when("选中节点 \"{id}\" 再收到和弦 \"{chord}\"")]
fn when_select_then_press(tui_interaction: &TuiInteraction, id: String, chord: String) {
    let mut fx = tui_interaction.fx.borrow_mut();
    let fx = fx.as_mut().expect("fixture mounted");
    assert!(
        fx.select_tree_node(&id),
        "node `{id}` must exist in the tree"
    );
    fx.handle_key(parse_chord(&chord));
}

#[then("树过滤模式为 \"{mode}\"")]
fn then_filter_mode_is(tui_interaction: &TuiInteraction, mode: String) {
    let fx = tui_interaction.fx.borrow();
    let got = fx
        .as_ref()
        .expect("fixture mounted")
        .tree_filter_name()
        .expect("tree slot owns the area");
    assert_eq!(got, mode, "FilterMode mismatch");
}

#[then("思考折叠默认态未被树槽过滤键触碰")]
fn then_thinking_default_untouched_by_tree_keys(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(!fx.fold().thinking_expanded);
    assert!(fx.fold().thinking_overrides.is_empty());
}

#[then("该节点子会话行被收起")]
fn then_node_folded(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    assert!(
        fx.as_ref().expect("fixture mounted").tree_node_folded("p1"),
        "parent children must be folded after Ctrl+Left"
    );
}

#[then("该节点子会话行重新展开")]
fn then_node_unfolded(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    assert!(
        !fx.as_ref().expect("fixture mounted").tree_node_folded("p1"),
        "parent children must be visible after Alt+Right"
    );
}

#[then("fork 请求交给主机且编辑器未收到字面输入")]
fn then_fork_pending_editor_clean(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert_eq!(
        fx.pending_tree_fork(),
        Some("root".to_string()),
        "Shift+F on a node hands its id to the host pump"
    );
    assert!(
        fx.editor_display_text().is_empty(),
        "editor must not receive the literal keystroke"
    );
}

#[when("挂载空模型的编辑器交互面")]
fn when_mount_empty_editor(tui_interaction: &TuiInteraction) {
    *tui_interaction.fx.borrow_mut() = Some(InteractionBdd::from_model(UiModel::new()));
}

#[when("在编辑器槽按下和弦 \"{chord}\"")]
fn when_press_chord_in_editor(tui_interaction: &TuiInteraction, chord: String) {
    let mut fx = tui_interaction.fx.borrow_mut();
    fx.as_mut()
        .expect("fixture mounted")
        .handle_key(parse_chord(&chord));
}

#[then("标签编辑在树内打开且编辑器未收到字面输入")]
fn then_label_edit_open(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(
        fx.tree_label_editing(),
        "Shift+L opens the in-tree label editor"
    );
    assert!(fx.editor_display_text().is_empty());
}

#[then("树仍开着且无标签写入动作排入")]
fn then_tree_open_no_label_writeback(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(
        fx.is_tree_slot(),
        "Esc during label edit cancels edit, not the tree"
    );
    assert!(!fx.tree_label_editing());
    assert!(fx.pending_tree_label().is_none());
}

#[then("编辑器文本保持为空且仍在编辑器槽")]
fn then_editor_slot_untouched(tui_interaction: &TuiInteraction) {
    let fx = tui_interaction.fx.borrow();
    let fx = fx.as_ref().expect("fixture mounted");
    assert!(!fx.is_tree_slot(), "still the editor slot");
    assert!(fx.editor_display_text().is_empty());
}

#[then("思考折叠默认态翻转且全帧不出现模型列表")]
fn then_thinking_flipped_no_models_panel(tui_interaction: &TuiInteraction) {
    let (frame, expanded) = {
        let mut fx = tui_interaction.fx.borrow_mut();
        let fx = fx.as_mut().expect("fixture mounted");
        let frame = fx.render_plain(80);
        (frame, fx.fold().thinking_expanded)
    };
    assert!(expanded, "Ctrl+T flips the thinking fold default");
    assert!(
        !frame.to_lowercase().contains("model"),
        "no model picker panel may appear: {frame}"
    );
}
