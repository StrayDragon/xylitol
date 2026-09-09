//! Steps for `package-tui-tree-selector` — 包级组件直驱（pts1–pts14）。
//!
//! 驱动路径与包内单测同一：构造 `TreeSelector`，键经 `Component::handle_input`
//! 进 `tui.select.*` / `tui.tree.*` 键表；渲染走 `Component::render`。
//! 回调（on_select / on_cancel / on_label_edit）写入 thread-local 捕获，
//! 供 then 步骤断言（选择器由夹具独占持有，无跨步借用）。

use crate::tests::bdd::prelude::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol_tui::keybindings::{
    KeybindingsConfig, KeybindingsManager, KeybindingsScope, create_default_definitions,
};
use xylitol_tui::utils::strip_ansi_codes;
use xylitol_tui::{
    Component, InputEvent, TreeNode, TreeSelector, TreeSelectorOptions, TreeSelectorTheme,
};

/// Shared state: the selector plus the last rendered plain frame.
pub struct TreeSelBdd {
    pub sel: RefCell<Option<TreeSelector>>,
    pub last_frame: RefCell<String>,
    /// pts5：场景级翻页键作用域（夹具销毁时自动还原默认键表）。
    pub paging_scope: RefCell<Option<KeybindingsScope>>,
}

#[fixture]
pub fn tree_sel_bdd() -> TreeSelBdd {
    TreeSelBdd {
        sel: RefCell::new(None),
        last_frame: RefCell::new(String::new()),
        paging_scope: RefCell::new(None),
    }
}

thread_local! {
    static SELECT_CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static CANCEL_CALLS: Cell<usize> = const { Cell::new(0) };
    static LABEL_EDITS: RefCell<Vec<(String, Option<String>)>> = const { RefCell::new(Vec::new()) };
}

fn reset_captures() {
    SELECT_CALLS.with(|s| s.borrow_mut().clear());
    CANCEL_CALLS.take();
    LABEL_EDITS.with(|s| s.borrow_mut().clear());
}

