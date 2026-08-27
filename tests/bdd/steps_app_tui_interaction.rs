//! Steps for `app-tui-transcript` P3 — headless keyboard/mouse interaction
//! over the product `UiRoot` via [`InteractionBdd`] (att20–att32).
//!
//! Paths exercised are the product's own: chord routing (`UiRoot::handle_key`),
//! the shared fold-toggle pipeline (`UiRoot::click_fold_at`, also called by the
//! host hit-priority wiring), and product paint for frames.

use crate::prelude::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol::app::tui::{FoldTarget, InteractionBdd, SceneBuilder};

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
