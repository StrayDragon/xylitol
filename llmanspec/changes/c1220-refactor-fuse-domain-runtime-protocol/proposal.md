---
change_id: c1220-refactor-fuse-domain-runtime-protocol
title: 消除纯 domain 层——类型就地融合；ports 并入 protocol；agent ≈ pi-agent-core
status: full
priority: 1220
depends_on:
- c1210-refactor-compose-bridge-llm
author: agent
branch: feat/c1220-refactor-fuse-domain-runtime-protocol
base_sha: 92fe7048a544aab9c8e4d0ea59b0ea8206150b25
checkpointed: false
---

# c1220-refactor-fuse-domain-runtime-protocol

## Why

`src/domain/` 纯类型桶 + `src/runtime_protocol/` 与 `src/protocol/` 双顶栏，是过度分层：会话词汇被迫共享、ports 与线协议同属「跨边界契约」却品牌分裂。c1210 已钉 LLM 叶组合；本 change 对齐 pi：**agent ≈ agent-core**（类型与循环同住），**一个 `protocol/`** 收契约，**删掉独立 domain/**。

## Purpose（已钉）

1. **消 `src/domain/`**：不再存在独立纯类型顶栏；进 port/wire 签名的共享类型落在 **`protocol` 根模块**（方案 B），**禁止**再立 `vocab/`/`types/` 第三子树顶栏。
2. **`src/agent/` ≈ pi-agent-core（语义）**：拥有 `project_for_llm`、ReAct、对 `AgentMessage`/`XyEvent` 的生产与再导出。因单 crate + `infra`↛`agent`，`AgentMessage`/`XyEvent`/`SessionEntry` **物理**在 `protocol` 根（避免环）。
3. **`XyModel` 入参 = `Vec<AiBridgeMessage>`**：infra adapter **MUST NOT** 再吃 `AgentMessage`。
4. **`src/protocol/` 方案 B**：仅两子树 `wire/` + `ports/`；共享类型为 protocol 根 `.rs`；删 `src/runtime_protocol/`（迁移期可留 re-export，apply 结束前去掉）。
5. **依赖纪律**：`infra` ↛ `agent`；`agent` ↛ `infra`；`protocol` MUST NOT 依赖 agent/infra 实现。
6. **`XyEvent` 钉死**：物理 `protocol/lifecycle.rs`；wire `Event` 分离；`XyEventSink` 在 `ports/`。

## What Changes

- live specs：`layer-architecture`（la1/la2/la14/la16–la18/…）、`domain-message`（归属与 valid_scope）、`infra-provider`（pa20）、相关 features
- 实现（apply）：搬模块、改 `XyModel`、合并 protocol、更新 AGENTS/`pub use`、全量编译与 BDD

## Out of scope

- 抽多 crate；改 Trust/MCP 产品语义；重做 Command/Event 线协议形状
- 强制改写用户磁盘 session（wire 形状由 c1210 已钉，本 change 不改 JSONL role 语义）

## Capabilities

- `layer-architecture`（modify）
- `domain-message`（modify；capability 名暂留，valid_scope → agent）
- `infra-provider`（modify；pa20 / pa7）
- `package-ai-bridge`（modify；pab3/pab4 措辞）
- 路径跟改：`agent-runtime`、`agent-session`、`agent-session-store`、`agent-tools`、`agent-prompt`、`app-tui`、`domain-security`、`domain-compaction`、`infra-bash`、`infra-process`、`runtime-model-registry`、`runtime-resource-discovery`、`server-runtime`、`test-standards`、`test-provider-integration`、`test-hooks-wiring`、`user-experience`

## Ethics

- risk_level: critical
- prohibited_actions: 再造第三个纯类型顶栏；AgentMessage 与 Command 无结构混源；infra 依赖 agent
- required_evidence: 无 `src/domain/` / `src/runtime_protocol/`（或仅短暂 re-export 再删）；`XyModel::generate_stream` 签名为 LLM DTO；`just qa` 绿
- escalation_policy: session JSONL 类型最终文件路径若与 design 冲突，apply 前以 design 为准改 tasks