/// 样例树：root → parent branch → (child leaf / second leaf)。
fn sample_tree() -> Vec<TreeNode> {
    vec![
        TreeNode::new("r", "root").with_child(
            TreeNode::new("p1", "parent branch")
                .with_child(TreeNode::new("c1", "child leaf"))
                .with_child(TreeNode::new("c2", "second leaf")),
        ),
    ]
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> InputEvent {
    InputEvent::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn key_char(ch: char, modifiers: KeyModifiers) -> InputEvent {
    key(KeyCode::Char(ch), modifiers)
}

fn with_sel<T>(bdd: &TreeSelBdd, f: impl FnOnce(&mut TreeSelector) -> T) -> T {
    let mut guard = bdd.sel.borrow_mut();
    let sel = guard.as_mut().expect("TreeSelector mounted");
    f(sel)
}

fn render_into(bdd: &TreeSelBdd, width: usize) -> String {
    let text = with_sel(bdd, |sel| strip_ansi_codes(&sel.render(width).join("\n")));
    *bdd.last_frame.borrow_mut() = text.clone();
    text
}

fn frame_of(bdd: &TreeSelBdd) -> String {
    bdd.last_frame.borrow().clone()
}

fn selected_id(bdd: &TreeSelBdd) -> Option<String> {
    with_sel(bdd, |sel| sel.selected_id().map(str::to_string))
}

// ── mounts ────────────────────────────────────────────────────

fn mount(bdd: &TreeSelBdd, roots: Vec<TreeNode>, options: TreeSelectorOptions) {
    reset_captures();
    let mut sel = TreeSelector::new(roots, TreeSelectorTheme::default(), options);
    sel.on_select = Some(Box::new(|id: String| {
        SELECT_CALLS.with(|s| s.borrow_mut().push(id));
    }));
    sel.on_cancel = Some(Box::new(|| {
        CANCEL_CALLS.with(|c| c.set(c.get() + 1));
    }));
    sel.on_label_edit = Some(Box::new(|id: String, current: Option<String>| {
        LABEL_EDITS.with(|s| s.borrow_mut().push((id, current)));
    }));
    *bdd.sel.borrow_mut() = Some(sel);
}

fn leaf_only_predicate() -> xylitol_tui::TreeNodePredicate {
    Box::new(|n: &TreeNode| n.children.is_empty())
}

// pts1 / pts2 / pts3 / pts4 …共用的「样例树挂载」入口
#[when("以样例树挂载 TreeSelector")]
fn when_mount_sample(tree_sel_bdd: &TreeSelBdd) {
    mount(tree_sel_bdd, sample_tree(), TreeSelectorOptions::default());
    render_into(tree_sel_bdd, 80);
}

// pts2
#[when("以样例树挂载 TreeSelector 并设置只含叶子的过滤")]
fn when_mount_leaf_filter(tree_sel_bdd: &TreeSelBdd) {
    mount(tree_sel_bdd, sample_tree(), TreeSelectorOptions::default());
    with_sel(tree_sel_bdd, |sel| {
        sel.set_include_node(Some(leaf_only_predicate()));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("可见列表仅剩叶子且行完整重算")]
fn then_leaf_only_visible(tree_sel_bdd: &TreeSelBdd) {
    let ids = with_sel(tree_sel_bdd, |sel| {
        sel.filtered_ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        ids,
        vec!["c1".to_string(), "c2".to_string()],
        "pts2: leaf-only filter"
    );
    let frame = frame_of(tree_sel_bdd);
    assert!(
        frame.contains("child leaf") && !frame.contains("parent branch"),
        "pts2: rebuilt rows must drop filtered-out ancestors:\n{frame}"
    );
}

#[when("设置放行全部的过滤")]
fn when_filter_all(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| sel.set_include_node(None));
    render_into(tree_sel_bdd, 80);
}

#[then("树行完整恢复")]
fn then_full_tree_restored(tree_sel_bdd: &TreeSelBdd) {
    let frame = frame_of(tree_sel_bdd);
    for label in ["root", "parent branch", "child leaf", "second leaf"] {
        assert!(frame.contains(label), "pts2: `{label}` restored:\n{frame}");
    }
}

// pts1
#[then("根与子节点行可见且选中态落在首项")]
fn then_rows_and_first_selected(tree_sel_bdd: &TreeSelBdd) {
    let frame = frame_of(tree_sel_bdd);
    for label in ["root", "parent branch", "child leaf"] {
        assert!(frame.contains(label), "pts1: `{label}` visible:\n{frame}");
    }
    assert_eq!(
        selected_id(tree_sel_bdd).as_deref(),
        Some("r"),
        "pts1: selection lands on the first visible row"
    );
}

// pts3
#[when("按下 select down 再按下 select up")]
fn when_down_then_up(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::Down, KeyModifiers::NONE));
        sel.handle_input(key(KeyCode::Up, KeyModifiers::NONE));
    });
}

#[then("选中态回到首项")]
fn then_selection_first(tree_sel_bdd: &TreeSelBdd) {
    assert_eq!(selected_id(tree_sel_bdd).as_deref(), Some("r"));
}

#[when("按下 select confirm")]
fn when_confirm(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::Enter, KeyModifiers::NONE));
    });
}

#[then("on_select 回调收到该节点 id")]
fn then_select_fired(tree_sel_bdd: &TreeSelBdd) {
    let expected = selected_id(tree_sel_bdd);
    let calls = SELECT_CALLS.with(|s| s.borrow().clone());
    assert_eq!(
        calls,
        vec![expected.expect("a selected id")],
        "pts3: confirm must fire on_select with the selected id"
    );
}

#[when("按下 select cancel")]
fn when_cancel(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::Esc, KeyModifiers::NONE));
    });
}

