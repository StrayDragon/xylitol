---
depends_on:
  - c2957-add-loop-example-plan-state
title: 计划 checkpoint：把 plan 进度落成可恢复的会话工件
---

# 计划 checkpoint：把 plan 进度落成可恢复的会话工件

## Why

`c2957` 的 `PlanStateMiddleware` 把进度放在**进程内** `Mutex<PlanProgress>`：

```rust
pub struct PlanStateMiddleware {
    progress: Mutex<PlanProgress>,
    max_settles: u32,
}
```

它能驱动一次 run 的 `decide_continue`，但**进程重启/会话恢复后进度丢失**。todo 工具虽然把列表落会话存储，但缺少「已验证到哪一步、下一步是什么、恢复游标」这类机器状态。ADPS M3 明确要求 checkpoint 同时保存可验证 artifact、动作回执和 resume cursor；常见失效是「Todo 显示完成，业务状态却没有提交；恢复时又用旧 checkpoint 重复不可逆动作」。

现状证据：

```text
crates/xylitol-plugin-sdk/src/session/entries.rs
  CustomEntry { base, custom_type, data }   ← 可承载任意机器状态

plugins/xylitol-official-plugin-tools/src/tools/todo.rs
  todo_rewrite / todo_update → CustomEntry(CUSTOM_TYPE_AGENT_TODO)

crates/xylitol-plugin-sdk/src/session/todo.rs
  latest_agent_todo(entries) 只读 todo，不读 checkpoint
```

## What Changes（候选，promote 时逐条重估）

### 1. checkpoint 工件

新增 `custom_type = "plan_checkpoint"` 的 `CustomEntry`，`data` 至少含：

```json
{
  "goal": "...",
  "steps": [{"id": "...", "status": "...", "verification": "...", "output_ref": "..."}],
  "last_verified_artifact": "...",
  "resume_cursor": {"step_id": "...", "attempt": 1},
  "updated_at": 0
}
```

写入时机：

- `after_turn`（每回合结束）；
- 某个 step 状态变为 `completed` / `failed`（由 `after_tool` 识别）；
- 压缩前后（避免 checkpoint 被压缩掉）。

### 2. 恢复

`before_turn` / `before_model` 读最新 checkpoint，注入短摘要（当前 step、未完成项、上次失败原因）。不得把整个 checkpoint 原样灌进上下文；只给恢复所需字段。

### 3. 与 todo 的关系

todo 是**人可见的任务台账**；checkpoint 是**机器恢复状态**。两者可引用同一 step id，但不互相替代：

- todo 允许模型自由重写；
- checkpoint 只在可验证状态变化时写入，且保留历史（不覆盖删除）。

### 4. 防重复不可逆动作

恢复时若 checkpoint 显示某 step 已 `completed` 且有 `verification`，不得重新执行该 step；若 `resume_cursor` 指向失败 step，先复验前置条件再继续。

## Capabilities

- `agent-session-store`：`plan_checkpoint` custom entry 的写入/读取语义（沿用 `CustomEntry`，不改磁盘格式）。
- `agent-runtime`：`after_turn` / `after_tool` 写入时机与 `before_model` 注入的合约。
- `agent-prompt`：恢复摘要的注入位置与长度约束。

## Impact

- crate：`plugins/xylitol-official-plugin-plan-state`（可从 `loop-example` 的 `PlanStateMiddleware` 抽出）或扩展现有插件；`xylitol-plugin-sdk` 增 checkpoint helper。
- 测试：写入/读取往返、恢复时不重复已完成 step、压缩后 checkpoint 仍可读、无 checkpoint 时行为不变。
- 文档：`src/AGENTS.md` 补「todo 给人看，checkpoint 给恢复用」。

## Out of Scope

- 改 todo 工具 schema。
- 自动回滚文件编辑（需要 VCS/事务，单开 change）。
- 跨会话共享 plan。
- 把 checkpoint 放进 journal（journal 只存事实事件，见 `c2953`）。

## Further Notes

**ROI**：高。长任务恢复是当前最明显的缺口；`CustomEntry` 已存在，不需要改存储格式。

**开放问题**：

1. checkpoint 写 `CustomEntry` 还是旁路文件？`CustomEntry` 随会话树/压缩走，旁路文件更易审计但需额外清理策略。建议先 `CustomEntry`。
2. 压缩会不会丢掉 checkpoint？`c2953` 已定 journal/tape 边界；checkpoint 是会话条目，需确认 compaction 的 keep 规则。
3. `resume_cursor` 的 `attempt` 上限与 `c2961`（自愈）如何共享？建议统一由 plan 状态提供，而不是两个中间件各自计数。
