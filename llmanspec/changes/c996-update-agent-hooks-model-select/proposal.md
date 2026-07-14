---
change_id: c996-update-agent-hooks-model-select
title: "库缝：model_select / thinking_level_select"
status: full
priority: 996
depends_on: ["c990-add-test-hooks-wiring-bdd"]
author: agent
track: A
wave: hooks-wiring-bdd
domain: c995
---

# c996-update-agent-hooks-model-select

## Why

`HookEvent` 已有 `model_select` / `thinking_level_select`，但 `Driver::select_model` / `cycle_model` / `set_thinking_level` 未经 `XyHookBus` 发射。嵌入方与脚本扩展无法观测选模；TUI picker 与 slash 若另挂会双路径漏发。

## Purpose

1. 在 **库权威路径** 成功变更后 MUST observe-dispatch（fail-open，对齐 c735 lifecycle）：
   - `Driver::select_model` / `cycle_model` → `model_select`（context：`model`、`previous`、`source`=`set`|`cycle`）
   - `Driver::set_thinking_level` → `thinking_level_select`（context：`level`、`previous`）
2. **不做** Blocked 拦截选模（pi 亦无 cancel 结果类型）；Blocked 仅日志 fail-open。
3. 所有 client（TUI/Print/Server/embed）凡走上述 Driver API 即自动触发；禁止仅在 UI 旁挂。
4. 启用 `hooks-wiring.feature` 观察例子：`选择模型 fake` / `设置思考级别 high`。

## What Changes

- `InProcessDriver`（及 agent 内实际 set 点若更权威）旁挂 `hook_bus`
- `agent-hooks` + `test-hooks-wiring` delta；BDD 操作字典扩两行

## Capabilities

- `agent-hooks`
- `test-hooks-wiring`

## Out of scope

- session tree/switch（c995）、user_bash（c997）、Completions 三缝（c998）
- 拦截选模 UX

## Ethics

- risk_level: low
- prohibited_actions: 只在 TUI 接线；静默吞选模失败
- required_evidence: wiring BDD 两行绿；`validate --strict`
- escalation_policy: 若 set 点在 agent 而非 Driver，以 agent 为唯一发射点并文档化

## Depends

- **c990-add-test-hooks-wiring-bdd**（已归档）
