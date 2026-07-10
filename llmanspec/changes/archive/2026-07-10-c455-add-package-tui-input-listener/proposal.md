---
change_id: c455-add-package-tui-input-listener
title: "package-tui InputListener：焦点前输入拦截"
status: full
priority: 455
depends_on: []
author: agent
track: A
---

# c455-add-package-tui-input-listener

## Why

c445 future：Esc abort、双 Esc、全局 app 键需在 Editor 焦点前拦截。产品与 demo 都要提前验证。解锁 c460 host / c480 input / c456 双 Esc。

## Purpose

为 `xylitol-tui` 补齐 `add_input_listener`（或等价）薄管道；`agent_demo` 验证 Esc/Ctrl+C 优先级链。

## What Changes

1. 引擎：listener 在 focused overlay/component 之前；返回 `Consumed` 则停止下传。
2. demo：流中 Esc→abort 脚本；Ctrl+C 清编辑器 / 空则退；不破坏 Editor 编辑键。
3. 更新 `PI_DELTAS.md`；单测覆盖 consume-before-focus。

## Capabilities

- `package-tui-engine`（modify）

## Out of scope

- 完整 overlay focus-restore
- 全局 debug 键（Shift+Ctrl+D）
- VT 字符串 listener（保持 `InputEvent`）
- `src/app/tui/` 产品面接线（c460/c480）