#[then("on_cancel 回调触发")]
fn then_cancel_fired(_tree_sel_bdd: &TreeSelBdd) {
    assert_eq!(
        CANCEL_CALLS.get(),
        1,
        "pts3: cancel with empty search fires on_cancel"
    );
}

#[then("选中行经主题闭包反色渲染")]
fn then_selected_row_styled(tree_sel_bdd: &TreeSelBdd) {
    let raw = with_sel(tree_sel_bdd, |sel| sel.render(80).join("\n"));
    let selected_line = raw
        .lines()
        .find(|l| l.contains("root"))
        .expect("selected row");
    assert!(
        selected_line.contains("\x1b[7m"),
        "pts3: theme selected_row (reverse) must style the selected row: {selected_line:?}"
    );
}

// pts4
#[when("以样例树挂载 TreeSelector 并输入搜索串 \"child\"")]
fn when_mount_and_search_child(tree_sel_bdd: &TreeSelBdd) {
    mount(tree_sel_bdd, sample_tree(), TreeSelectorOptions::default());
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key_char('c', KeyModifiers::NONE));
        sel.handle_input(key_char('h', KeyModifiers::NONE));
        sel.handle_input(key_char('i', KeyModifiers::NONE));
        sel.handle_input(key_char('l', KeyModifiers::NONE));
        sel.handle_input(key_char('d', KeyModifiers::NONE));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("可见列表收窄到匹配项")]
fn then_search_narrows(tree_sel_bdd: &TreeSelBdd) {
    let ids = with_sel(tree_sel_bdd, |sel| {
        sel.filtered_ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    assert_eq!(ids, vec!["c1".to_string()], "pts4: search narrows to match");
    assert_eq!(
        with_sel(tree_sel_bdd, |sel| sel.search_query().to_string()),
        "child"
    );
}

#[when("退格删除一个字符")]
fn when_backspace(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::Backspace, KeyModifiers::NONE));
    });
}

#[then("搜索串变为 \"chil\"")]
fn then_search_backspaced(tree_sel_bdd: &TreeSelBdd) {
    assert_eq!(
        with_sel(tree_sel_bdd, |sel| sel.search_query().to_string()),
        "chil",
        "pts4: backspace pops the last search char"
    );
}

#[when("按下 select cancel 且搜索串非空")]
fn when_cancel_with_search(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        assert!(!sel.search_query().is_empty());
        sel.handle_input(key(KeyCode::Esc, KeyModifiers::NONE));
    });
}

#[then("搜索被清空且取消回调未触发")]
fn then_search_cleared_not_cancelled(tree_sel_bdd: &TreeSelBdd) {
    assert_eq!(
        with_sel(tree_sel_bdd, |sel| sel.search_query().to_string()),
        "",
        "pts4: Esc clears the search first"
    );
    assert_eq!(
        CANCEL_CALLS.get(),
        0,
        "pts4: Esc with a live search MUST NOT cancel"
    );
}

#[when("设置只含叶子的过滤并搜索 \"second\"")]
fn when_filter_then_search_second(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.set_include_node(Some(leaf_only_predicate()));
        sel.handle_input(key_char('s', KeyModifiers::NONE));
        sel.handle_input(key_char('e', KeyModifiers::NONE));
        sel.handle_input(key_char('c', KeyModifiers::NONE));
        sel.handle_input(key_char('o', KeyModifiers::NONE));
        sel.handle_input(key_char('n', KeyModifiers::NONE));
        sel.handle_input(key_char('d', KeyModifiers::NONE));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("搜索与过滤按 AND 组合")]
