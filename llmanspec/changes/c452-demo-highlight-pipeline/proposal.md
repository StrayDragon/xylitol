---
change_id: c452-demo-highlight-pipeline
title: "agent_demo 接入真实代码高亮管线（与产品面同库）"
status: purpose-draft
priority: 452
depends_on: []
author: agent
track: A
---

# c452-demo-highlight-pipeline

> **status: purpose-draft**

## Why

包 Markdown 已有 `highlight_code` 回调；产品面与 demo 必须用**同一套真实高亮库**验证，禁止假 highlighter。

## Purpose

在 `agent_demo`（及后续 `src/app/tui`）注入真实 syntect（或选定库）实现 `MarkdownTheme.highlight_code`；包本身仍不绑 syntect。

## What Changes（意向）

1. 主 crate 或 demo 可共享的 highlight 适配模块（开闭：包只收回调）。
2. `agent_demo` 流式 fence 高亮验收测试。
3. 主题随暗色 token；安全上限（体积/行数）防卡死。

## Capabilities

- 可能触及 `package-tui-editor` 文档说明；产品侧未来 `app-tui-transcript`

## Out of scope

- 把 syntect 打进 `xylitol-tui` 默认依赖
