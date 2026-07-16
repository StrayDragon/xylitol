---
change_id: c1145-update-runtime-model-thinking-levels
title: "模型配置：thinking levels（high / xhigh / max 等）"
status: draft
priority: 1145
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: thinking
domain: runtime
ethics:
  risk_level: medium
  prohibited_actions:
    - 对 thinking:false / 空 levels 的模型伪造可用 level 调用
    - 在本变更接线产品 TUI 边框或 footer（属 c1150）
    - 把 xylitol-tui ThinkingBorderLevel 依赖塞进 domain
  required_evidence:
    - 配置解析：ModelEntry.thinking_levels → XyModelMeta.thinking_levels
    - 不支持 level 拒绝；换模 clamp
    - ThinkingLevel 含 xhigh/max 的序列化/解析测
  escalation_policy: 与 Anthropic/OpenAI 专属 thinking API 映射冲突时升级；本变更只做枚举+配置+clamp，不改 provider body
---

# c1145-update-runtime-model-thinking-levels

## Why

产品 thinking UX 需要「特定模型配置项」声明可用 level（如 high / xhigh / max），不能全局硬编码。`XyModelMeta.thinking_levels` 与 Settings `default_thinking_level` 已有字段槽，但 `ModelEntry` 未暴露、`resolve_model_meta` 恒空、`ThinkingLevel` 缺 xhigh/max、`set_thinking_level` 未按模型列表校验。

## Purpose

在模型/registry 配置中声明该模型支持的 thinking levels 与默认值；无效 level **拒绝**；换模时 **clamp** 到当前模型支持集（空集且 `thinking:false` → 仅 Off）；供 footer 回退与 c1150 边框共用同一 `domain::ThinkingLevel` 源。

**本变更不定义 TUI 反馈形态**：产品静默（边框+footer，无 transcript 刷屏）与 agent_demo 上行验证，由 **c1150 purpose** 钉死。

## What Changes

- `domain::ThinkingLevel`：补齐 `xhigh` / `max`（与 pi / 包 `ThinkingBorderLevel` 名对齐）；`parse` / `as_str` / serde
- `ModelEntry`：可选 `thinking_levels: Vec<String>`；`resolve_model_meta` 填入 `XyModelMeta`
  - 缺省：`thinking:true` → 标准档 `off|minimal|low|medium|high`；`thinking:false` → 空或仅 `off`
  - 显式列表：以配置为准（可含洞，如仅 `high`+`max` 无 `xhigh`）
- `ModelManager` / Driver：`set_thinking_level` 不在支持集 → 拒绝（不改当前值）；`select_model` 后 clamp
- Settings `default_thinking_level`：选模/启动时若合法则采用，否则 clamp
- 可选：`cycle_thinking_level`（仅在支持集内前进）供 c1150 调用——若本变更不加，c1150 自循环列表亦可
- delta：`runtime-model-registry` · `runtime-config`

## Capabilities

- `runtime-model-registry`（modify）
- `runtime-config`（modify）

## Out of scope

- TUI 边框 / footer / Shift+Tab（c1140 已归档包；c1150 产品接线）
- `/settings` UI
- provider 请求体 effort/thinking 参数映射 → **`c1165-add-runtime-thinking-level-map`**（deferred；用户配置表，对齐 pi `thinkingLevelMap`）
- agent_demo 行为

## Impact

- 配置可声明 per-model levels；协议 `SetThinkingLevel` 有真实校验源；c1150 可静默 cycle

## Downstream

- `c1150-add-app-tui-thinking-level`
- `c1165-add-runtime-thinking-level-map`（deferred）
