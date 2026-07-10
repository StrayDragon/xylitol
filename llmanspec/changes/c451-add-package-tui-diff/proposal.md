---
change_id: c451-add-package-tui-diff
title: "package-tui-diff：可复用 Diff 组件（含 word-level）"
status: purpose-draft
priority: 451
depends_on: []
author: agent
track: A
---

# c451-add-package-tui-diff

> **status: purpose-draft**

## Why

edit 工具已产出 `display_diff`；产品 transcript 与 `agent_demo` 都需要统一、复制友好的 diff 渲染。准通用实现不得塞进 `src/app/tui/`。

## Purpose

在 `packages/xylitol-tui` 新增可复用 Diff 组件：行级 +/-/context + word-level intra-line；宽屏可切 left-right；CJK/emoji 宽度正确；`agent_demo` 可展开 edit 块验证。

## What Changes（意向）

1. 新 capability `package-tui-diff`；依赖 `similar`（包内，零引用主 crate）。
2. API：输入 unified / display_diff 文本或 structured hunks → `Vec<String>` ANSI。
3. Tokens 由闭包/theme 注入（added/removed/context/gutter/strong）；无 Unicode 表线。
4. 宽 ≥ 阈值（DESIGN 定）时可选 side-by-side；否则 unified。
5. 五层 harness 覆盖：宽度、CJK、intra-line、折叠上下文。
6. `agent_demo` 增加可展开 Diff 块（条件展示）。

## Capabilities

- `package-tui-diff`（新建）

## Out of scope

- 旧 `diff-review` 审批流（另清理）
- 产品面 `src/app/tui` 接线（c470）
- syntect 高亮（c452）

## Soft depends

- UX 规则优先读 c449 `design/diff-block.md`（可并行）
