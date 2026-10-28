---
depends_on: []
title: 明确计划模式工具拒绝是否结束当前 run
needs_specs_change: true
branch: sdd/c2980-update-plan-deny-run-semantics
base_branch: main
base_sha: c9d44e334dbc5ad772727274a6357a573ad3d66c
---

# 明确计划模式工具拒绝是否结束当前 run

## Why

- `agent-plan` 的 `r2960`（plan-mode-read-only）要求：拒绝须在工具执行前发生，返回稳定原因 `plan mode is read-only`，且不得执行该工具。
- 但该规则没有说明拒绝之后 **run 是否继续**。当前实现返回 `LoopStep::stopped`，runtime 把任何非 `Continue` 控制视为 `stop_run = true`，随后发 `TurnEnd` 并 `break 'outer`。
- 产品后果：模型在 plan mode 尝试一次 `write`，整个回合结束；模型无法在同一回合改调 `plan_write`，用户必须重新发消息。
- 现有 BDD 只断言中间件返回值（`LoopStep::control == Stop`），没有端到端覆盖 run 行为。

## What Changes

- 明确产品语义（二选一，见 design）：
  - **A. 拒绝即结束 run**：在 `agent-plan` 写清楚“拒绝后本 run 结束”，并补 E2E。
  - **B. 拒绝但继续 run**：core 提供不终止 run 的拒绝通道；模型收到工具错误后可在同一回合改调 `plan_write`。
- 按决策更新 `agent-plan` 的 `r2960`，并补对应 runtime / BDD 覆盖。
- 若选 B，评估 SDK 词表变更（`BeforeToolCtx` 增加拒绝字段，或新增控制枚举）与 `SDK_API_VERSION`。

## Capabilities

- `agent-plan`：只读闸拒绝后的 run 语义。
- `agent-runtime`：`before_tool` 非 `Continue` 控制的 runtime 行为与工具错误回写。

## Impact

- `plugins/xylitol-official-plugin-plan/src/middleware.rs`：拒绝路径。
- `crates/xylitol-runtime/src/agent/runtime/tool_exec.rs` / `react/mod.rs`：拒绝后的控制流。
- `crates/xylitol-plugin-sdk/src/loop_middleware.rs`：若选 B，词表/上下文变更。
- `tests/bdd`：端到端 run 行为。

## Out of Scope

- core 权限终裁本身（`permission_check` 仍在 middleware 之后执行，且不可被绕过）。
- 其他中间件的 `Stop` 语义（`decide_continue` / `before_turn` 等保持现状）。
- plan 工具 allowlist 的具体内容。

## Further Notes

- 由 PR #30 review F3 记录。
- 这是**决策型 change**：design 给出 A/B 取舍，未定前不进入 tasks/attach。
