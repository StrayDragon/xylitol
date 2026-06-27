# c277-sink-assembly-to-composition-root — Design

> 装配下沉。c277 原提案覆盖 16 项白名单，但调研发现其中 12 项与 session 架构/执行器迁移深度
> 耦合，一次性做会破坏 BDD。本 design 把 c277 收窄为**纯装配注入**（3 项，低-中风险），其余按
> 性质拆分到 c278（session 解耦）与 c279（执行器/配置迁移），靠依赖图管理。

## 1. 调研结论：为何不全做

| 白名单项 | 修法 | 风险 | 归属 |
|---|---|---|---|
| `model/manager.rs` build_provider | 注入 model factory 闭包 | 低（ModelManager::new 仅 1 处调用） | **c277 本期** |
| `runtime/react.rs` SandboxVerdict | SandboxEngine trait + SandboxVerdict 移 core::ports（纯 port 上提） | 低（类型/trait 迁移，同 c276） | **c277 本期** |
| `session/mod.rs` sandbox（noop_engine 默认） | sandbox 改构造注入（必填），移除 noop_engine 回退 | 中（AgentSession::new 5 处 + with_ports 3 处） | **c277 本期** |
| `model/registry.rs` config::value | 新建 SecretResolver port + 注入 | 中-高（ModelRegistry::new **8 处**调用点） | **→ c279** |
| SessionManager 具体持有（5 项） | SessionStore port 需扩 `build_session_context`/`load`/`get_tree` | 高（port 扩 + 公共 API） | **→ c278** |
| EventBus 具体持有（2 项） | emit 流从 sync `event_bus.emit_lifecycle` 迁到 async `sink.emit` | 高（async 化全部 emit 点 + public API） | **→ c278** |
| `runtime/bash.rs` exec 原语（4 项） | bash executor 本质是 infra runtime，应整体迁出 agent | 高（模块迁移） | **→ c279** |
| `prompt/system.rs` DefaultResourceLoader（1 项） | resource loader 构造下沉组合根 | 中 | **→ c279** |

关键调研事实：
- `AgentSession` 已有 `store: Arc<dyn SessionStore>` + `sink: Arc<dyn EventSink>` port（c272 加），但
  **同时**保留具体 `session_manager: SessionManager` + `event_bus: EventBus`，且 emit 走 sync
  `event_bus.emit_lifecycle` 而非 async `sink.emit`。`subscribe()`/`event_bus()` 公共 API **零外部调用者**
  （死 API），但移除仍涉及 async 化 emit 流 → 归 c278。
- `SessionStore` port 仅 `load_context/append_entry/exists`；compaction/export 用的
  `load`/`build_session_context`/`get_tree` 是 SessionManager 特有 → 需扩 port，归 c278。
- `SecretResolver` port 不存在（c260 标为"触发型"）→ 本期不立（ModelRegistry::new 8 处调用点成本过高），归 c279。

## 2. 本期实现（Tier A，清 3 项）

### 2.1 model factory 注入（清 model/manager.rs）

- `core::ports` 无需新 trait；用闭包类型 `Arc<dyn Fn(&ModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>`。
  - 满足 HC-5：2 实现（组合根传 `build_provider`；测试传 fake）。
- `ModelManager::new(registry, model_builder)`；`build_current_model` 调 `self.model_builder(&meta.config)`。
- 移除 `use crate::infra::provider::factory::build_provider`。
- `AgentSession::new` 增 `model_builder` 参数，透传 `ModelManager::new`。
- `Agent::with_ports` 增 `model_builder` 参数。
- 组合根（cli/server/rpc）传 `Arc::new(build_provider)`（`fn` 指针自动协变为 `dyn Fn`）。

### 2.2 SandboxEngine + SandboxVerdict 移 core::ports（清 react.rs）

- `SandboxEngine` trait + `SandboxVerdict` enum 迁到 `core::ports`（sandbox 子段）。它们本质是 port
  （c260 已把 XyModel/XyTool/SessionStore/EventSink 放 core::ports，SandboxEngine 同类）。
- `infra/sandbox` 保留 impls（FallbackBackend、noop_engine、build_engine）+ `pub use` 转发 trait/verdict。
- `react.rs`：`use crate::infra::sandbox::SandboxVerdict` → `use crate::core::ports::SandboxVerdict`。

### 2.3 sandbox 构造注入（清 session/mod.rs sandbox）

- `AgentSession.sandbox_engine`：`Option<Arc<dyn SandboxEngine>>` → `Arc<dyn SandboxEngine>`（必填）。
- `AgentSession::new` 增 `sandbox: Arc<dyn SandboxEngine>` 参数；`get_sandbox_engine` 返回 `self.sandbox_engine.clone()`（无 noop 回退）。
- 移除 `use crate::infra::sandbox::{..., noop_engine}` 与 `unwrap_or_else(noop_engine)`。
- 移除 `set_sandbox_engine`（公共 API；cli/rpc 改为构造时传 sandbox，不再构造后 set）。
  - cli/rpc：`sandbox: sandbox_engine.clone().unwrap_or_else(|| Arc::new(noop_engine()))`（noop 在组合根导入，合法）。
- `Agent::with_ports` 增 `sandbox` 参数。
- 组合根 + BDD/react 测试点更新。

### 2.4 白名单更新

- 删 3 项：`runtime/react.rs` sandbox、`session/mod.rs` sandbox、`model/manager.rs` build_provider。
- 重标 13 项：SessionManager(5)+EventBus(2) → c278；config::value(1)+bash(4)+resource(1) → c279。

## 3. 验证护栏

- 每次 signature 变更后 `cargo build --lib` + 跑相关 BDD。
- 全程 88 BDD 不变（纯装配位置迁移，零行为变化）。
- arch_guard 白名单 21 → 18（删 3）。
- API baseline 快照会变（`set_sandbox_engine` 移除、`AgentSession::new`/`with_ports` 签名变）→ 接受。
