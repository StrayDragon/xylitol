---
change_id: c485-add-app-tui-vertical-slice
title: "app-tui 垂直切片：可聊一轮 E2E"
status: purpose-draft
priority: 485
depends_on: ["c475-add-app-tui-chrome", "c480-add-app-tui-input", "c465-add-app-tui-bridge"]
author: agent
track: B
---

# c485-add-app-tui-vertical-slice

> **status: purpose-draft**

## Purpose

端到端：TTY 启动 → trust 已决或跳过 → 提交 → 流式回复/工具 → steer/follow-up → abort → `/exit` restore。作为轨 B MVP 归档门槛。

## 依赖说明

- **不再**硬依赖 c470（Codex 式 TranscriptView 已搁置）。
- Live 输出以 bridge + 极简 scrollback 行为为准；分支导航可与 **c491** 并行/提前。

## Out of scope

- Codex 式 transcript 浏览器
- bash（c492）、compaction UI（c493）除非已提前完成
