---
change_id: c997-update-agent-hooks-bang-input
title: "库缝：user_bash（Driver::execute_bash）"
status: full
priority: 997
depends_on: ["c990-add-test-hooks-wiring-bdd"]
author: agent
track: A
wave: hooks-wiring-bdd
domain: c995
---

# c997-update-agent-hooks-bang-input

## Why

pi 可拦 `user_bash`。xylitol TUI bang 与任意 client 最终应走 `Driver::execute_bash`；该库 API 尚未 dispatch。

## Purpose

1. **`Driver::execute_bash` 执行前** MUST dispatch `user_bash`（context：`command`、`exclude_from_context`；可选 `cwd`）。
2. **Blocked** → 不执行 bash，返回可观测错误（与产品 reject 文案可后续对齐）。
3. **Modify.command**（MAY）：若实现则须测；最小 MUST 为 allow/block。
4. **`input` 事件本 change 不做**（后置；易与 slash/steer 抢语义）。
5. TUI bang 只调用同一 Driver API，禁止第二套 hook 点；`packages/xylitol-tui` 不依赖 hooks。
6. wiring：`执行 bash` 操作 allow + block 场景。

## What Changes

- `InProcessDriver::execute_bash`（或 agent `execute_bash` 唯一点）旁挂
- `agent-hooks` + `test-hooks-wiring`；BDD

## Capabilities

- `agent-hooks`
- `test-hooks-wiring`

## Out of scope

- `input` 事件；c995/c996/c998；改 bang abort/(cancelled) 语义

## Ethics

- risk_level: medium
- prohibited_actions: 默认 block 全部 bash；hook 超时阻塞超过既有策略
- required_evidence: allow + block 各一
- escalation_policy: Modify 有歧义时只交 allow/block

## Depends

- **c990-add-test-hooks-wiring-bdd**
