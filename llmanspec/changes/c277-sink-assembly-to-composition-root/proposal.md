---
depends_on:
  - c275-refactor-layer-enforcement
---

# c277-sink-assembly-to-composition-root

> **状态**：draft 提案（2026-06-27）。c275 路线图第二项。与 c276 并行（均仅依赖 c275）。

## Why

c275 review 发现 `agent/` 内部直接**构造或持有 infra 具体实现**（而非注入 port），这是 c260 HC-1/HC-2
"装配在 server/cli 组合根发生"的直接违反，约占 38 处白名单违规中的 15 处。最严重的是
`agent/model/manager.rs:49` 在 `pub fn build_current_model()` 内直接调用
`infra::provider::factory::build_provider`——provider 装配发生在 agent 内部。

## What Changes

把所有具体 infra 实现的**构造/持有**从 `agent/` 下沉到组合根（`interactive/cli` + `server/runtime`），
agent 改为只接受注入的 port（`Arc<dyn Port>`）或纯参数：

- `build_provider` 调用（`agent/model/manager.rs`）→ 由组合根构造 `Arc<dyn XyModel>` 注入
- `SessionManager` 持有（`agent/facade.rs`、`agent/session/mod.rs`、`agent/compaction/{mod,orchestrator}`、
  `agent/session/export.rs`）→ 改持 `Arc<dyn SessionStore>`（la7/la8）
- `EventBus` 持有（`agent/session/{mod,events}.rs`）→ 改持 `Arc<dyn EventSink>`
- `SandboxEngine`（`agent/runtime/react.rs`、`agent/session/mod.rs`）→ 注入 `Arc<dyn SandboxPort>`
  或下沉路由逻辑
- `config::value` 秘钥解析（`agent/model/registry.rs`）→ 注入 `SecretResolver` port
- `runtime/bash.rs` 借用的 exec 原语（`tools::{accumulator,process,truncate}`、`process::shell`）→
  评估提取到 core 共享 util 或注入（带 c260 已有 NOTE 自辩解）

## Capabilities

- `layer-architecture`（modify）：强化 la7（runtime-residence）+ la8（agent-independence）的注入语义

## Impact

- **白名单收缩 ~15 项**：c275 白名单中装配类条目删除。
- **HC-2 真正落地**：`Agent`/`AgentSession` 不再持有具体后端，可在 server 中自由托管、脱离文件系统单测。
- **零行为变更**：装配位置移动 + 注入；BDD 不受影响。
