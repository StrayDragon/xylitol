---
change_id: c1130-add-app-tui-dollar-skill
title: "产品 TUI：多 $skill 高亮 · 提交注入 SKILL.md"
status: purpose-draft
priority: 1130
apply_band: P3-feature
depends_on: ["c1085-update-agent-skills-runtime"]
author: agent
track: R
wave: editor-ux
domain: app-tui
---

# c1130-add-app-tui-dollar-skill

## Why

刻意不做 pi `/skill:name` 与逐 skill 色块。产品用内联多 `$name`；用户需在**同一条用户消息**里看见引用被识别（紫色高亮），同时模型侧拿到 SKILL.md 正文——不靠系统行、`/session` 或 `/status skills`。

## Purpose

1. `$` 补全（真实 c1085 目录）；Tab 插入 `$name`。
2. 提交：解析全部 `$name` → **读对应 SKILL.md 并注入**模型上下文（可多引用）；未知名可测失败/提示策略升格时钉。
3. Scrollback：**仅在用户消息文本内**用特殊色（如紫）高亮 `$name`；**MUST NOT** 另加系统消息行、N 个 SkillInvocation 色块、footer 计数。
4. 验收：断言 **SKILL.md 内容进入模型侧消息 / 发生 read**；**MUST NOT** 以 TUI 像素或 dump 文案作主门禁（高亮可有轻量组件测，非产品验收 SSOT）。

## What Changes（升格 full 时）

- 产品 `CompletionSource` + 提交 expand/inject 路径
- 用户消息渲染：`$skill` token 高亮（DESIGN token）
- 单测/集成：注入正文或 read 可观测（agent/history/投影层）
- delta：`app-tui-input` · `agent-prompt`（或 runtime）；A10

## Capabilities

- `app-tui-input`（modify）
- `agent-prompt` / runtime（引用）

## Out of scope

- `/skill:name`
- `/session` Skills 段、`/status skills`
- 系统消息「已加载 skills」行
- prompt template `$` 混用

## Ethics

- risk_level: medium
- prohibited_actions: 未信任读入 project SKILL.md；用 TUI 元素替代注入断言
- required_evidence: 提交含 `$demo` 后模型侧含 SKILL.md 正文（或等价 read 记录）；多 `$` 均注入；高亮非主门禁
- escalation_policy: 呈现已钉 A10；注入失败策略（硬错 vs 透传 $token）升格前拍板

## Depends

- `c1085-update-agent-skills-runtime`
