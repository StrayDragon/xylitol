---
change_id: c449-split-app-tui-design-docs
title: "拆分 app TUI DESIGN.md 为 design/* 组件文档"
status: full
priority: 449
depends_on: []
author: agent
track: D
---

# c449-split-app-tui-design-docs

## Why

产品 TUI 视觉 SSOT 挤在单文件 `DESIGN.md`；第四次重做需要按组件固定 UX，并按中文 + YAML frontmatter tokens 体系化，避免实现时口头漂移。

## Purpose

把 `src/app/tui/DESIGN.md` 拆成主索引 + `design/*.md` 组件文档，锁定布局/键位/diff/可展开等 UX，供轨 A demo 与轨 B 产品面共用。

## What Changes

1. 主 `DESIGN.md`：Overview / Colors / Typography / Layout / Elevation / Shapes / 组件索引 / Do's and Don'ts；frontmatter 保留全局 tokens。
2. 新建 `src/app/tui/design/` 组件与后置能力文档（见 tasks）。
3. 对齐已决议键位：Esc=abort（流中）；Ctrl+C 清输入 / 空再退；Alt+Enter=follow-up；流中 Enter=steer；双 Esc=会话树。
4. 不写产品/包实现代码。

## Capabilities

- `app-tui-chrome`（modify：设计文档 SSOT 指针）

## Out of scope

- 实现任何渲染逻辑
- 修改 `packages/xylitol-tui`
