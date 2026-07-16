---
change_id: c1130-add-app-tui-dollar-skill
title: "产品 TUI：$skill-name 内联引用（非 /skill:）"
status: purpose-draft
priority: 1130
depends_on: ["c1085-update-agent-skills-runtime"]
author: agent
track: R
wave: editor-ux
domain: app-tui
---

# c1130-add-app-tui-dollar-skill

## Why

刻意不做 pi `/skill:name`。`agent_demo` 已有 `$` CompletionSource 原型（c545）；产品需接真实 skills 目录（c1085）。

## Purpose

产品 editor：`$` 触发技能名补全；Tab 插入 `$name`；提交时展开/注入技能内容（语义对齐「引用 skill」，具体 expand 时机升格钉死）。**MUST NOT** 识别 `/skill:`。

## What Changes（升格 full 时）

- 产品 `CompletionSource`（真实 skill 目录，非 demo stub）
- 提交路径：expand 或保留标记供模型（钉一种）
- harness
- delta：`app-tui-input` · `app-tui-commands`（若有）· skills

## Capabilities

- `app-tui-input`（modify）
- `runtime-resource-discovery` / `agent-prompt`（引用）

## Out of scope

- `/skill:name`
- prompt template `$` 混用

## Ethics

- risk_level: medium
- prohibited_actions: 未信任加载 project skill 内容进提交
- required_evidence: 补全列表 = 已加载 skills；未知名提示
- escalation_policy: expand-at-submit vs model-sees-$token 二选一需确认

## Depends

- `c1085-update-agent-skills-runtime`
