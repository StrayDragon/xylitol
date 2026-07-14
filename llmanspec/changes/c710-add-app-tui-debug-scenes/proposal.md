---
change_id: c710-add-app-tui-debug-scenes
title: "产品 /debug:<scene>：可复现手测场景（Fake + 固定 session）"
status: purpose-draft
priority: 710
depends_on: ["c685-update-app-tui-session-tree-slot-help"]
author: agent
track: A
---

# c710-add-app-tui-debug-scenes

## Why

手测 Fake / 清 session 后常卡在「Model fake not found」或「session not found」；E2E 有隔离 config，日常 `cargo run -- --trust --tui --model fake` 没有。需要**一键灌场景**，减少对真云与脏 HOME 的依赖。

## Purpose

提供 `/debug:<scene>`（或等价）在 trust 下装载可复现 fixture：保证 Fake 模型可用、session 存在且含最少对话树，便于验树槽 Search/Help、travel、fork 等。**非** Settings UI；仅 debug 入口。

## What Changes（实现时）

1. 场景目录或内置表：`tree-empty` / `tree-branch` / `tree-labeled` 等（名称后定）。
2. 装载：必要时写入临时/隔离 session；注册或解析到 `fake`；开 TUI 后树可双 Esc。
3. 文档：人类手测路径改为优先 `/debug:…`；与 `tests/tui_e2e` Fake 隔离策略对齐。
4. 升格 full 时补 `app-tui-input` / `app-tui-commands` delta。

## Out of scope

- 真 LLM debug；改生产 session 默认行为；完整 fixture 编辑器

## Ethics

- risk_level: low
- prohibited_actions: debug 场景默认真扣费 API；写坏用户默认 sessions 目录而不隔离
- required_evidence: harness 或文档化命令可一键开树

## Depends

- **c685**（树槽头行已有）；与 **c690** / **c705** 互补（手测入口 vs E2E 闸）