fn then_search_and_filter_and(tree_sel_bdd: &TreeSelBdd) {
    let ids = with_sel(tree_sel_bdd, |sel| {
        sel.filtered_ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    assert_eq!(ids, vec!["c2".to_string()], "pts4: search AND include_node");
}

// pts5
fn thirty_leaves() -> Vec<TreeNode> {
    let leaves: Vec<TreeNode> = (0..30)
        .map(|i| TreeNode::new(format!("l{i}"), format!("leaf {i:02}")))
        .collect();
    vec![TreeNode::new("root", "root").with_children(leaves)]
}

#[when("以 30 叶树挂载并作用域绑定翻页键")]
fn when_mount_paging(tree_sel_bdd: &TreeSelBdd) {
    let mut user = KeybindingsConfig::new();
    user.insert("tui.select.pageDown".into(), vec!["pageDown".into()]);
    user.insert("tui.select.pageUp".into(), vec!["pageUp".into()]);
    let kb = std::rc::Rc::new(std::cell::RefCell::new(KeybindingsManager::new(
        create_default_definitions(),
        user,
    )));
    // 场景级作用域：随夹具销毁自动还原默认键表（RAII）。
    let scope = KeybindingsScope::enter(kb);
    *tree_sel_bdd.paging_scope.borrow_mut() = Some(scope);
    mount(
        tree_sel_bdd,
        thirty_leaves(),
        TreeSelectorOptions::default(),
    );
    render_into(tree_sel_bdd, 80);
}

#[when("按下 select pageDown")]
fn when_page_down(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::PageDown, KeyModifiers::NONE));
    });
}

#[then("选中前进 max_visible")]
fn then_paged_forward(tree_sel_bdd: &TreeSelBdd) {
    let (idx, max_visible) = with_sel(tree_sel_bdd, |sel| {
        let id = sel.selected_id().expect("selected").to_string();
        let idx = sel
            .filtered_ids()
            .iter()
            .position(|&s| s == id)
            .expect("selected in filtered");
        (idx, 12usize)
    });
    assert_eq!(idx, max_visible, "pts5: pageDown moves by max_visible");
}

#[when("按下 select pageUp")]
fn when_page_up(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::PageUp, KeyModifiers::NONE));
    });
}

#[then("选中回到首项")]
fn then_paged_back(tree_sel_bdd: &TreeSelBdd) {
    assert_eq!(selected_id(tree_sel_bdd).as_deref(), Some("root"));
}

#[when("默认键表下按左右方向键")]
fn when_default_left_right(tree_sel_bdd: &TreeSelBdd) {
    let before = selected_id(tree_sel_bdd);
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::Right, KeyModifiers::NONE));
        sel.handle_input(key(KeyCode::Left, KeyModifiers::NONE));
    });
    assert_eq!(
        selected_id(tree_sel_bdd),
        before,
        "pts5: ←→ unbound by default (reserved for product)"
    );
}

#[then("选中保持不变")]
fn then_selection_unchanged(tree_sel_bdd: &TreeSelBdd) {
    let _ = tree_sel_bdd; // 断言在上一步骤内完成，占位保持步骤同组
}

// pts6
#[when("以样例树挂载并设置状态后缀 \"[no-tools]\"")]
fn when_mount_with_suffix(tree_sel_bdd: &TreeSelBdd) {
    mount(
        tree_sel_bdd,
        sample_tree(),
        TreeSelectorOptions {
            status_suffix: Some("[no-tools]".into()),
            ..TreeSelectorOptions::default()
        },
    );
    render_into(tree_sel_bdd, 80);
}

#[then("状态行在 (i/n) 后附加该后缀")]
fn then_suffix_after_counter(tree_sel_bdd: &TreeSelBdd) {
    let frame = frame_of(tree_sel_bdd);
    let counter_line = frame
        .lines()
        .find(|l| l.contains('(') && l.contains('/'))
        .expect("a status line");
    assert!(
        counter_line.contains("(1/4)") && counter_line.contains("[no-tools]"),
        "pts6: suffix appended after (i/n): {counter_line:?}"
    );
}

