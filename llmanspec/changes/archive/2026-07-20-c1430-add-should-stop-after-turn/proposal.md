---
change_id: c1430-add-should-stop-after-turn
title: ReAct 对齐 pi shouldStopAfterTurn（开放停闸 + 移除 max_iterations）
status: full
priority: 1430
depends_on: []
author: agent
track: A
wave: agent-runtime-stop
domain: agent-runtime
apply_band: P2
branch: feat/c1430-add-should-stop-after-turn
base_sha: cef465c1c4b832e8168dde66057dcbd69be2a01e
checkpointed: true
checkpoint_sha: cef465c1c4b832e8168dde66057dcbd69be2a01e
---

# c1430-add-should-stop-after-turn

## Why

1. xylitol 在 ReAct 内硬编码 `max_iterations`（默认 50）：撞上限时静默 `AgentEnd`，用户易误以为任务已做完。
2. **pi** 无硬步数闸；停条件走可选单个 `shouldStopAfterTurn`。硬编码 50 不是对齐源。
3. 停策略应开闭：组合根 / 嵌入方注册回调，而不是改 `react.rs` 或堆配置旋钮。

## Purpose

对齐 pi：每个 ReAct turn 在 **`TurnEnd` 之后** 调用可选的 `should_stop_after_turn`；返回 true 则优雅结束本轮 `run`（直接 `AgentEnd`，不再开下一拍 LLM，且 **不** drain steer / follow-up）。

默认 **完全开放**（无步数闸）；一次性移除 `max_iterations` 旧配置与 loop 内判断。无兼容 shim。

## What Changes

- ReAct：`TurnEnd` 后调用可选单槽 `should_stop_after_turn`；`true` → `AgentEnd` 并跳过 steer / follow-up（严格 pi）
- 删除 `max_iterations`：loop、`AgentBuilder` / `AgentCapabilities`、profile YAML、`config.schema.json`、相关 defaults
- 事件：不新增 `RunStopped`；停闸路径仅既有 `TurnEnd` → `AgentEnd`（对齐 pi）
- Specs：`agent-runtime`（ar1/ar8/ar16 + 新 ar24）；`runtime-config`（删字段合约）
- 可执行 BDD：停闸跳过 follow-up；无钩子开放结束

## Capabilities

- `agent-runtime`（modify）
- `runtime-config`（modify）

## Impact

- 长跑不再被 50 步静默掐断；失控循环靠用户 abort / 另案 compact·Goal
- 配置里若仍写 `max_iterations`：加载失败或拒绝未知字段（实现钉死；倾向 serde deny/忽略策略在 design）

## Out of scope

- `prepareNextTurn`、Goal 续跑、compact 自动闭环、Sub-agent
- 新 `XyEvent` 停闸变体 / 达闸文案 UX
- 脚本 `XyHookBus` 镜像（后置）

## Ethics

- risk_level: medium
- prohibited_actions: 保留 max_iterations shim；新增非 pi 停闸事件；stop 后仍 drain follow-up；做成 hook Vec
- required_evidence: BDD/单测证明调用点与跳过 follow-up；配置字段已删；`just qa` 绿
- escalation_policy: 无（决议已钉）

## Notes

- 2026-07-20：purpose-draft → full；决议见 design「已决议」。
