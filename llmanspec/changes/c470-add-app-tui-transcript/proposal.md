---
change_id: c470-add-app-tui-transcript
title: "app-tui-transcript：消息流、可展开块、diff"
status: purpose-draft
priority: 470
depends_on: ["c465-add-app-tui-bridge", "c451-add-package-tui-diff", "c452-demo-highlight-pipeline"]
author: agent
track: B
---

# c470-add-app-tui-transcript

> **status: purpose-draft**

## Why

主内容区：用户回显、流式助手、thinking/tool/diff 可展开；高亮与 diff 用包能力 + 真实 syntect。

## Purpose

实现 transcript 应用面组件（Expandable 留在 app）；接线 Markdown + Diff + highlight。

## Out of scope

- 会话历史回放灌入（可另 change）
- Compaction 专用 UI（c493）
