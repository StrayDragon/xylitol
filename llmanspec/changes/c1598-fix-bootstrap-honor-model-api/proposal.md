---
change_id: c1598-fix-bootstrap-honor-model-api
title: Bootstrap 注册模型时 MUST 尊重 YAML `models.*.api`
status: purpose-draft
priority: 1598
depends_on: []
author: agent
note: "2026-07-24: 修复代码已合入工作区并随讨论 commit；完整 SDD promote（live specs attach）仍可后补。"
---

# c1598-fix-bootstrap-honor-model-api

## Discussion context

见 [`../DEFERRED-responses-tool-batch-CONTEXT.md`](../DEFERRED-responses-tool-batch-CONTEXT.md)。

关键证据：开发配置字面 Completions，Langfuse `llm.request.api=openai-responses`（bootstrap 丢 `entry.api` → `default_for(OpenAi)`）。

## Why

仓库 `.xylitol/config.yaml` 曾为模型写 `api: openai-completions`，但 `resolve_assembly` 注册 `XyModelMeta` 时把 `config.api` **写死为 `None`**。

`resolve_adapter_kind` 在 `api` 缺失时走 `AdapterKind::default_for(OpenAi) → OpenAiResponses`。

证据（session `59832e27-…` / Ornith）：Langfuse `api=openai-responses`，与 YAML 字面不一致。

## Decisions

1. Bootstrap MUST 传入 `entry.api`；同步 `XyModelMeta.api` / `provider`。
2. 缺省 `api` → 仍 Responses（不借修 bug 改默认）。
3. 顺带对齐 `manifest.rs` / `resolve_model_meta`。
4. 产品弃 Completions 默认见 `c1600`。

## What landed（本提交）

- `bootstrap.rs` + 单测 `yaml_model_api_is_honored_in_registry_config`
- `manifest.rs` / `AppConfig::resolve_model_meta`

## Non-Goals

- 改默认 dialect（`c1600`）；删 Completions；改 tool_batch

## Status

**purpose-draft + code landed** — 行为已修；合约 BDD 可后补 promote。

## Ethics

- risk_level: medium（显式 completions 会真走 Completions）
- required_evidence: Langfuse `api` 与 YAML 一致
