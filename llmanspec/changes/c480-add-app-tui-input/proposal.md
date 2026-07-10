---
change_id: c480-add-app-tui-input
title: "app-tui-input：Editor、slash、steer/follow-up 键位"
status: purpose-draft
priority: 480
depends_on: ["c460-add-app-tui-host", "c461-expose-steer-followup-seam", "c455-add-package-tui-input-listener"]
author: agent
track: B
---

# c480-add-app-tui-input

> **status: purpose-draft**

## Purpose

Editor 操作区；`/exit` `/model`；CompletionSource；键位：
- 流中 Enter → steer
- Alt+Enter → follow-up
- Esc → abort（流中）/ 关选择器
- Ctrl+C → 有内容清缓冲；空则退出
- 双 Esc → 会话树（若 c491 未到，可先 stub 或依赖 demo 行为文档）

## Out of scope

- 产品 palette/settings（demo 已有；产品 future）
- bash `!`（c492）
