---
change_id: c455-add-package-tui-input-listener
title: "package-tui InputListener：焦点前输入拦截"
status: purpose-draft
priority: 455
depends_on: []
author: agent
track: A
---

# c455-add-package-tui-input-listener

> **status: purpose-draft**

## Why

c445 future：Esc abort、双 Esc、全局 app 键需在 Editor 焦点前拦截。产品与 demo 都要提前验证。

## Purpose

为 `xylitol-tui` 补齐 `add_input_listener`（或等价）薄管道；`agent_demo` 验证 Esc/Ctrl+C 优先级链。

## What Changes（意向）

1. 引擎：listener 在 focused component 之前；返回 consumed 则停止下传。
2. demo：流中 Esc→abort 脚本；Ctrl+C 清/退；不破坏 Editor 编辑键。
3. 更新 `PI_DELTAS.md` / c445 future。

## Capabilities

- `package-tui-engine`（modify）

## Out of scope

- 完整 overlay focus-restore
