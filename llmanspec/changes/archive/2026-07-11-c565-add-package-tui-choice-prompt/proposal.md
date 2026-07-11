---
change_id: c565-add-package-tui-choice-prompt
title: "package-tui-choice-prompt：Ask 单选/多选 + Other 自由输入 + 多题 Tab"
status: full
priority: 565
depends_on: []
author: agent
track: P
---

# c565-add-package-tui-choice-prompt

## Why

Agent 需要 Cursor AskQuestion 类交互：一题内单选或多选，并可自由输入；多题时用 Tab 切换。现有 `SelectList` 仅单选且无 Other；pi `questionnaire` 在 coding-agent 扩展里，不在 tui 包。轨 P 应先提供可测通用组件 + demo/playground，产品工具接线留轨 B。

## Purpose

在 `xylitol-tui` 新增 `ChoicePrompt`（名称可微调），覆盖单选/多选、Other 自由输入、多题 Tab；agent_demo 内联槽验证；DESIGN playground 可审色与态。

## What Changes

1. 新组件 `ChoicePrompt`（`packages/xylitol-tui`）：每题 `Single` | `Multi`；可选 `allow_other`；多题 Tab + Submit。
2. 键位：↑↓ 移动；Space 多选切换；Enter 确认/提交；**Tab 聚焦 Other 输入**；Esc 取消。
3. `agent_demo` plates：`ask-single` / `ask-multi` / `ask-tabs`（多题）；结果写入 transcript。
4. playground 新槽 **Ask**（示意单选/多选/多题 Tab / Other）。
5. 单测 + harness 覆盖空态、Other、多选提交、多题切换。

## Capabilities

- `package-tui-choice-prompt`（新增）

## Impact

- `packages/xylitol-tui/src/components/choice_prompt.rs`（新）+ `mod` / `lib` 导出
- `packages/xylitol-tui/examples/agent_demo.rs`、tests
- `src/app/tui/design/playground/`（Ask 槽）

## Out of scope

- 产品 `AskQuestion` 工具 / Driver 接线（轨 B）
- Overlay 浮层确认（确认类继续内联槽）
- 把 pi questionnaire 整文件移植

## Ethics

- risk_level: low
- prohibited_actions: 不引入主 crate agent/session 类型进包；不实现产品工具协议
- required_evidence: `llman sdd validate` 通过；`cargo test -p xylitol-tui` 相关绿；demo plate 可手验
