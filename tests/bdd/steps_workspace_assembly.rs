use std::cell::RefCell;
use std::process::Command;

use rstest_bdd_macros::{then, when};
use xylitol_official_plugin_hooks::{
    DispatchResult, HookDispatcher, HookEvent, HookPhase, HooksConfig,
};

thread_local! {
    static EMPTY_HOOKS: RefCell<Option<(bool, DispatchResult)>> = const { RefCell::new(None) };
    static CRATE_GRAPH_OK: RefCell<Option<bool>> = const { RefCell::new(None) };
}

#[when("扫描工作区各包对 rutis 的 Cargo 依赖")]
pub(crate) fn w_scan_workspace_rutis_deps() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let status = Command::new(crate::bdd::helpers::python3())
        .arg("scripts/check_crate_graph.py")
        .current_dir(root)
        .status()
        .expect("run check_crate_graph.py");
    CRATE_GRAPH_OK.with(|slot| *slot.borrow_mut() = Some(status.success()));
}

#[then("仅装配适配层声明了 rutis")]
pub(crate) fn t_only_adapter_declares_rutis() {
    let ok = CRATE_GRAPH_OK.with(|slot| slot.borrow().expect("已扫描 Cargo 依赖"));
    assert!(ok, "crate 图门禁必须通过：仅装配适配层可依赖 rutis");
}

#[when("以空 hooks 配置分发工具调用")]
pub(crate) async fn w_dispatch_with_empty_hooks() {
    let dispatcher = HookDispatcher::new(&HooksConfig::default());
    let empty = dispatcher.is_empty();
    let result = dispatcher
        .dispatch(
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({"command": "true"}),
            },
            HookPhase::Pre,
        )
        .await;
    EMPTY_HOOKS.with(|slot| *slot.borrow_mut() = Some((empty, result)));
}

#[then("分发器为空且结果为允许")]
pub(crate) fn t_empty_hooks_allowed() {
    let (empty, result) = EMPTY_HOOKS.with(|slot| slot.borrow().clone().expect("已分发"));
    assert!(empty, "空 hooks 配置必须得到空分发器");
    assert!(
        matches!(result, DispatchResult::Allowed),
        "空 hooks 分发必须允许，得到 {result:?}"
    );
}