// pts7
#[when("以样例树挂载并把选中移到可折叠父节点")]
fn when_mount_select_parent(tree_sel_bdd: &TreeSelBdd) {
    // is_foldable 要求父层有 >1 个可见子节点（分支段语义）。
    let tree = vec![
        TreeNode::new("r", "root")
            .with_child(
                TreeNode::new("p1", "parent branch")
                    .with_child(TreeNode::new("c1", "child leaf"))
                    .with_child(TreeNode::new("c2", "second leaf")),
            )
            .with_child(TreeNode::new("p2", "sibling branch")),
    ];
    mount(tree_sel_bdd, tree, TreeSelectorOptions::default());
    with_sel(tree_sel_bdd, |sel| {
        assert!(sel.select_id("p1"), "parent must be selectable");
    });
}

#[when("按下 tree foldOrUp")]
fn when_fold_or_up(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::Left, KeyModifiers::CONTROL));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("该节点子树折叠且渲染出现折叠标记")]
fn then_folded_with_marker(tree_sel_bdd: &TreeSelBdd) {
    assert!(
        with_sel(tree_sel_bdd, |sel| sel.is_folded("p1")),
        "pts7: foldOrUp folds the foldable node"
    );
    let ids = with_sel(tree_sel_bdd, |sel| {
        sel.filtered_ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    assert!(!ids.contains(&"c1".to_string()), "children hidden: {ids:?}");
    assert!(frame_of(tree_sel_bdd).contains('⊞'), "fold marker visible");
}

#[when("按下 tree unfoldOrDown")]
fn when_unfold_or_down(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key(KeyCode::Right, KeyModifiers::CONTROL));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("子树重新展开")]
fn then_unfolded(tree_sel_bdd: &TreeSelBdd) {
    assert!(
        !with_sel(tree_sel_bdd, |sel| sel.is_folded("p1")),
        "pts7: unfoldOrDown unfolds"
    );
    let ids = with_sel(tree_sel_bdd, |sel| {
        sel.filtered_ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    assert!(ids.contains(&"c1".to_string()));
}

#[when("输入搜索字符")]
fn when_type_search_char(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key_char('p', KeyModifiers::NONE));
    });
}

#[then("折叠态被清空")]
fn then_folds_cleared(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        assert!(!sel.is_folded("p1"), "pts7: search change clears folds");
        sel.set_search_query("");
    });
}

// pts8
#[when("以带注解与时间戳的样例树挂载")]
fn when_mount_annotated(tree_sel_bdd: &TreeSelBdd) {
    let mut roots = sample_tree();
    roots[0].children[0].annotation = Some("pinned".into());
    roots[0].children[0].annotation_at = Some("12:00".into());
    mount(tree_sel_bdd, roots, TreeSelectorOptions::default());
    render_into(tree_sel_bdd, 80);
}

#[then("注解以括号形式先于主标签渲染")]
fn then_annotation_renders(tree_sel_bdd: &TreeSelBdd) {
    let frame = frame_of(tree_sel_bdd);
    let line = frame
        .lines()
        .find(|l| l.contains("pinned"))
        .expect("annotated row");
    let ann = line.find("[pinned]").expect("bracketed annotation");
    let label = line.find("parent branch").expect("label");
    assert!(ann < label, "pts8: annotation precedes label: {line:?}");
    assert!(!frame.contains("12:00"), "timestamps hidden until enabled");
}

#[when("开启时间戳显示")]
fn when_show_timestamps(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| sel.set_show_annotation_timestamps(true));
    render_into(tree_sel_bdd, 80);
}

#[then("注解时间戳随之出现")]
fn then_timestamp_visible(tree_sel_bdd: &TreeSelBdd) {
    assert!(
        frame_of(tree_sel_bdd).contains("12:00"),
        "pts8: annotation_at shown when enabled"
    );
}

