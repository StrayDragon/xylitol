# Design

## Core seam

`XyToolCtx` 增 `session_id: Option<String>`；`ToolExecEnv` 从 ReAct 的 `session_id` 透传。工具据此把工件写回当前会话，不需要额外 gateway 绑定。core 仍持落盘格式与权限终裁。

## SDK 工件

`xylitol-plugin-sdk::session::plan`：

```rust
pub const CUSTOM_TYPE_PLAN: &str = "plan";

pub enum PlanStepStatus { Pending, InProgress, Completed, Failed, Blocked }
pub struct PlanStep {
    id, description, depends_on, inputs, outputs, risk,
    verification?, rollback?, requires_human, status, output_ref?
}
pub struct Plan { goal, steps, resume_cursor?, updated_at }
```

- `latest_plan(entries)` 取叶分支最新一条；
- `validate_plan`：id 唯一、依赖存在、无环；
- `runnable_step`：`pending` 且依赖全部 `completed`（`requires_human` 未批准时不视为可执行）；
- `downstream_ids`：反向依赖闭包；
- `validate_replan(previous, next)`：已完成步骤不得删除/降级/改写关键字段；只有失败步骤及其下游可改；无失败时只允许 status / output_ref 变化。

## plan 工具

`PlanTool` 由 loop 插件在 `plan_execute` 启用时注册到 `HostRegistries`，卸载时移除。参数为 `{ goal, steps[] }`，执行时：

1. 解析并 `validate_plan`；
2. 从 `ctx.session_id` 读最新 plan（无会话则报错）；
3. `validate_replan`，拒绝重写已完成/无关步骤；
4. append `CustomEntry(CUSTOM_TYPE_PLAN)`，返回新 plan JSON。

工具是唯一写 plan 的入口；中间件只读结果并驱动决策。

## 中间件状态机

`PlanExecuteMiddleware`：order 190，阶段 `before_model` / `after_tool` / `decide_continue`。

```text
before_model:
  plan 未加载 → 从会话读最新 plan
  无 plan     → 注入一次 plan 契约（用 plan 工具建计划）
  有失败步骤 → 注入 <plan_replan>（只改失败步骤及下游）
  有 blocked  → 注入 <plan_blocked>
  有 runnable → 注入 <plan_step>（当前步骤、inputs、verification）
  全部完成    → 注入一次 <plan_complete> 要求总结

after_tool:
  plan 工具成功 → 采用新 plan；若此前有失败步骤，replan_count += 1
  其它工具失败 → 把当前 in_progress 步骤标记 failed，并记录失败摘要
  当前步骤 completed → 清除该步骤的审批

decide_continue:
  blocked 且未批准        → Stop（等用户）
  blocked 且用户有新消息  → 批准一次，Continue
  有失败且 replan < cap   → Continue（强制重规划）
  有 runnable             → Continue
  全部完成且未总结        → Continue（一次）
  全部完成且已总结        → Stop
  无法推进（无 runnable / 无失败） → Stop
```

## 审批

`requires_human` 步骤在未批准时阻塞。用户的下一条消息视为一次批准：中间件把该步骤加入 approved 集合并放行；步骤进入 `completed` 或 `failed` 后清除批准（只消费一次）。不新增审批 UI，复用「用户发言即批准」的保守语义。

## 有界

`max_replans` 缺省 3；达到上限后 `decide_continue` Stop 并记录原因。局部重规划由 plan 工具校验，不靠模型自觉。

## 恢复

plan 本身是 `CustomEntry`，叶分支最新一条即恢复状态；`runnable_step` 在恢复时重新计算，不需要额外 resume cursor。`c2960` 的 plan_checkpoint 仍可并存。

## 不做什么

- 多 Agent / 子 Agent。
- 云端 PR、向量索引、Repo map。
- 自动 VCS 回滚。
- 把 plan 执行器做成 core 内联逻辑。
- 独立 planner 模型。
