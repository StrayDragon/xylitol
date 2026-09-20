//! Steps for `app-tui-fixed-zone` — 下缘待办栏与队列条 / 通知条 / status / editor 堆叠。

use crate::app::tui::{InteractionBdd, UiModel};
use crate::protocol::session::{TodoItem, TodoList, TodoStatus};
use crate::tests::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};

pub struct FixedZoneBdd {
    pub model: RefCell<UiModel>,
    pub frame: RefCell<Option<String>>,
}

#[fixture]
pub fn fixed_zone_bdd() -> FixedZoneBdd {
    FixedZoneBdd {
        model: RefCell::new(UiModel::new()),
        frame: RefCell::new(None),
    }
}

fn sample_todo() -> TodoList {
    TodoList::new(vec![
        TodoItem {
            id: "a".into(),
            content: "read glossary".into(),
            status: TodoStatus::Completed,
        },
        TodoItem {
            id: "b".into(),
            content: "write todo-bar copy".into(),
            status: TodoStatus::InProgress,
        },
        TodoItem {
            id: "c".into(),
            content: "paint lab states".into(),
            status: TodoStatus::Pending,
        },
    ])
}

fn paint_dock(model: UiModel) -> String {
    let mut fx = InteractionBdd::from_model(model);
    fx.push_toast_notice("boom-toast");
    fx.render_plain(80)
}

#[when("产品 TUI 同时存在非空待办栏、队列条与通知条")]
fn when_todo_queue_toast_present(fixed_zone_bdd: &FixedZoneBdd) {
    let mut model = UiModel::new();
    model.todo = sample_todo();
    model.enqueue_steer_strip("steer-me-queue".into());
    model.set_busy_status("Working");
    *fixed_zone_bdd.frame.borrow_mut() = Some(paint_dock(model.clone()));
    *fixed_zone_bdd.model.borrow_mut() = model;
}

#[then("从上到下 MUST 为队列条、待办栏、通知条、status、editor")]
fn then_dock_order_queue_todo_toast_status_editor(fixed_zone_bdd: &FixedZoneBdd) {
    let plain = fixed_zone_bdd
        .frame
        .borrow()
        .clone()
        .expect("painted frame");
    let idx = |needle: &str| {
        plain
            .find(needle)
            .unwrap_or_else(|| panic!("missing `{needle}` in:\n{plain}"))
    };
    let queue_at = idx("Steering: steer-me-queue");
    let todo_at = idx("write todo-bar copy");
    let toast_at = idx("Error: boom-toast");
    let status_at = idx("Working");
    assert!(
        queue_at < todo_at && todo_at < toast_at && toast_at < status_at,
        "dock order queue → 待办栏 → toast → status:\n{plain}"
    );
    let after_status = &plain[status_at..];
    assert!(
        after_status.contains('─'),
        "editor operation zone MUST sit below status:\n{plain}"
    );
}

#[then("空表时待办栏 MUST 占 0 行")]
fn then_empty_todo_bar_is_zero_rows(fixed_zone_bdd: &FixedZoneBdd) {
    let mut model = fixed_zone_bdd.model.borrow().clone();
    model.todo = TodoList::default();
    let plain = paint_dock(model);
    assert!(
        plain.contains("Steering: steer-me-queue"),
        "queue remains: {plain}"
    );
    assert!(
        plain.contains("Error: boom-toast"),
        "toast remains: {plain}"
    );
    assert!(
        !plain.contains("write todo-bar copy")
            && !plain.contains("completed")
            && !plain.contains("pending"),
        "empty 待办栏 MUST occupy 0 rows: {plain}"
    );
}
