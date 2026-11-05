---
depends_on: []
needs_specs_change: false
title: 拆分阶段名自动生成策略
branch: sdd/c3170-refactor-tui-stage-name-policy
base_branch: main
base_sha: 8d7213fdf74ffbc7dc5897846146b0b323a1c6f7
---

## Why

阶段名当前只在 `AgentEnd` 里硬判 `stage_name.is_none()`，并用 `stage_name_requested_for` 防止重复；后续要支持按轮次 / 配置重新生成时，判定与去重会耦合在事件处理里。

## What Changes

- 新增 `StageNamePolicy` / `StageNameContext`，把“是否请求生成”从事件处理里拆出。
- 默认策略 `FirstTurn`：本会话未尝试、无阶段名、非 abort 时请求一次；失败不自动重试。
- Host 记录 `completed_turns` 与 `attempted`，供后续 `EveryTurns` / 配置策略使用。
- 移除 `stage_name_requested_for` 会话 id 去重；effects 不再重复判重。
