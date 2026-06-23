---
id: c225-refactor-domain-layering
title: "引入 core 基础层承载领域核心类型 — 消除 infra→agent 反向依赖"
depends_on: []
---

## Why

架构审计发现 **6 个 infra 文件反向 import `crate::agent::*`**,违反文档声明的"依赖只能向下流动、infra 永不 import agent/interface"分层规则:

| infra 文件 | 依赖的 agent 抽象 |
|---|---|
| `infra/config/types.rs` | `agent::model::{ModelConfig, ModelKind, ModelMeta}`、`agent::profile::ResolvedProfile`、`agent::registry` |
| `infra/session/compaction.rs` | `agent::message::AgentMessage`、`agent::traits::XyModel`、`agent::types::XyChunk` |
| `infra/session/manager.rs` | 返回 `Vec<AgentMessage>`、接收 `&dyn XyModel` |
| `infra/event/lifecycle.rs` | 事件 payload 携带 `AgentMessage` |
| `infra/skills/mcp.rs` | `impl XyTool for McpToolAdapter` |
| `infra/config/loader.rs` | `ModelKind` |

**根因**:领域核心类型(`AgentMessage`、`XyModel`、`XyTool`、`ModelKind`、`XyChunk`)是整个系统的基础词汇,却被归入中层 `agent/`。底层 `infra/`(持久化/配置/事件/skills)天然需要它们,于是依赖箭头只能反向指向上层。`config/types.rs:57` 的代码注释("Split from agent-level ModelConfig")已暴露该切分尴尬,但未推进到底。

每新增一个 infra 功能都会加深这层泄漏,必须在功能扩张前根治。

## What Changes

1. **新增 `src/core/` 基础层**,承载领域核心词汇类型:
   - `AgentMessage` / `AgentPart`(来自 `agent/message.rs`)
   - `XyModel` / `XyTool` trait(来自 `agent/traits.rs`)
   - `ModelKind` / `ModelConfig` / `ModelMeta`(来自 `agent/model.rs`、`agent/types.rs`)
   - `XyChunk` / `XyUsage` 等流/用量类型
2. **修订分层规则**为:`interface/ → agent/ → core/ ← infra/`(agent 与 infra 都依赖 core;infra 不再依赖 agent)。
3. **迁移 6 个违规 infra 文件**改为 `crate::core::*`。
4. **更新所有调用点** import 路径(`crate::agent::message` → `crate::core::message` 等)。项目规则"不留后向兼容 shim",一次性改完。
5. **新增架构守卫测试**:断言 `src/infra` 下无 `crate::agent` import(grep 实现),防止回归。

## Capabilities

- 新增 capability **`layer-architecture`**:规约向下依赖不变量与 core 层职责。
- `agent-runtime` / `provider-integration` / `model-registry` 的类型物理位置变更,但其 requirement 语义不变(纯物理迁移,不改行为规约)。

## Impact

- **规模**:大型机械重构,触及所有引用被迁移类型的文件(预计 30+ 文件)。
- **行为**:零行为变更;类型移动 + import 改写。
- **安全网**:454 lib 测试 + 83 BDD 场景全绿作为回归保障。
- **风险**:物理位置移动本身低风险(编译器强制 import 一致),主要工作量在机械改写。
- **不做**:不改变任何 public API 行为、不调整 ReAct 循环逻辑、不动 session/compaction 编排(那是 c230 的范围)。
