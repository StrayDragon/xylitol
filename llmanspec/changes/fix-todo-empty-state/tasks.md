# Tasks — fix-todo-empty-state

垂直切片；每个 task 独立可验证（编译 + 自带测试绿）。seam 全部复用既有：
preview 纯函数单测（`preview.rs` tests）+ transcript BDD（`#[scenario]` 绑定 live
spec，steps 在 `tests/bdd/steps_app_tui_transcript.rs`，跑
`cargo test --lib --all-features tests::bdd::`），不发明新 seam。

## T1 att13：todo_* 摘要空串（去 `...` 占位）

- [ ] `src/app/tui/bridge/preview.rs`：`human_tool_args_preview_with_path` todo 分支
      无 `items` 可解析 → `String::new()`（原 `PATH_PLACEHOLDER`）；有 `items`
      （含空数组 → `0 items`）行为不变
- [ ] 单测：`todo_args_preview_is_item_count_not_json` 改断言——`todo_list({})` → ""、
      `todo_update({id,status})` → ""、rewrite 流式缺 `items` → ""、空数组 → `0 items`
- [ ] BDD：live spec 新 `@req:att13 @executable` 场景
      `todo-summary-empty-not-placeholder` + when/then steps + `bindings_app_tui_transcript`
      绑定；既有 `tool-human-summary-location-only`（路径族 `...` 占位）不动
- [ ] 验证：`cargo test --lib --all-features tests::bdd::` att13 相关 + preview 单测

## T2 att36：空列表 body 显式空态提示

- [ ] `src/app/tui/bridge/preview.rs`：`humanize_tool_result_for_tui` todo 分支空列表
      → `Some("(empty list)")`（固定英文词；is_error 仍原样）；非空清单渲染不变
- [ ] 单测：`humanize_todo_result_is_glyph_checklist` 空表断言 `Some("")` →
      `Some("(empty list)")`；补 `output_looks_like_machine_json` 不误判
- [ ] BDD：live spec 新 `@req:att36 @executable` 场景
      `todo-block-empty-list-body-hint`（todo_list 空结果，live 无 `TodoUpdated`、
      rebuild 无 agent_todo snapshot）+ 空表 fixture + when/then steps（直播 vs travel
      重建两路 body 逐行一致且为空态提示行）+ 绑定
- [ ] 验证：同上 att36 相关；`bridge/tests.rs` 既有 todo 用例回归
- [blocked-by: T1]

## T3 门禁收口

- [ ] `just fmt` / `just lint` / `just test` / `just test-tui` / `just qa` 全绿
- [blocked-by: T1, T2]
