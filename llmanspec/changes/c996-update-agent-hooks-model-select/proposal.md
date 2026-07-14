---
change_id: c996-update-agent-hooks-model-select
title: "Hook：model_select / thinking_level_select"
status: purpose-draft
priority: 996
depends_on: ["c735-update-agent-hooks-pi-parity"]
author: agent
track: A
wave: hooks-pi-parity-followup
domain: c995
---

# c996-update-agent-hooks-model-select

> **status: purpose-draft** — 承接 `c735` future 选模/思考级别缺口。
> **depends_on c735**：归档后再 apply。

## Why

pi 在 `/model` 与 thinking level 变更时发 `model_select` / `thinking_level_select`。xylitol `HookEvent` 已有名，但 Driver/`SetModel`/thinking 设置路径未 `dispatch`，扩展无法观测或拦截选模。

## Purpose

1. 在 **成功变更模型** 与 **成功变更 thinking level** 的权威路径（Driver / session / settings）上 MUST 发：
   - `model_select`（context 含 model id / display 等）
   - `thinking_level_select`（context 含 level）
2. 可选：Blocked 表示拒绝切换（须 BDD）；默认建议 observe + fail-open，与 c735 lifecycle 一致，升格时二选一写死。
3. 与 models picker / `/model` slash 共用同一 hook 点，禁止双路径漏发。

## What Changes（升格后预期）

- Driver `set_model` / thinking API 旁挂 `XyHookBus`
- `agent-hooks` delta + BDD
- 勾销 c735 future 对应行

## Capabilities

- `agent-hooks`
- 可能 `app-tui-commands` / models picker（升格时声明）

## Out of scope

- Driver session tree/switch（`c995`）
- bang/input（`c997`）
- Completions HTTP（`c998`）

## Ethics

- risk_level: low
- prohibited_actions: 静默吞掉选模失败且不通知用户
- required_evidence: 改模型与改 thinking 各至少一测
- escalation_policy: 若要做拦截选模，须产品确认 UX

## Depends

- **c735-update-agent-hooks-pi-parity**
