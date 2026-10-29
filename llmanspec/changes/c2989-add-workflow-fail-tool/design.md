# Design

## Context

`workflow_write` 是“重写整个 DAG”的工具，`validate_workflow_replan` 会限制已完成步骤与结构变更。把单步失败塞进 `workflow_write` 会让模型重发整张图，且容易触发不必要的结构校验。`workflow_fail` 只做一件事：把目标步骤置 failed 并记录原因。

## Decision

采用 **A. 显式 `workflow_fail` 工具**，不改 `workflow_write` 语义。

### 工具边界

- `step_id` 可选：缺省取当前唯一的 `in_progress` 步骤；显式给出时按 id 查找。
- `reason` 必填且非空。
- `completed` 步骤拒绝（不得把已完成步骤降级为 failed）。
- 写入前复用 `validate_workflow_replan(previous, &next)`；只改 status / output_ref，符合既有重规划约束。
- 成功 append `custom_type = "workflow"` 的 CustomEntry 并发布 `WorkflowUpdated`；失败返回 `InvalidArgs` 且不写盘。
- 返回 `{ step_id, status: "failed", reason }`，便于模型确认。

### 中间件

`after_tool` 对 `workflow_fail` 与 `workflow_write` 走同一路径：成功 `reload_workflow()`，失败写 `pending_workflow_error`。不改变 `decide_continue` 的既有重规划策略。

## Alternatives

- **B. 复用 `workflow_write` 改 status**：改动面大，语义是重写 DAG，且需要模型重发全部步骤。
- **C. 按验证输出自动归因**：需要解析验证文本/退出码，超出本 change；先给模型显式入口。
- **D. 不提供工具**：非工具错误无归因入口，步骤会长期停在 `in_progress`。
