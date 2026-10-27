# Design

## Core seam

两个 ctx 增 `session_id: Option<String>`：

- `BeforeModelCtx`（restore + persist 时机）；
- `AfterTurnCtx`（回合结束兜底 persist）。

`run_react_loop` 已有 `session_id`，直接填入；`after_turn_or_fault` 增一个参数透传。不改 turn 边界、权限或落盘格式。

## SDK 工件

`xylitol-plugin-sdk::session::checkpoint`：

```rust
pub const CUSTOM_TYPE_PLAN_CHECKPOINT: &str = "plan_checkpoint";

pub struct PlanCheckpointStep { id, content, status, verification?, output_ref? }
pub struct PlanResumeCursor { step_id, attempt }
pub struct PlanCheckpoint {
    goal?, steps, last_verified_artifact?, last_failure?, resume_cursor?, updated_at
}
```

- `to_custom_entry_shell()` 沿用 `CustomEntry`，不改 v7 磁盘布局；
- `latest_plan_checkpoint(entries)` 取叶分支最新一条，损坏则忽略。

## 中间件

`PlanCheckpointMiddleware`（order 180，`before_model` / `after_tool` / `after_turn`）：

| 阶段 | 行为 |
|---|---|
| `after_tool` | 解析 `todo_rewrite` / `todo_update` 结果更新 steps（保留同 id 的 verification/output_ref）；工具失败记录 `last_failure` 并累加当前 `in_progress` 步骤 attempt；非 todo 成功记录 `last_verified_artifact` |
| `before_model` | 首次调用从会话读最新 checkpoint 并恢复；若有未完成步骤，注入一次 `<plan_checkpoint>` 短摘要；状态有变化则 append 一条 checkpoint |
| `after_turn` | 兜底 append（若仍 dirty） |

写入只追加，不覆盖历史。`before_model` 的写入点让「step 状态变化后、下一次模型决策前」已持久化，等价于 after_tool 写入，但不需要把 session_id 穿进 tool_exec。

## 恢复语义

- 只在恢复后的**首个** `before_model` 注入一次；同一会话后续 run 不再重复。
- 摘要只含：已完成/总数、当前步骤、上次失败；不灌完整 checkpoint。
- 不自动重跑已完成步骤；不自动改写 todo。

## 降级

会话 id 缺失或 `dyn XySessionStore` 不可用时，中间件仍记录内存状态并注入当前 run 的摘要，但不落盘；不中断 run。

## 不做什么

- 不改 todo 工具 schema。
- 不做 VCS 回滚。
- 不把 checkpoint 放进 journal。
- 不跨会话共享 plan。
- 不在中间件里调用模型或工具。
