# Tasks — refine-todo-display-planes

垂直切片；每个 task 独立可验证（编译 + 自带测试绿）。seam 全部复用既有（见
design.md「测试 seam」），不发明新 seam。

## T1 协议面：`XyEvent::TodoUpdated` 变体 + 线协议表达

- `src/protocol/lifecycle.rs`：新增 `TodoUpdated { list: TodoList }` 变体。
- `protocol::Event` 映射该变体（远程 attach 路径可表达；未映射端降级语义不变）。
- `remote.rs` `is_cold_replay_tape`：`TodoUpdated` 不归 tape（D3）。
- 测试：serde 往返 / camelCase tag / 描述键稳定性单测；protocol::Event 映射单测。
- 验证：`cargo test -p xylitol` 相关模块 + 编译器穷举检查扫全 match。

## T2 host 发射：gateway SSOT 变更点发布（D1）

- `SessionAgentTodoGateway` 增可选事件发布器（unbound 静默）；rewrite / update 成功
  persist 后发布 `TodoUpdated { list }`（list 为空也发布，语义为清除）。
- composition root（app 组装处）接线发布器到 driver 事件流。
- 测试：`src/infra/tools/todo.rs` 既有测试基座补发射断言；unbound 路径不 panic。
- 验证：同上。
- [blocked-by: T1]

## T3 client 展示：typed live 同步 + todo 块人话化

- bridge `handlers/tools.rs`：处理 `TodoUpdated` → `sync_todo_checklist`（typed）；
  删除 `sync_todo_checklist_from_tool_result` 字符串解析路径与 End 里的
  `is_todo_tool` 分支。
- `preview.rs`：`human_tool_args_preview_with_path` 补 todo_* 分支（条目计数摘要，
  D4）；`humanize_tool_result_for_tui` 补 todo_* 分支（勾选清单 body，is_error 不
  人话化）；`output_looks_like_machine_json` 补 items 形态；字形表收口单一 helper
  （checklist 行与块 body 共用）。
- 测试：`src/app/tui/bridge/tests.rs` + `preview.rs` 内嵌测试补 live 用例
  （Start 空摘要 → End 清单 body → TodoUpdated 刷新 checklist）。
- 验证：`cargo test -p xylitol`；手跑 TUI 目检（`cargo run` + 真实 todo 会话）。
- [blocked-by: T1]

## T4 重建同构验证

- `session_tree.rs` rebuild 路径仍走 SSOT 快照 + humanize helper；断言直播 flush 后
  与 travel 重建的 Todo 块 / checklist 形态同构（att50 幂等语义延伸到 todo 块 body）。
- 测试：`session_tree.rs` 既有测试扩用例；transcript BDD 场景经 `tests/features`
  既有 step 落 executable（挂 @req:att36）。
- 验证：`cargo test --lib --all-features tests::bdd::` 相关 feature。
- [blocked-by: T3]

## T5 docs + designing

- `docs/architecture/` 新增「信息平面总览」：五面真值源 / 读写权 / 流向，todo 作
  worked example；`docs/AGENTS.md` 文档闭环遵守。
- `designing/tui/modules/tool/`：补 todo_* 块形态（draft.yaml + intent.md 可观察
  MUST + states 固定态）；如 checklist 呈现有变体，同步相应模块；跑
  `just gen-designing-index` + `just export-design-frame`。
- 验证：`just doc-check` + `scripts/check_tui_designing`（随 qa）。
- [blocked-by: T3]

## T6 门禁收口

- `just fmt` / `just lint` / `just test` / `just test-tui` / `just qa` 全绿；
  `just qa-e2e` 视 TUI 交互改动需要而跑（PTY）。
- [blocked-by: T2, T4, T5]
