---
change_id: c449-split-app-tui-design-docs
title: "拆分 app TUI DESIGN.md 为 design/* 组件文档"
status: purpose-draft
priority: 449
depends_on: []
author: agent
track: D
---

# c449-split-app-tui-design-docs

> **status: purpose-draft** — 仅 purpose；升格 full 后再写 specs/tasks。

## Why

产品 TUI 视觉 SSOT 挤在单文件 `DESIGN.md`；第四次重做需要按组件固定 UX，并按 `common-design-md-zh`（中文 + YAML frontmatter tokens）体系化，避免实现时口头漂移。

## Purpose（一句话）

把 `src/app/tui/DESIGN.md` 拆成主索引 + `design/*.md` 组件文档，锁定布局/键位/diff/可展开等 UX，供轨 A demo 与轨 B 产品面共用。

## What Changes（意向）

1. 主 `DESIGN.md`：Overview / Colors / Typography / Layout / Elevation / Shapes / 组件索引 / Do's and Don'ts；frontmatter 保留全局 tokens。
2. 新建 `src/app/tui/design/`：
   - `transcript.md` `expandable.md` `status.md` `editor.md` `footer.md` `overlay.md`
   - `diff-block.md`（含 unified + 宽屏 left-right 自适应、word-level intra-line、CJK）
   - `glyphs.md` `theme-tokens.md` `keybindings.md` `markdown.md` `errors.md`
   - 后置能力也先写 DESIGN 草稿：`session-tree.md` `bash-mode.md` `queue-steer.md` `trust-prompt.md` `compaction-status.md`
3. 对齐已决议键位：Esc=abort（流中）；Ctrl+C 清输入 / 空再退；Alt+Enter=follow-up；流中 Enter=steer；双 Esc=会话树。
4. 不写产品/包实现代码。

## Capabilities

- （文档）无新 llmanspec capability；升格时可挂 `app-tui-chrome` / 设计约定说明

## Impact

- `src/app/tui/DESIGN.md`、`src/app/tui/design/**`
- 更新 `src/app/tui/AGENTS.md` 指向子文档

## Out of scope

- 实现任何渲染逻辑
- 修改 `packages/xylitol-tui`

## Promote trigger

轨 A/B 任一 change 需要引用组件级 MUST 规则时升格为 full。
