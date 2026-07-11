---
change_id: c530-add-lib-embed-api
title: "公开库嵌入面：Driver / bootstrap 出 crate 边界"
status: proposed
priority: 530
depends_on: []
author: agent
track: A2
---

# c530-add-lib-embed-api

## Why

精选 `Xy*` 已在 `lib.rs`，但多 client 装配缝（`bootstrap` / `Driver` / `composition`）仍是 `pub(crate)`。外部 crate 无法正规嵌入 xylitol 作为后端，只能 reach-in 内部模块——与「内核可作库」目标冲突。

依据：`docs/architecture/库与多客户端.md`。

## What Changes

1. 定义稳定嵌入入口（建议 `xylitol::embed` 或 feature `embed` 下 re-export）：至少 `BootstrapInput` / `bootstrap` / `Driver` / `InProcessDriver` / `BuildAgentOptions` / `McpSession`（或等价）。
2. 文档：`lib.rs` + `docs/architecture/库与多客户端.md` 标明嵌入方依赖清单；禁止把 `infra::*` 当稳定 API。
3. 可选：对非嵌入路径的深层 `pub mod` 加 `#[doc(hidden)]` 或文档警告（不一次收死可见性）。

## Capabilities

- `architecture`（add ar-embed1）
- `layer-architecture`（add la-embed1）

## Impact

- `src/lib.rs`、`src/app/mod.rs`、`src/app/core/*` 可见性
- 嵌入示例或 doc 测试（可选）

## Out of scope

- Server 改走 Driver（c535）
- 线协议对齐（c540）
- 产品 TUI 开闸
