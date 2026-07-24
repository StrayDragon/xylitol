---
change_id: c1598-fix-bootstrap-honor-model-api
title: Bootstrap 注册模型时 MUST 尊重 YAML `models.*.api`
status: in-progress
priority: 1598
depends_on: []
author: agent
branch: feat/c1598-c1600-model-api
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
note: 代码已于主线合入；本分支补 live specs 并归档。
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1598-fix-bootstrap-honor-model-api

## Why

`resolve_assembly` 注册 `XyModelMeta` 时曾把 `config.api` 写死为 `None`，YAML `models.*.api` 形同未接线。

## Decisions

1. Bootstrap MUST 传入 `entry.api`；同步 `XyModelMeta.api` / `provider`。
2. 缺省 `api` → 仍走 `AdapterKind::default_for`（OpenAI→Responses），不借修 bug 改默认。
3. 顺带对齐 `manifest.rs` / `resolve_model_meta` 的 `config.api` 同步。
4. 产品弃 Completions 默认见 `c1600`。

## What landed

- `bootstrap.rs` + 单测 `yaml_model_api_is_honored_in_registry_config`
- `manifest.rs` / `AppConfig::resolve_model_meta`

## Non-Goals

改默认 dialect（`c1600`）；删 Completions；改 tool_batch。

## Status

**verify → finalize** on `feat/c1598-c1600-model-api`。

## Ethics

- risk_level: medium（显式 completions 会真走 Completions）
- required_evidence: 单测 Langfuse/registry `api` 与 YAML 一致
