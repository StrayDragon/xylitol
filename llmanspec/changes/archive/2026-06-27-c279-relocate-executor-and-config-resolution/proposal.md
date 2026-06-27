---
depends_on:
  - c277-sink-assembly-to-composition-root
---

# c279-relocate-executor-and-config-resolution

> **状态**：draft 提案（2026-06-27）。c277 路线图分流项——把 c277 调研中判定为"高成本/高牵连"
> 的装配耦合单独成 change，避免塞进 c277 破坏 BDD。

## Why

c277 调研发现 6 项白名单的修法各自牵连较大，不宜与 c277 的纯装配注入混做：

1. **`agent/runtime/bash.rs` exec 原语（4 项）**——`tools::{accumulator,process,truncate}` +
   `process::shell::find_bash`。bash executor 本质是 **infra runtime**（进程派生 + 输出流），
   却住在 `agent/runtime/`。正确修法是把 executor 整体迁到 `infra/`，agent 经 port 或组合根调用，
   而非在 agent 里借 infra 工具原语。现有 NOTE 自辩解为"低层 util 可接受"，但 ceiling 已到
   （c275 收紧了守卫，这 4 项必须正式处理）。
2. **`agent/model/registry.rs` config::value（1 项）**——`value::resolve_config_value` / `resolve_headers`
   运行时调用于 auth/headers 解析。修法是立 `SecretResolver` port（c260 标为"触发型"），但
   `ModelRegistry::new` 有 **8 处调用点**，注入成本高，单独做更安全。
3. **`prompt/system.rs` DefaultResourceLoader（1 项）**——`DefaultResourceLoader` 是 resource loader
   具体服务，应在组合根构造注入，而非 agent prompt 内直接 `use`。
4. **`session/mod.rs` TrustManager（1 项，c277 修守卫后新暴露）**——`save_trust_decision` 直接引用
   `infra::trust::TrustManager`。应抽象为 `TrustStore` port 注入，与 c255 的信任 SSoT 解耦。

## What Changes

- **P1 bash executor 迁移**：`agent/runtime/bash.rs` → `infra/`（如 `infra/bash_exec/` 或并入
  `infra/process/`）；agent 侧 `session/bash_exec.rs` 改为经 port 或组合根注入的执行器句柄；
  exec 原语（accumulator/process/truncate/shell）随之留在 infra，agent 不再 import。消 4 项白名单。
- **P2 SecretResolver port**：`core::ports` 立 `SecretResolver` trait
  （`resolve_config_value`/`resolve_headers`）；`infra/config/value` 改为其 impl；
  `ModelRegistry` 持有 `Arc<dyn SecretResolver>`（构造注入，8 处调用点更新）。消 1 项白名单。
- **P3 resource loader 下沉**：`prompt/system.rs` 的 `DefaultResourceLoader` 依赖改为构造时注入
  （组合根构建 loader，传入 agent）。消 1 项白名单。
- **P4 trust store 下沉**：`session/mod.rs` 的 `TrustManager` 依赖抽象为 `TrustStore` port；
  `AgentSession::save_trust_decision` 改为接收 `&dyn TrustStore`。消 1 项白名单。

## Capabilities

- `layer-architecture`（modify）：强化 la7（runtime-residence）覆盖 executor + secret resolver + loader

## Impact

- **白名单收缩 7 项**（c277 retag 过来的 bash exec 4 项 + config::value 1 项 + resource loader 1 项 +
  c277 修守卫后新暴露的 trust 1 项）。
- **HC-2 进一步落地**：agent 不再借 exec 原语、不再在 prompt 内直接持 loader/trust store。
- **零行为变更**：迁移 + 注入；BDD 不受影响。
