---
change_id: c464-fix-package-tui-diff-sbs-paint
title: "package-tui-diff：SBS 去整行红/绿底，避免与 tool-*-bg 抢语义"
status: full
priority: 464
depends_on: ["c459-enhance-diff-edit-line-syntax", "c462-demo-tool-status-bg"]
author: agent
track: A
---

# c464-fix-package-tui-diff-sbs-paint

> **status: full**（方案 A：SBS 关行底；unified 不动）

## Why

**背景 tint = 执行态**（`tool-*-bg`）。SBS 整行红/绿底与执行态抢语义。pi Diff 仅 unified + fg；本变更只修 SBS。

## Purpose

`render_side_by_side` 默认不套 `added_line_bg` / `removed_line_bg`；保留 fg + gutter。Unified 行底/词级保持现状。

## What Changes

1. SBS 路径跳过 `paint_kind_line_bg`。
2. DESIGN / `diff-block.md`：写明 SBS vs unified 的 bg 分叉。
3. harness：SBS + 带行底的 theme → 输出无行底 SGR；unified 仍有行底。

## Capabilities

- `package-tui-diff`

## Out of scope

- Unified 重做
- Session Tree（c456）
- 强制产品默认 unified
