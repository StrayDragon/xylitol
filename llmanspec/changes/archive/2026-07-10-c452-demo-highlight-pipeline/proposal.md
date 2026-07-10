---
change_id: c452-demo-highlight-pipeline
title: "agent_demo 接入真实代码高亮管线（与产品面同库）"
status: full
priority: 452
depends_on: []
author: agent
track: A
---

# c452-demo-highlight-pipeline

## Why

包 Markdown 已有 `highlight_code` 回调；产品面与 demo 必须用**同一套真实高亮库**验证，禁止假 highlighter。

## Purpose

在 `agent_demo`（及后续 `src/app/tui`）注入真实 syntect 实现 `MarkdownTheme.highlight_code`；包默认依赖仍不绑 syntect（optional feature）。

## What Changes

1. `xylitol-tui` optional feature `highlight`：`syntect` + `two-face`；模块 `highlight::syntect_bridge`。
2. `agent_demo` 助手消息走 `Markdown` + 真实 `highlight_code`；seed 含 fenced ```rust 样例。
3. 安全上限（字节/行数）；主题 Catppuccin Mocha（对齐 DESIGN 暗色意向）。
4. 单测：fence 输出含 ANSI；超限回退纯文本。

## Capabilities

- `package-tui-editor`（文档说明：高亮经回调注入）或新建说明挂 `package-tui-diff` 旁——本变更以 `package-tui-editor` modify 记「高亮注入约定」。

## Out of scope

- 把 syntect 打进 `xylitol-tui` **默认**依赖
- 产品面 `src/app/tui` 完整接线（仅预留同 feature）