// pts9
#[when("以带注解的样例树挂载并选中该节点")]
fn when_mount_annotated_select(tree_sel_bdd: &TreeSelBdd) {
    let mut roots = sample_tree();
    roots[0].children[0].annotation = Some("pinned".into());
    mount(tree_sel_bdd, roots, TreeSelectorOptions::default());
    with_sel(tree_sel_bdd, |sel| {
        assert!(sel.select_id("p1"));
    });
}

#[when("按下 tree editLabel")]
fn when_edit_label(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key_char('l', KeyModifiers::SHIFT));
    });
}

#[then("on_label_edit 收到 id 与当前注解")]
fn then_label_edit_fired(_tree_sel_bdd: &TreeSelBdd) {
    let edits = LABEL_EDITS.with(|s| s.borrow().clone());
    assert_eq!(
        edits,
        vec![("p1".to_string(), Some("pinned".to_string()))],
        "pts9: editLabel hands (id, current_annotation) to the host"
    );
}

#[when("按下 tree toggleLabelTimestamp")]
fn when_toggle_ts(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key_char('t', KeyModifiers::SHIFT));
    });
}

#[then("时间戳显示翻转")]
fn then_timestamp_toggled(tree_sel_bdd: &TreeSelBdd) {
    assert!(
        with_sel(tree_sel_bdd, |sel| sel.show_annotation_timestamps()),
        "pts9: Shift+T toggles timestamp display on"
    );
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key_char('t', KeyModifiers::SHIFT));
    });
    assert!(
        !with_sel(tree_sel_bdd, |sel| sel.show_annotation_timestamps()),
        "pts9: and back off"
    );
}

// pts10
#[when("以超长标签树挂载并按窄宽渲染")]
fn when_mount_long_label_narrow(tree_sel_bdd: &TreeSelBdd) {
    // 深层缩进让 anchor 列溢出窄宽——这才是平移触发的真实几何。
    let deep = TreeNode::new("r", "root").with_child(TreeNode::new("a", "A").with_child(
        TreeNode::new("b", "B").with_child(TreeNode::new("c", "C").with_child(TreeNode::new(
            "leaf",
            "UNIQUE_TAIL_MARKER_xyz_should_remain_visible_when_selected",
        ))),
    ));
    mount(
        tree_sel_bdd,
        vec![deep],
        TreeSelectorOptions {
            active_id: Some("leaf".into()),
            max_visible: 8,
            ..TreeSelectorOptions::default()
        },
    );
    render_into(tree_sel_bdd, 28);
}

#[then("选中行保留光标 gutter 且标签尾部内容仍可见")]
fn then_pan_keeps_gutter_and_tail(tree_sel_bdd: &TreeSelBdd) {
    let frame = frame_of(tree_sel_bdd);
    let selected = frame
        .lines()
        .find(|l| l.contains("UNIQUE_TAIL"))
        .expect("pts10: tail must stay visible under horizontal pan");
    assert!(
        selected.starts_with("› ") || selected.starts_with('›'),
        "pts10: cursor gutter stays fixed: {selected:?}"
    );
}

