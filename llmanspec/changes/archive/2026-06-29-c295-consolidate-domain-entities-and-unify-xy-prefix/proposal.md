---
id: c295-consolidate-domain-entities-and-unify-xy-prefix
depends_on:
  - c285-refactor-domain-runtime-protocol-boundary
  - c290-restructure-app-layer
---

# c295-consolidate-domain-entities-and-unify-xy-prefix

## Why

c285 将 `core/` 拆分为 `domain/` 和 `runtime_protocol/` 后，词汇层内部仍然存在概念重复和命名风格漂移：

1. **同一领域概念多个类型**：token 使用量有 `Usage` 和 `XyUsage`；停止原因有 `StopReason` 和 `XyFinishReason`；compaction 配置有 `CompactionConfig` 和两个 `CompactionSettings`。
2. **事件体系过度分裂**：ReAct 循环输出流 (`AgentEvent`)、EventBus 生命周期事件 (`AgentLifecycleEvent`)、`EventSink` port 的临时 `LifecycleEvent`、线协议 `Event` 四者高度重叠但各自维护。
3. **`Xy*` 前缀规则不清晰**：最初是为“替换 adk-rust 的自定义抽象”打上的烙印，但后续新增的 `SessionStore`/`EventSink`/`BashExecutor` 等 runtime port 没有前缀，导致公共 API 看起来两套风格并存。

本变更先合并/清理冗余概念，再为公共对外接口统一 `Xy*` 前缀，使 `domain/` 和 `runtime_protocol/` 的命名空间一致、可预测。

## What Changes

### P0 — 合并重复领域实体

- **删除 `XyUsage`**，`agent/compaction/token_estimator.rs` 改用 `domain::message::Usage`。
- **删除 `XyFinishReason`**，`XyChunk::Done` 使用 `domain::message::StopReason`，同时删除 `XyStopReason` alias。
- **删除 `CompactionConfig`**（仅含 `enabled`），`infra/config/types.rs::AppConfig.compaction` 改为 `Option<domain::compaction_config::CompactionSettings>`。
- **重命名 `domain::compaction_config::CompactionSettings`** → `CompactionSettingsConfig`，表示文件加载形态；`agent::compaction::settings::CompactionSettings` 保留为运行时形态。
- **`ToolDefinition` 包含 `XyToolSchema`**，避免 `name/description/parameters` 字段重复。

### P1 — 统一事件体系

- **合并 `AgentEvent` 与 `AgentLifecycleEvent`**：以 `domain::lifecycle::AgentLifecycleEvent` 为唯一领域事件枚举；`agent::runtime::event::AgentEvent` 删除，ReAct 循环输出 `AgentEventStream` 改为 `AgentLifecycleEventStream`。
- **删除 `runtime_protocol::event::LifecycleEvent`**：`EventSink` 直接接收 `AgentLifecycleEvent`。
- **自动化 `protocol::Event` 映射**：提供 `From<&AgentLifecycleEvent> for protocol::Event` 与 `TryFrom<&protocol::Event> for AgentLifecycleEvent`，替换 `app/driver.rs` 中手写 `proto_to_agent`。

### P2 — 统一公共 API 的 `Xy*` 前缀

按“public 对外接口/定义/工具”原则，为以下类型加 `Xy` 前缀：

- `runtime_protocol/` 全部 port trait 与伴生类型：
  - `SessionStore` → `XySessionStore`
  - `EventSink` → `XyEventSink`
  - `BashExecutor` → `XyBashExecutor`
  - `ExportIo` → `XyExportIo`
  - `SandboxEngine` → `XySandboxEngine`
  - `SecretResolver` → `XySecretResolver`
  - `TrustStore` → `XyTrustStore`
  - `ResourceLoader` → `XyResourceLoader`
  - `ModelBuilder` → `XyModelBuilder`
  - `ToolExecutionMode` → `XyToolExecutionMode`
  - `BashResult` → `XyBashResult`
  - `SandboxVerdict` → `XySandboxVerdict`
- `domain/` 中对外核心抽象：
  - `AgentLifecycleEvent` → `XyEvent`
  - `ModelConfig` → `XyModelConfig`
  - `ModelKind` → `XyModelKind`
  - `ModelMeta` → `XyModelMeta`
  - `Usage` → `XyUsage`
  - `StopReason` → `XyStopReason`
  - `ToolDefinition` → `XyToolDefinition`
  - `CompactionSettingsConfig` → `XyCompactionSettingsConfig`

**不加前缀的**：
- `AgentMessage`, `AgentPart`, `ImageContent` 等 xylitol 独有但语义自明的消息类型。
- `protocol::Command` / `protocol::Event`：已在 `protocol` 命名空间下，对外 wire 名称保持不变。
- `infra/` 中具体实现：`EventBus`, `SessionManager`, `InfraBashExecutor` 等。

### P3 — 测试与文档

- 更新 `src/tests.rs::arch_guard` 中任何硬编码类型名扫描（如需要）。
- 更新 `tests/support/` 中的测试替身和快照。
- 更新 `AGENTS.md` 和 `_ARCH.md` 中涉及的类型名。
- 全量 `cargo fmt/clippy/test`、`cargo test --test bdd -- --test-threads=1`。

## Capabilities

- `architecture`
- `layer-architecture`
- `agent-runtime`
- `tool-system`
- `compaction`

## Impact

- **行为影响**：零运行时行为变化；纯重构 + 重命名。
- **公共 API 影响**：`lib.rs` 导出的公共类型名大量变化，但项目目前版本 `0.0.0-dev`，无 backwards-compatibility 承诺。
- **测试影响**：BDD feature 文件无需改动；Rust 测试和快照需更新。
- **编译影响**：全 crate 重编译一次，无新增依赖。

## Future

- 若后续引入更多 runtime port（如 `VectorStore`、`MemoryStore`），自动遵循 `Xy*` 前缀。
- 考虑为 `protocol::Command` / `protocol::Event` 增加 `Xy` 前缀或保持命名空间隔离，待 public API 稳定后决定。
