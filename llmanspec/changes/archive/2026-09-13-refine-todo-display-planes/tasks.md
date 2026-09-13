# Tasks — refine-todo-display-planes

垂直切片；每个 task 独立可验证（编译 + 自带测试绿）。seam 全部复用既有（见
design.md「测试 seam」），不发明新 seam。

## T1 协议面：`XyEvent::TodoUpdated` 变体 + 线协议表达

- [x] `src/protocol/lifecycle.rs`：新增 `TodoUpdated { list: TodoList }` 变体
- [x] `protocol::Event` 映射该变体（远程 attach 可表达；未映射端降级语义不变）
- [x] `remote.rs` `is_cold_replay_tape`：`TodoUpdated` 不归 tape（D3）
- [x] 测试：serde 往返 / camelCase tag / protocol::Event 映射单测
- [x] 验证：`cargo test -p xylitol` 相关模块 + 穷举 match 编译检查

## T2 host 发射：SSOT 变更点发布（D1 修订案——ctx 类型化上行）

> 实施期修订：发射点由「gateway 发布器 + composition root 接线」改为
> `XyToolCtx::publish_state` 类型化上行（工具在 gateway 写成功后发布，react
> drain 转发进 run 流）；修订理由见 design.md D1。

- [x] `XyToolCtx` 增 `state_events` 上行（`with_state_event_tx` / `publish_state`，
      unbound 静默）；react `run_one` drain 并原样转发
- [x] todo rewrite / update 成功写入后 `publish_state(TodoUpdated { list })`
      （空表也发布，语义为清除；todo_list 只读不发布）
- [x] 测试：infra 发射断言 + unbound 不 panic + react run 流顺序集成测试
- [blocked-by: T1]

## T3 client 展示：typed live 同步 + todo 块人话化

- [x] bridge `handlers/tools.rs`：处理 `TodoUpdated` → `sync_todo_checklist`（typed）；
      删除 `sync_todo_checklist_from_tool_result` 解析路径与 End 里 `is_todo_tool` 分支
- [x] `preview.rs`：args 摘要补 todo_* 分支（条目计数摘要）；结果人话化补 todo_* 分支
      （勾选清单 body，is_error 不套用）；`output_looks_like_machine_json` 补 items 形态；
      字形表收口单一 helper（checklist 行与块 body 共用）
- [x] 测试：`src/app/tui/bridge/tests.rs` + `preview.rs` 补 live 用例
- [blocked-by: T1]

## T4 重建同构验证

- [x] `session_tree.rs` rebuild 仍走 SSOT 快照 + humanize helper；断言直播与 travel
      重建的 Todo 块 / checklist 形态同构（att50 幂等延伸到块 body）
- [x] transcript BDD executable 场景经 `tests/features` 既有 step 落地（挂 att36）
- [x] 验证：`cargo test --lib --all-features tests::bdd::` 相关 feature
- [blocked-by: T3]

## T5 docs + designing

- [x] `docs/architecture/` 新增「信息平面总览」：五面真值源 / 读写权 / 流向，todo 作
      worked example；遵守 `docs/AGENTS.md` 文档闭环
- [x] `designing/tui/modules/tool/`：补 todo_* 块形态（draft.yaml / intent.md /
      states）；跑 `just gen-designing-index` + `just export-design-frame`
- [x] 验证：`just doc-check` + `check_tui_designing`（随 qa）
- [blocked-by: T3]

## T6 门禁收口

- [x] `just fmt` / `just lint` / `just test` / `just test-tui` / `just qa` 全绿；
      PTY 交互改动需要时 `just qa-e2e`
- [blocked-by: T2, T4, T5]
