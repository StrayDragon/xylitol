---
change_id: c470-add-app-tui-transcript
title: "app-tui-transcript：已搁置 — 不做 Codex 式 TranscriptView"
status: paused
priority: 470
depends_on: []
author: agent
track: B
---

# c470-add-app-tui-transcript

> **status: paused（产品决策：不做 Codex 式 transcript view）**

## 决策（2026-07-10）

**不做**独立的、Codex 风格的 transcript 浏览/呈现面（可滚动消息栈作为主 UX、专用 TranscriptView 组件树等）。

历史浏览、分支 travel / fork 的目标改由 **双 Esc 会话树**达成：

| 优先 | Change |
|---|---|
| 包 TreeSelector | **c454** |
| demo 双 Esc 原型 | **c456** |
| 产品接线 | **c491**（提前，不再等本变更） |

## 为何搁置

- Codex 式 transcript view 与当前选定的 pi 交互（scrollback + editor 槽选择器）重复且更重。
- 双 Esc 树已能覆盖「回看 / 跳转 / fork」目的，应先落地可验证路径。

## 若日后需要「当前轮输出」

仅允许 **极简 live 行写入终端 scrollback**（bridge 驱动的短消息/工具摘要），**MUST NOT** 复活本变更名义下的 Codex 式 TranscriptView。届时另开 change，且不得复用本 id 的旧 purpose。

## Out of scope（永久相对本 change）

- 应用面 Expandable 消息栈 / Diff 块专用 transcript 组件树
- 以 c470 为垂直切片（c485）硬依赖
