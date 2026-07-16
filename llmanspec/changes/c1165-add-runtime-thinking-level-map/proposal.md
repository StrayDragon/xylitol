---
change_id: c1165-add-runtime-thinking-level-map
title: "模型配置：thinking level → provider effort 映射（用户表）"
status: purpose-draft
priority: 1165
apply_band: P9-deferred
depends_on:
  - c1145-update-runtime-model-thinking-levels
author: agent
track: R
wave: thinking
domain: runtime
---

# c1165-add-runtime-thinking-level-map

## Why

c1145 已提供域枚举与 per-model **可用 levels**。真正发往 Anthropic / OpenAI-compat / 其它 provider 的 effort、thinking budget、reasoning 参数仍缺 **用户可配置映射**（对齐 pi `thinkingLevelMap`）。硬编码每家方言会在新 provider 增多时爆炸；等 catalog/配置表更齐后再做更省。

## Purpose（**暂缓 / deferred · 低优先级**）

本变更 **P9 暂缓**：不挡 c1150 产品 TUI（边框/footer/Shift+Tab）。升格时机建议：更多 provider/adapter 接入、或用户需要自定义 effort 字符串之后。

升格后目标（备忘，非本波合约）：

1. 模型配置可声明 **level → provider 原生值** 的映射表（可含洞 / `null` 表示该档不发）
2. 发请求前经表解析；缺表时用 adapter 内置默认（须文档化）
3. **用户配置优先于** 内置表；未知 level 不静默伪造

参考：pi `thinkingLevelMap`（`off|minimal|…|max` → string | null）。

## What Changes（升格 full 时）

- `ModelEntry`（或等价）：`thinking_level_map` / `thinkingLevelMap`（名称升格钉）
- bridge / provider 请求组装消费映射
- 解析测：洞、null、覆盖默认
- delta：`runtime-config` · `runtime-model-registry` · 相关 provider/bridge capability

## Capabilities

- `runtime-config`（modify）
- `runtime-model-registry`（modify）
- 视实现：`package-ai-bridge` / provider adapter

## Out of scope

- 本波实现（deferred）
- TUI 边框 / footer（c1150）
- 再扩 `ThinkingLevel` 枚举本身（属 c1145 已完成面）

## Ethics

- risk_level: medium
- prohibited_actions: 无映射时伪造 provider 专属字段导致计费/行为异常；把密钥写进 map
- required_evidence: 有/无 map 的请求体快照或单测（升格后）
- escalation_policy: 与某厂商官方 SDK 字段冲突时先钉 design，再改代码

## Depends

- c1145（枚举与 levels 列表就绪）

## Downstream

- 可选：产品 footer 显示「mapped effort」调试（非必须）

## Notes

- 2026-07-16：从 c1145 out-of-scope 拆出；等更多 provider 带表再升格。
