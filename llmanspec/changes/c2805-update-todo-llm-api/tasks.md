# Tasks — Todo LLM API（A3 + slim + 三态）

垂直切片。接缝全部复用既有入口（见 `research/current-impl-and-seams.md`），
不新发明 harness。

## 接缝（已锁定）

| 接缝 | 入口 |
|---|---|
| 领域 | `src/protocol/session/todo.rs` 单测 |
| 工具 + 网关 | `src/infra/tools/todo.rs` 单测 |
| 10 闭集 / Barrier | `src/infra/tools/mod.rs`；`steps_tools.rs` 十工具烟测 |
| 冻表 | `src/agent/capabilities/mod.rs` `freeze_includes_todo_builtins_first_turn` |
| 待办栏 | `src/app/tui/widgets/todo_bar.rs`；BDD `todo-bar-default-shows-in-progress` / `todo-bar-resume-matches-live` |
| 工具块 | `src/app/tui/bridge/preview.rs`；BDD att36 `todo-block-body-renders-checklist` / 空摘要 / 空表 body |
| 事件 | `TodoUpdated` 仍全量；`react/tests.rs`、`wire/event.rs` 往返 |
| compact | `ensure_agent_todo_after_compact` |

## T1 领域：三态 + 稳 id + 旧 cancelled 跳过

- [ ] `TodoStatus` 去掉 `Cancelled`；`done_total` 只计 completed
- [ ] rewrite/add 省略 id → `t_` + 8 hex，禁止 `todo-{idx}`
- [ ] 读快照：`status: "cancelled"` 丢掉该行，其余行仍加载
- [ ] 单测：`src/protocol/session/todo.rs`
- 验证：域单测绿；含 cancelled 的夹具仍能 `latest_agent_todo`

## T2 工具面：A3 schema / slim ack / guidelines

- [ ] `todo_update` 改为 `items[]`（每条必有 id）；拒绝顶层 `{id,status}`
- [ ] mutate 回 `{ids}` / `{changed:[{id,status}]}`；`todo_list` 仍全表
- [ ] description 三行短句；`todo_update` 一条 `prompt_guidelines`
- [ ] description 去掉 dual in_progress
- [ ] gateway `update` → patches；成功仍 persist 全量 + `TodoUpdated { list }`
- [ ] 单测：`infra/tools/todo.rs`；闭集仍 10 名
- 验证：工具单测 + freeze 单测绿
- [blocked-by: T1]

## T3 待办栏：前翼只 completed

- [ ] `todo_bar` 不再把 cancelled 混进 past；翼头无 `· n cancelled`
- [ ] glyph 表去掉 `[-]`
- [ ] 单测：`todo_bar.rs` 含 cancelled 的夹具改为「读取跳过 / 不出现」
- 验证：待办栏单测 + BDD r1129/r1130
- [blocked-by: T1]

## T4 工具块：按 ack 画受影响行

- [ ] `humanize_tool_result_for_tui`：list 仍 `{items}` 勾选；rewrite/update 按 ack
- [ ] update header：`{n} updated`（args.`items` 是补丁数组）
- [ ] att36 BDD 步骤与夹具同步（rewrite 直播仍可有全表 args；result 改 ack）
- 验证：`preview.rs` 单测 + `todo-block-body-renders-checklist` / 空态场景
- [blocked-by: T2]

## T5 门禁

- [ ] `just fmt` / `just lint` / 相关 `cargo test` / `tests::bdd::` todo 场景
- [blocked-by: T3, T4]
