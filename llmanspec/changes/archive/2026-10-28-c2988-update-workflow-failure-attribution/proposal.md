---
depends_on: []
title: 收敛 workflow 工具失败的步骤归因
needs_specs_change: true
branch: sdd/c2988-update-workflow-failure-attribution
base_branch: main
base_sha: 64139091ed121fa5967eca7695836d4e8c2dd0f3
---

# 收敛 workflow 工具失败的步骤归因

## Why

- 当前 workflow 中间件的 `after_tool` 对**任何**工具失败，都会把当前 `in_progress` 步骤标记为 `failed`。
- 一个与步骤无关的 `read` / `grep` 失败（例如路径写错）也会触发局部重规划，打断正常流程。
- spec `agent-workflow` 目前明确写的是“其他工具失败时把当前 in_progress 步骤标记 failed”，所以这是**合约层面的粗归因**，不是实现 bug。
- 粗归因的代价：误判失败、浪费 replan 次数、用户体验上“无关错误导致计划重写”。

## What Changes

- 明确失败归因规则（见 design）：
  - **A. 按步骤声明的工具归因**：步骤声明 `tools`（或 `verification.tools`），只有这些工具失败才标记 failed；
  - **B. 按 `output_ref` / 步骤产物归因**：只有产出该步骤结果的工具失败才标记 failed；
  - **C. 保持粗归因**：任何工具失败都标 failed（现状），但补文档说明；
  - **D. 显式失败工具**：由模型调用 `workflow_fail` 标记失败，其他工具错误只回写。
- 同步更新 `agent-workflow` 条款与单测/BDD。
- 评估是否需要在 `WorkflowStep` 增加字段（`tools` / `failure_policy`）。

## Capabilities

- `agent-workflow`：工具失败与步骤状态归因。

## Impact

- `plugins/xylitol-official-plugin-workflow/src/middleware.rs`：`after_tool` 失败分支。
- `crates/xylitol-plugin-sdk/src/session/workflow.rs`：`WorkflowStep` 若增字段。
- spec 与测试。

## Out of Scope

- workflow DAG 校验与重规划约束（已有）。
- 嵌套 workflow 的失败冒泡（已有）。

## Further Notes

- 由 PR #30 review F11 记录。
- 这是**合约变更**：现状是 spec 明确要求的粗归因，细化需要改 spec。
