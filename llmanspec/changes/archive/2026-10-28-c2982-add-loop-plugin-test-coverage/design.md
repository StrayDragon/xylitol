# Design

## Coverage debt

| 函数 | 位置 | CRAP / CC | 当前覆盖 | 测试方式 |
|---|---|---:|---:|---|
| `PlanSubmitTool::execute_typed` | `plugins/xylitol-official-plugin-plan/src/tools.rs:703` | 240 / 15 | 0% | fake `AskUserGateway` + `SessionPlanGateway`（in-memory store） |
| `PlanModeTool::execute_typed` | `plugins/xylitol-official-plugin-plan/src/tools.rs:860` | 240 / 15 | 0% | 同上；覆盖已批准跳过 ask、未知 mode、拒绝 |
| `todo_items_from_parts` | `plugins/xylitol-official-plugin-todo/src/middleware.rs:392` | 240 / 15 | 0% | 纯函数，表驱动（合法/缺字段/非 JSON） |
| `TodoReminderMiddleware::before_llm_request` | `plugins/xylitol-official-plugin-todo/src/middleware.rs:532` | 72 / 8 | 0% | fake todo + 构造 `BeforeLlmRequestCtx`；覆盖 stale / completion / error / 同 turn 去重 |
| `TodoProgressMiddleware::record_todo_result` | `middleware.rs:72` | 56 / 7 | 0% | `AfterToolCtx` 表驱动 |
| `TodoCheckpointMiddleware::before_llm_request` | `middleware.rs:427` | 56 / 7 | 0% | fake store + todo；覆盖首轮注入/无 checkpoint/已完成 |
| `TodoCheckpointMiddleware::inject_summary` | `middleware.rs:287` | 30 / 5 | 0% | 纯格式化 + 条件 |
| `TodoProgressMiddleware::decide_continue` | `middleware.rs:125` | 20 / 4 | 0% | `DecideContinueCtx` 表驱动 |
| `TodoCheckpointMiddleware::record_tool_outcome` | `middleware.rs:241` | 20 / 4 | 0% | `AfterToolCtx` |
| `TodoCheckpointMiddleware::persist` | `middleware.rs:325` | 20 / 4 | 0% | fake store，断言只追加 |
| `ensure_state_entries_after_compact` | `plugins/xylitol-official-plugin-compaction/src/lib.rs:397` | 34 / 16 | 58.6% | fake store；覆盖 `plan_doc` / `plan_ref` / `workflow` / `todo_checkpoint` |
| `SessionPlanGateway::read_file` | `plan/tools.rs:197` | 42 / 6 | 0% | temp dir + trust/deny 分支 |
| `PlanWriteTool::execute_typed` | `plan/tools.rs:489` | 42 / 6 | 0% | 已有 fixture 扩展 |
| `PlanReadTool::execute_typed` | `plan/tools.rs:565` | 30 / 5 | 0% | 同上 |
| `map_bootstrap_error` | `crates/xylitol-kernel/src/occupancy/runtime.rs:93` | 42 / 6 | 0% | 全变体表驱动 |
| `build_ports` | `runtime.rs:76` | 20 / 4 | 0% | 两个分支（`sessions_dir` 有/无） |
| `XyRemoteDriver::exec_export_html` / `exec_export_jsonl` | `crates/xylitol-app-tui/src/driver/remote.rs:1566` / `:1588` | 30 / 5 | 0% | fake `HostClient`，覆盖 content/path 回退 |
| `slash/misc.rs::plan` | `crates/xylitol-app-tui/src/effects/slash/misc.rs:70` | 20 / 4 | 0% | TUI harness 或提取纯 helper |
| `same_workflow_structure` | `crates/xylitol-plugin-sdk/src/session/workflow.rs:411` | 16.7 / 6 | 33.3% | 表驱动结构比较 |

## Test seams

### todo middleware

- fake `AgentTodoGateway`：内存 `TodoList`，记录 `rewrite` / `update` 调用；
- fake `XySessionStore`：内存 entries，记录 append；
- `BeforeLlmRequestCtx { turn_index, messages, todo: Some(list), session_id }`；
- 断言 `LoopStep.ctx.messages` 末尾是否注入 `<todo_reminder>` / checkpoint 摘要，以及 `properties` 的语义键。

### plan tools

- 复用现有 `SessionPlanGateway::new_with_roots` + `SessionManager::in_memory()` fixture；
- fake `AskUserGateway`：可编程返回 `approve` / `revise` / 自由文本；
- fake `AgentTodoGateway`：记录 `rewrite`，可注入失败；
- 断言 `PlanDocStatus`、`PlanMode`、todo seed 结果与 `publish_state` 事件（`XyToolCtx::with_state_event_tx` 已有测试范式）。

### compaction retention

- 构造 store entries（`plan_mode` / `plan_doc` / `plan_ref` / `workflow` / `todo_checkpoint`）；
- 调 `ensure_state_entries_after_compact`；
- 断言压缩后重新追加的条目种类与顺序，且不覆盖历史。

### low-risk

- `map_bootstrap_error` / `build_ports`：直接调用，表驱动；
- `exec_export_html/jsonl`：复用 `driver/tests.rs` 的 fake host；
- `slash/misc.rs::plan`：若 effects 层难测，先把文案/分支提取成纯函数再测（不改行为）；
- `same_workflow_structure`：构造相同/不同结构。

## Test layering

- 以上均为**组件边界单测**，不新增 BDD；
- 若某条 spec 明确要求端到端（例如 `agent-plan` 的审批流），可在后续 change 补 BDD，不在本 change 扩场景；
- 覆盖率以 `cargo llvm-cov` / CRAP 报告复核，不追求 100%。

## Risks

- 测试可能暴露与 spec 不一致的行为（例如 reminder 注入条件、checkpoint 去重）。处理原则：小偏差同 change 修；大偏差另开 change，不把错误行为写成断言。
- fake gateway 与真实 gateway 行为漂移；尽量复用现有 fixture（`SessionManager::in_memory`、`SessionPlanGateway`）。
- `publish_state` 事件断言依赖 `with_state_event_tx`，与生产接线一致；不要只测“函数返回”。
- 覆盖率提升不一定线性降低 CRAP（CC 不变）；本 change 目标是**行为守护**，不是数字归零。

## Open questions

1. 低风险薄函数是否全部纳入？建议纳入 `map_bootstrap_error` / `build_ports` / `exec_export_*` / `slash plan` / `same_workflow_structure`，其余按性价比取舍。
2. 是否要为 plan 审批流补 BDD？若要，需改 `agent-plan` 的裸规则为嵌套场景（属 spec 变更，超出本 change）。
3. `todo` 中间件的 `properties` 键是否需要与 `c2977` 的观测约定对齐后再断言？

## Decision (implemented)

- 已补 todo 中间件 12 个测试、plan 审批/模式 7 个测试、压缩状态保留 3 个测试。
- 已补 `map_bootstrap_error` 全变体、远端 `export_html/jsonl`、`same_workflow_structure` 测试。
- 未新增 BDD；测试暴露的偏差未出现，未改动生产行为。
