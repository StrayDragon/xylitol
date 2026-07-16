---
change_id: c1085-update-agent-skills-runtime
title: "Skills 运行时：Trust 目录 · system 注入 · 重载"
status: draft
priority: 1085
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: agent
ethics:
  risk_level: medium
  prohibited_actions:
    - 未信任项目加载 project skills 进 system / 目录
    - 把 prompt 模板冒充 skill
    - footer/status 塞 skills:N；pi 式 N 色块；用 /session 或 /status skills 作 skill 观测面（A10）
  required_evidence:
    - 单测：trusted 时 system 含 available_skills / skill 名；untrusted 跳过项目 skill
    - 单测：reload_skills 后目录与 system 更新且历史条数不变
    - MUST NOT 以 TUI dump/harness 像素或 /session Skills 段作为本变更验收
  escalation_policy: 呈现与 $ 提交注入见 A10 / c1130；本变更只钉运行时目录
---

# c1085-update-agent-skills-runtime

## Why

Loader 已发现 skills，`build_system_prompt` 能渲染 `<available_skills>`，但 bootstrap **未**把 `get_skills()` 写入 `SystemPromptOpts`；reload 路径也不更新 skills。`$` 提交注入与用户消息高亮属 c1130；本变更只钉**目录就绪**。

## Purpose

1. Trust 语义下装配 user/project skills → system prompt + 可查询目录（供 c1130 /reload）。
2. `reload_skills`（或等价）报告 names/count/diags；不碰 session 历史。
3. 对齐 A10：**不做** `/session` Skills 段、**不做** `/status skills`、**不做**系统消息行观测面。
4. 验收：断言 system / loader 目录（及日后 c1130 的 SKILL.md 注入），**不**验 TUI 元素。

## What Changes

- bootstrap：`get_skills()` → `prompt_opts.skills`（Trust 同 temp-cwd）
- Agent/Driver：`apply_skills`（或扩展 apply_prompt_resources）+ 可查询已加载名
- `app/core`：`reload_skills` / `discovered_skills`（镜像 c1100/c1095）
- delta：`runtime-resource-discovery` · `agent-prompt` · `agent-runtime`

## Capabilities

- `runtime-resource-discovery`（modify）
- `agent-prompt`（modify）
- `agent-runtime`（modify）

## Out of scope

- `$skill` 补全、提交读 SKILL.md 注入、用户消息紫色高亮（c1130）
- `/session` / `/status skills` skill 清单
- 启动 header（c1135）
- pi `/skill:` / SkillInvocation 色块（A10）

## Impact

- 解锁 c1120 skills 半边与 c1130 数据源

## Depends

- []

## Downstream

- `c1120-add-app-tui-reload-slash`
- `c1130-add-app-tui-dollar-skill`
- `c1135-add-app-tui-startup-header`
