---
change_id: c1480-add-otel-genai-langfuse
title: GenAI / Langfuse 属性映射（fastrace → Langfuse）
status: purpose-draft
priority: 1480
depends_on:
  - c1475-add-otel-export
author: agent
---

# c1480-add-otel-genai-langfuse

## Why

`c1475` 只解决「OTLP 管子通」。要在 Langfuse UI 里稳定看到 **generation / tool / session / 用量**，需要 GenAI semantic conventions 与 `langfuse.*` 属性（observation type、input/output 档位、session_id 等）。

## What Changes（意向）

- 在既有 fastrace span 上补充 / 规范化属性：
  - `provider.request` → generation（model、usage、可选截断 input/output）
  - `react.turn` / agent 根 → span/agent + `langfuse.session.id`
  - `tool.execute` → tool observation（name、id、status）
- Baggage / 根属性传播 session、environment、version（Langfuse 过滤依赖）
- 敏感载荷档位：默认元数据；显式档才带 observation input/output
- 文档：roadmap M2 兑现说明

## Out of scope

- 新 OTLP 装配（归 c1475）
- 自研 Inspect UI
- Langfuse Prompt / Score / Experiment 全套（可后续）

## Capabilities（预计）

- `infra-otel`（扩展）
- 可能触及 `infra-provider-trace` / `agent-runtime` obs 辅助

## Status

**purpose-draft only** — apply 前须 promote（tasks + live specs + attach）。依赖 `c1475-add-otel-export` 归档或本分支已落地其行为。
