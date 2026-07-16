---
change_id: c1165-add-runtime-thinking-level-map
title: "模型配置：thinking level → provider effort 映射（用户表）"
status: draft
priority: 1165
apply_band: P3-feature
depends_on:
  - c1145-update-runtime-model-thinking-levels
author: agent
track: R
wave: thinking
domain: runtime
ethics:
  risk_level: medium
  prohibited_actions:
    - 无映射时伪造 provider 专属字段导致计费/行为异常
    - 把密钥写进 map
    - 未知 level 键静默忽略（加载须失败）
  required_evidence:
    - resolve 单测：洞、null、覆盖默认
    - bridge 请求体断言：off/medium/high（及自定义 map）
  escalation_policy: 与某厂商官方 SDK 字段冲突时先钉 design，再改代码
---

# c1165-add-runtime-thinking-level-map

## Why

c1145 已提供域枚举与 per-model 可用 levels；c1150 已把 Shift+Tab 接到 `Driver`。真正发往 Anthropic / OpenAI-compat 的 effort、thinking budget、reasoning 参数仍缺映射（对齐 pi `thinkingLevelMap`）。

## Purpose

1. 模型配置可声明 **level → provider 原生值** 的映射表（可含洞 / `null` 表示该档不发）
2. 发请求前经表解析；缺表或缺键时用 adapter 内置默认（见 design）
3. **用户配置优先于** 内置表；未知 map 键名 MUST 使配置加载失败
4. ReAct / `XyModel::generate_stream` MUST 携带当前 `ThinkingLevel`（及 map/budgets），使 UI cycle 影响下一轮请求体

## What Changes

- `ModelEntry.thinking_level_map`（YAML snake_case）；填入 `XyModelMeta`
- domain `ThinkingLevelMap`（或等价）+ `resolve_thinking_for_request`
- `XyGenerateOptions` 传入 `generate_stream`（默认保持今日无 thinking 字段行为兼容测试）
- bridge：OpenAI Completions `reasoning_effort`；OpenAI Responses `reasoning.effort`；Anthropic `thinking` budget 路径
- Settings `thinking_budgets` 接入 Anthropic budget（缺省用内置 token 表）
- 单测 + 请求体断言
- delta：`runtime-config` · `runtime-model-registry` · `package-ai-bridge`

## Capabilities

- `runtime-config`（modify）
- `runtime-model-registry`（modify）
- `package-ai-bridge`（modify）

## Out of scope

- TUI 边框 / footer（c1150 已归档）
- 再扩 `ThinkingLevel` 枚举
- Anthropic adaptive / `output_config.effort` 全路径（可后置；本变更钉 budget）
- OpenRouter 专属 nested `reasoning` 方言（可后置）

## Impact

- Shift+Tab / `set_thinking_level` 后下一轮 LLM 请求体反映对应 effort/budget（或 off 省略）

## Depends

- c1145（已归档）