// pts11
#[when("以样例树挂载并过滤到空集")]
fn when_mount_empty_filter(tree_sel_bdd: &TreeSelBdd) {
    mount(tree_sel_bdd, sample_tree(), TreeSelectorOptions::default());
    with_sel(tree_sel_bdd, |sel| {
        sel.set_include_node(Some(Box::new(|_: &TreeNode| false)));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("渲染出现可辨识空态行且计数为 (0/0)")]
fn then_empty_state_row(tree_sel_bdd: &TreeSelBdd) {
    let frame = frame_of(tree_sel_bdd);
    assert!(
        frame.contains("No entries found") && frame.contains("(0/0)"),
        "pts11: recognisable empty state:\n{frame}"
    );
}

// pts12
#[when("以样例树挂载并选中仍可见的节点后变更过滤")]
fn when_mount_select_then_refilter(tree_sel_bdd: &TreeSelBdd) {
    mount(tree_sel_bdd, sample_tree(), TreeSelectorOptions::default());
    with_sel(tree_sel_bdd, |sel| {
        assert!(sel.select_id("c2"));
        sel.set_include_node(Some(leaf_only_predicate()));
    });
}

#[then("选中保持该节点")]
fn then_selection_kept(tree_sel_bdd: &TreeSelBdd) {
    assert_eq!(
        selected_id(tree_sel_bdd).as_deref(),
        Some("c2"),
        "pts12: selection survives a filter that keeps it"
    );
}

#[when("变更为排除该节点的过滤")]
fn when_filter_excluding_c2(tree_sel_bdd: &TreeSelBdd) {
    with_sel(tree_sel_bdd, |sel| {
        sel.set_include_node(Some(Box::new(|n: &TreeNode| n.id != "c2")));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("选中落到可见首项")]
fn then_selection_falls_to_first(tree_sel_bdd: &TreeSelBdd) {
    assert_eq!(
        selected_id(tree_sel_bdd).as_deref(),
        Some("r"),
        "pts12: falls back to the first visible row"
    );
}

// pts13
fn kind_tree(kind: Option<&str>) -> Vec<TreeNode> {
    let mut node = TreeNode::new("n1", "alpha");
    node.kind = kind.map(str::to_string);
    vec![node]
}

#[when("以带 kind 的样例树挂载")]
fn when_mount_kind(tree_sel_bdd: &TreeSelBdd) {
    mount(
        tree_sel_bdd,
        kind_tree(Some("user")),
        TreeSelectorOptions::default(),
    );
    render_into(tree_sel_bdd, 80);
}

#[then("kind 前缀经主题闭包渲染在主标签前")]
fn then_kind_prefix_renders(tree_sel_bdd: &TreeSelBdd) {
    let raw = with_sel(tree_sel_bdd, |sel| sel.render(80).join("\n"));
    let line = raw.lines().find(|l| l.contains("alpha")).expect("row");
    let kind = line.find("user: ").expect("kind prefix");
    let label = line.find("alpha").expect("label");
    assert!(kind < label, "pts13: kind prefix precedes label: {line:?}");
    assert!(
        line.contains("\x1b[35m"),
        "pts13: theme kind_prefix colors the tag: {line:?}"
    );
}

#[when("以无 kind 的样例树挂载")]
fn when_mount_no_kind(tree_sel_bdd: &TreeSelBdd) {
    mount(
        tree_sel_bdd,
        kind_tree(None),
        TreeSelectorOptions::default(),
    );
    render_into(tree_sel_bdd, 80);
}

#[then("渲染不强制 kind 前缀")]
fn then_no_kind_prefix(tree_sel_bdd: &TreeSelBdd) {
    let frame = frame_of(tree_sel_bdd);
    assert!(
        !frame.contains("user: ") && !frame.contains("["),
        "pts13: no kind prefix without a kind: {frame}"
    );
}

// pts14
#[when("以带 kind 的样例树挂载并搜索 kind 词")]
fn when_mount_kind_search(tree_sel_bdd: &TreeSelBdd) {
    mount(
        tree_sel_bdd,
        kind_tree(Some("user")),
        TreeSelectorOptions::default(),
    );
    with_sel(tree_sel_bdd, |sel| {
        sel.handle_input(key_char('u', KeyModifiers::NONE));
        sel.handle_input(key_char('s', KeyModifiers::NONE));
        sel.handle_input(key_char('e', KeyModifiers::NONE));
        sel.handle_input(key_char('r', KeyModifiers::NONE));
    });
    render_into(tree_sel_bdd, 80);
}

#[then("该节点经 kind 匹配保持可见")]
fn then_kind_search_matches(tree_sel_bdd: &TreeSelBdd) {
    let ids = with_sel(tree_sel_bdd, |sel| {
        sel.filtered_ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        ids,
        vec!["n1".to_string()],
        "pts14: kind participates in the search match"
    );
}
