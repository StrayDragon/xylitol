---
change_id: c1085-update-agent-skills-runtime
title: "Skills 运行时加载：发现→注入→可选视觉表现"
status: purpose-draft
priority: 1085
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: agent
---

# c1085-update-agent-skills-runtime

## Why

`DefaultResourceLoader` 已发现 skills，system prompt 可渲染 `<available_skills>`；CLI `resources` 可 list/info。产品仍缺：会话内激活语义、`$skill` 数据源、启动 header 清单，以及**一种用户可见的视觉表现**（形态待定）。

## Purpose

钉死 skills 作为「可加载能力包」的运行时路径（Trust 闸项目技能）；提供可订阅的加载/重载结果；视觉表现先留扩展点，apply 前再定（列表条 / header chip / status 注脚等）。

## What Changes（升格 full 时）

- 装配：Trust 后加载 user/project skills；进 system / 会话可查询目录
- 重载钩子：返回新增/移除/诊断，供 `/reload` 与 header
- 视觉：TBD（proposal 不锁 UI）；至少有一处用户可见「已加载 N skills」信号路径
- **不做** prompt templates 产品替代路径（明确用 skill）
- delta：`runtime-resource-discovery` · `agent-prompt` · 可选 `app-tui-*`

## Capabilities

- `runtime-resource-discovery`（modify）
- `agent-prompt`（modify）

## Out of scope

- `/skill:name` slash（→ `$skill-name` 见 c1130）
- prompt templates 产品 slash / header
- pi Packages / 扩展市场

## Ethics

- risk_level: medium
- prohibited_actions: 未信任项目加载 project skills；把 prompt 模板冒充 skill
- required_evidence: Trust on/off 加载差异测；重载后目录更新可测
- escalation_policy: 视觉形态未定时保持 purpose-draft，升格前用户拍板

## Depends

- []

## Downstream

- `c1120-add-app-tui-reload-slash`
- `c1130-add-app-tui-dollar-skill`
- `c1135-add-app-tui-startup-header`
