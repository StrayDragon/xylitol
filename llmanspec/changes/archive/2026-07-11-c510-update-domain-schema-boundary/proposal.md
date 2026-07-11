---
change_id: c510-update-domain-schema-boundary
title: "domain 与配置边界：JsonSchema 下沉"
status: proposed
priority: 510
depends_on: ["c500-refactor-xy-lib-export"]
author: agent
track: A
---

# c510-update-domain-schema-boundary

## Why

精选 `pub use` 后，domain 再 `derive(JsonSchema)` 会把 schemars 拖进库依赖图。schema 生成属于配置/工具链关注点，应留在 infra/config（或 settings）。

## What Changes

1. 从 `src/domain` 移除所有 `JsonSchema` derive 与 schemars import。
2. 需要 schema 的类型在 `infra/config` / `infra/settings` 保留或新增包装 DTO，并用 `From` 映射到 domain（若仍有对应领域类型）。
3. 更新 `architecture` / `runtime-config` 合约。

## Capabilities

- `architecture`（add ar10）
- `runtime-config`（add rc8）

## Impact

- `src/domain/model.rs`、`src/domain/compaction_config.rs` 等
- `src/infra/config/types.rs`、`src/infra/settings/types.rs`

## Out of scope

- 改变配置文件用户可见字段名（除非映射被迫）
- MCP（c515）
