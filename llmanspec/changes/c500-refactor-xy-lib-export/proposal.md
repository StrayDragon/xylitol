---
change_id: c500-refactor-xy-lib-export
title: "精选库导出面与 Xy* 契约收敛"
status: proposed
priority: 500
depends_on: []
author: agent
track: A
---

# c500-refactor-xy-lib-export

## Why

已定调：近期要精选 `pub use`；`Xy*` 只标库入口契约，不是全局品牌。现状混用（有死包装、有叶子类型乱加前缀、`lib.rs` 整模导出），阻碍稳定 API 与后续重构。

依据：根/`src` `AGENTS.md`（2026-07-11）、`_NOTE.md`。

## What Changes

1. 在 `src/lib.rs` 建立**精选 `pub use`**（至少 `XyModel`/`XyTool`/`XySessionStore`/`XyEvent`/`XyChunk` 及文档化配套类型）。
2. 修订 `architecture.ar06`：Xy 前缀仅约束精选公开契约，不强制所有「项目公共类型」。
3. 删除死包装：`SessionIO`、`PermissionGate`（改为直接持有 `Arc<dyn …>`）、`LlmMessageConverter`（零 impl）、`XyToolDefinition`（无消费者）。
4. 清理明显死字段（如 `InProcessDriver.model_builder` 若仍未读）。
5. 同步 AGENTS / 简短导出文档注释（不写第二份百科）。

## Capabilities

- `architecture`（modify ar06 + add ar09）
- `layer-architecture`（add la21）

## Impact

- `src/lib.rs`、`src/agent/session/{io,permission,mod}.rs`、`src/domain/message.rs`、`src/agent/tools/definition.rs`、`src/app/core/driver.rs`
- 调用点一次性改完，无 shim

## Out of scope

- Provider 双路径折叠（c505）
- domain 去 JsonSchema（c510）
- MCP（c515）
- XyEvent 扩展策略实现细节以外的大改（c520）
- 产品 TUI
