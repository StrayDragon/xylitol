---
depends_on: []
title: 为 workflow 增加显式失败工具
needs_specs_change: true
branch: sdd/c2989-add-workflow-fail-tool
base_branch: main
base_sha: 75f1ba6c2560f5c56f57109a474458f78e9ca1ba
---

# 为 workflow 增加显式失败工具

## Why

- c2988 把失败归因收敛为“步骤声明 `tools` 时只有集合内工具失败才标记 failed”；无关工具错误只回写。
- 但验证不通过、环境不满足、模型判断无法继续等非工具错误没有归因入口，步骤会长期停在 `in_progress`，只能靠改写整个 workflow 绕过。
- `workflow_write` 虽可改 status，但语义是“重写 DAG”，用它表达单步失败会放大改动面，也容易触发不必要的结构校验。

## What Changes

- 新增 `workflow_fail` 工具：`step_id` 可选（缺省当前 `in_progress`），`reason` 必填。
- 目标步骤非 completed 时标记 failed，写 `output_ref = failure:<reason>`，append 新 workflow 条目并发布 `WorkflowUpdated`。
- 中间件 `after_tool` 对 `workflow_fail` 复用 `workflow_write` 的 reload / pending error 路径。
- 更新 `agent-workflow` 的工具契约与场景。

## Capabilities

- `agent-workflow`：显式失败工具与归因语义。

## Impact

- `plugins/xylitol-official-plugin-workflow/src/tools.rs`：新增工具与注册。
- `plugins/xylitol-official-plugin-workflow/src/middleware.rs`：`after_tool` 特判扩展。
- `llmanspec/specs/agent-workflow/agent-workflow.feature`：`r2966` 扩展。
- `tests/bdd/steps_loop_middleware.rs` + `bindings_agent_workflow.rs`：场景。

## Out of Scope

- 不新增 `workflow_fail` 的自动重规划策略；沿用既有 `decide_continue`。
- 不改 DAG 校验与 `workflow_write` 形状。
- 不引入新的 `custom_type` 或磁盘格式。
