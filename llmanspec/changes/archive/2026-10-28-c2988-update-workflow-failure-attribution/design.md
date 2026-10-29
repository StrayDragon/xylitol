# Design

## Evidence

- `plugins/xylitol-official-plugin-workflow/src/middleware.rs:367-382`：`if ctx.is_error` → 找 `WorkflowStepStatus::InProgress` 的步骤 → 置 `Failed`、写 `output_ref = failure:<tool_call_id>`、清审批。
- `llmanspec/specs/agent-workflow/agent-workflow.feature:10`：明确“其他工具失败时把当前 in_progress 步骤标记 failed”。
- `llmanspec/specs/agent-workflow/agent-workflow.feature:23`：嵌套失败冒泡与 `max_replans` 约束。

## Current behavior

- 任意工具错误 → 当前 in_progress 步骤 failed；
- `decide_continue` 见到 failed → 注入局部重规划（受 `max_replans` 限制）；
- 无关工具错误会消耗 replan 次数，甚至导致 workflow 停止。

## Options

### Option A：按步骤声明的工具归因

- `WorkflowStep` 增加 `tools: Vec<String>`（或 `verification.tools`）；
- 只有失败工具在步骤声明集合内，才标记 failed；
- 未声明时回退到当前粗归因（兼容旧 workflow）。

优点：语义精确；误判少。
缺点：需要模型在 `workflow_write` 时声明工具；旧 workflow 无声明仍粗归因。

### Option B：按 `output_ref` 归因

- 只有产出该步骤 `output_ref` 的工具失败才标 failed；
- 优点：无需新增字段。
- 缺点：`output_ref` 是结果而非工具名；映射不直接，可能需要约定。

### Option C：保持粗归因，补文档

- 现状即 spec；
- 优点：零改动。
- 缺点：误判持续；用户看到无关错误触发 replan。

### Option D：显式 `workflow_fail` 工具

- 模型主动标记步骤失败；其他工具错误只回写、不改状态；
- 优点：最精确、可控；不会误判。
- 缺点：模型可能忘记调用；需要 prompt 指引；工具面增加一个。

## Recommendation

推荐 **A**（按声明工具归因）+ 保留粗归因作为旧 workflow 的回退：

- 新 workflow 在 `workflow_write` 时声明每步工具，归因精确；
- 旧 workflow 无声明时维持现状，不破坏兼容；
- 若模型漏声明，仍可能误判，但可通过 prompt 指引缓解。

若不想改 `WorkflowStep` 结构，退而选 **D**。

## Spec wording (draft)

`agent-workflow` 的失败归因改为：

> 步骤声明了工具集合时，只有该集合内工具失败才把步骤标记 failed；未声明时按当前粗归因。与步骤无关的工具错误只回写错误，不得改变步骤状态。

## Test seam

- 单测：步骤声明 `tools=["write"]`，`read` 失败不标 failed，`write` 失败标 failed；
- 兼容：未声明工具的旧 workflow 仍按粗归因；
- `decide_continue`：无 failed 时不注入 replan；有 failed 时按 `max_replans` 重规划。

## Risks

- 模型漏声明工具 → 失败被忽略，workflow 卡住；需要 `decide_continue` 的“无法推进 → Stop”兜底。
- `WorkflowStep` 增字段会影响 `workflow_write` schema 与旧 workflow 解析；需可选字段 + 默认。
- 归因过细可能让真正的失败不被重规划；需要保留显式失败入口（D 可作为补充）。

## Open questions

1. 选 A（声明工具）、B（output_ref）、D（显式失败工具）还是 C（保持现状）？
2. `tools` 字段放在步骤顶层还是 `verification` 下？
3. 旧 workflow 无声明时的回退：粗归因还是“不归因”？

## Decision (implemented)

- 采用 Option A：`WorkflowStep.tools`（serde default）；步骤声明后只有集合内工具失败才标 failed。
- 未声明 `tools` 的旧 workflow 回退粗归因；无关工具错误只回写，不改步骤状态。
- `workflow_write` schema 与建计划契约声明 `tools`；显式 `workflow_fail` 工具留作后续补充。
