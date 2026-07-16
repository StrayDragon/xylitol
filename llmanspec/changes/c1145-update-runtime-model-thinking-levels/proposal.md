---
change_id: c1145-update-runtime-model-thinking-levels
title: "模型配置：thinking levels（high / xhigh / max 等）"
status: purpose-draft
priority: 1145
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: thinking
domain: runtime
---

# c1145-update-runtime-model-thinking-levels

## Why

产品 thinking UX 需要「特定模型配置项」声明可用 level（如 high / xhigh / max），不能全局硬编码。协议已有 `SetThinkingLevel` / `ThinkingLevel`；缺模型侧配置与解析。

## Purpose

在模型/registry 配置中声明该模型支持的 thinking levels 与默认值；无效 level 拒绝或降级（钉一种）；供 footer 回退显示与 c1150 边框共用同一枚举源。

## What Changes（升格 full 时）

- 配置 schema：`thinking_levels` / `default_thinking_level`（名称升格钉）
- registry / model 解析
- Driver/protocol 校验
- delta：`runtime-model-registry` · `runtime-config` · 可选 `protocol-app`

## Capabilities

- `runtime-model-registry`（modify）
- `runtime-config`（modify）

## Out of scope

- TUI 边框绘制（c1140/c1150）
- `/settings` UI

## Ethics

- risk_level: medium
- prohibited_actions: 对不支持 thinking 的模型伪造 level 调用
- required_evidence: 配置解析测；不支持模型拒绝测
- escalation_policy: 与 Anthropic/OpenAI 专属 thinking API 映射冲突时升级

## Depends

- []

## Downstream

- `c1150-add-app-tui-thinking-level`
