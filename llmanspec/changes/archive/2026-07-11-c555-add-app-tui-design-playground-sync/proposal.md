---
change_id: c555-add-app-tui-design-playground-sync
title: "app-tui-design-playground：token 同步与 Markdown 槽对齐"
status: full
priority: 555
depends_on: []
author: agent
track: P
---

# c555-add-app-tui-design-playground-sync

## Why

DESIGN playground 已能给人审色，但 token 与 `DESIGN.md`、Markdown 槽与现行「粗体/斜体纯 SGR」易漂移；轨 P 需要一条**轻量合约**：同步脚本为 SSOT 入口，槽位示意对齐 `design/markdown.md`，且明确 Agent 默认忽略 playground。

## Purpose

新建 capability `app-tui-design-playground`：人类预览壳的同步与示意对齐（非运行时）。

## What Changes

1. 新建 capability 与精简 requirements。
2. `sync_tokens.py` MUST 从 `DESIGN.md` frontmatter 生成 `tokens.css` / `tokens.js`。
3. playground Markdown 槽示意 MUST 反映无井号标题、链接 `text (url)`、粗体/斜体无星号（可用（加粗）标注）。
4. 更新 `design/AGENTS.md` / playground README 一句（若缺口）。

## Capabilities

- `app-tui-design-playground`（新建）

## Impact

- `src/app/tui/design/playground/**`
- `src/app/tui/design/AGENTS.md`（指针）
- **不改** `packages/xylitol-tui` 运行时（除非仅文档交叉引用）

## Out of scope

- 产品 host / Rust TUI
- 把 playground 当实现真值源

## Ethics

- risk_level: low
- prohibited_actions: 不在 playground 堆 Rust/host 接线
- required_evidence: sync 脚本可跑；validate 通过
