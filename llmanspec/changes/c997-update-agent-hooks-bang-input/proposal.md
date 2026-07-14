---
change_id: c997-update-agent-hooks-bang-input
title: "Hook：user_bash / input（TUI bang 与提前提交）"
status: purpose-draft
priority: 997
depends_on: ["c735-update-agent-hooks-pi-parity"]
author: agent
track: A
wave: hooks-pi-parity-followup
domain: c995
---

# c997-update-agent-hooks-bang-input

> **status: purpose-draft** — 承接 `c735` future 的 `user_bash` / `input`。
> **depends_on c735**：归档后再 apply。

## Why

pi 扩展可拦 `user_bash` 与 `input`。xylitol TUI bang（`!`/`!!`）与 editor 提交走 host/`effects`，未发脚本 hook；枚举名已有或可加，但热路径未接。

## Purpose

1. **`user_bash`**：交互 bang 提交前 MUST dispatch（command、exclude_from_context 等）；Blocked → 不执行并表面提示（与现有 bang reject 一致可测）。
2. **`input`**（可选升格范围）：idle/busy 提交用户文本前观察或改写；须避免与 slash/steer 抢键；升格时写清是否 MUST 或 MAY。
3. 通过 app 层持有的 `XyHookBus`/`HookDispatcher` 调用（TUI 可依赖 infra/composition），**不**让 `packages/xylitol-tui` 依赖主 crate hooks。
4. 空配置零开销；不改变 bang abort 与 `(cancelled)` 语义（c665/c669）。

## What Changes（升格后预期）

- `effects` / host bang 与 submit 路径挂 hook
- `agent-hooks` + 可能 `app-tui-input` / commands delta
- BDD 或 harness：bang block / allow

## Capabilities

- `agent-hooks`
- `app-tui-input` 和/或 `app-tui-commands`（升格时声明）

## Out of scope

- Driver session hooks（`c995`）
- model_select（`c996`）
- Completions HTTP（`c998`）
- 通用 keybinding 扩展市场

## Ethics

- risk_level: medium（可拦用户输入/shell）
- prohibited_actions: 默认 block 所有 input；hook 超时阻塞 UI 超过既有 timeout 策略
- required_evidence: bang allow + block 场景
- escalation_policy: `input` 改写语义有歧义时先确认再 MUST

## Depends

- **c735-update-agent-hooks-pi-parity**
