---
change_id: c1130-add-app-tui-dollar-skill
title: "产品 TUI：多 $skill 高亮 · 提交注入 SKILL.md"
status: applied
priority: 1130
apply_band: P3-feature
depends_on: ["c1085-update-agent-skills-runtime"]
author: agent
track: R
wave: editor-ux
domain: app-tui
ethics:
  risk_level: medium
  prohibited_actions:
    - 未信任读入项目 SKILL.md
    - 用 TUI 像素 /session dump 替代注入断言
    - N 个 SkillInvocation 色块或系统「已加载」行（A10）
  required_evidence:
    - 提交含 $demo 后发往模型的 user 文本含 SKILL.md 正文
    - 历史/scrollback 仍为原始 $token（可高亮）
    - 多 $ 均注入；未知 $ 透传
  escalation_policy: 注入失败（读盘错）→ 透传 $token + log warn；不拖垮 turn
---

# c1130-add-app-tui-dollar-skill

## Why

刻意不做 pi `/skill:name` 与逐 skill 色块。产品用内联多 `$name`；用户需在同一条用户消息里看见引用（紫高亮），模型侧拿到 SKILL.md 正文。

## Purpose

1. `$` 补全（c1085 目录）；Tab 插入 `$name`。
2. 提交：解析 `$name` → 读 SKILL.md → **仅在发模型前**展开进 user 文本；会话历史与 scrollback 保留原始 `$`。
3. Scrollback：用户消息内 `skill-ref` 高亮；无系统行 / N 色块 / footer 计数。

## What Changes

- `agent/prompt/skill_expand`：parse + read + expand（catalog 闸 = Trust）
- `run_react_loop`：`project_for_llm` 后展开 user 行
- 产品 `DollarSkillSource` + scrollback 高亮
- delta：`app-tui-input` · `agent-prompt`

## Capabilities

- `app-tui-input`（modify）
- `agent-prompt`（modify）

## Out of scope

- `/skill:name`、`/session` Skills、系统「已加载」行
- prompt template `$` 混用
- `.agents/skills` 多源（A11）

## Impact

- 解锁真实 `$skill` 产品路径；对齐 pi expand 语义、A10 呈现
