# Tasks: c277-sink-assembly-to-composition-root

## P1 — SandboxEngine port 上提 core::ports

- [x] T1 把 `SandboxEngine` trait + `SandboxVerdict` enum 从 `infra/sandbox/mod.rs` 迁到 `core/ports.rs`（sandbox 段）
- [x] T2 `infra/sandbox/mod.rs` 改 `pub use crate::core::ports::{SandboxEngine, SandboxVerdict}` + 保留 FallbackBackend/noop_engine/build_engine impls
- [x] T3 `react.rs`：`use crate::infra::sandbox::SandboxVerdict` → `use crate::core::ports::SandboxVerdict`
- [x] T4 验证 build + arch_guard（react.rs sandbox 项应可删）

## P2 — sandbox 构造注入（清 session/mod.rs sandbox）

- [x] T5 `AgentSession.sandbox_engine`: `Option<Arc<dyn SandboxEngine>>` → `Arc<dyn SandboxEngine>`；`AgentSession::new` 增 `sandbox` 参数；`get_sandbox_engine` 去掉 `unwrap_or_else(noop_engine)`
- [x] T6 移除 `use crate::infra::sandbox::noop_engine`；移除 `set_sandbox_engine` 方法
- [x] T7 `Agent::with_ports` 增 `sandbox: Arc<dyn SandboxEngine>` 参数
- [x] T8 更新组合根 cli/rpc/server：构造 sandbox（`build_engine` 或 `noop_engine()`）传入 with_ports；删 `set_sandbox_engine` 调用
- [x] T9 更新 BDD（3 处）+ react.rs 测试（2 处）AgentSession::new 调用，传入 sandbox
- [x] T10 验证 build + BDD + arch_guard（session/mod.rs sandbox 项应可删）

## P3 — model factory 注入（清 model/manager.rs）

- [x] T11 `ModelManager::new(registry, model_builder: Arc<dyn Fn(&ModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>)`；`build_current_model` 调 `self.model_builder`；移除 `build_provider` import
- [x] T12 `AgentSession::new` 增 `model_builder` 参数透传；`Agent::with_ports` 增 `model_builder` 参数
- [x] T13 组合根 cli/rpc/server 传 `Arc::new(build_provider)`（fn 协变 dyn Fn）；BDD/react 测试传 fake factory
- [x] T14 验证 build + BDD + arch_guard（model/manager.rs 项应可删）

## P4 — 白名单更新 + retag + c279 draft

- [x] T15 从 arch_guard 白名单删 3 项；并修复守卫的 `#[cfg(test)]` 单行属性误判为测试模块边界的 bug（改用 `#[cfg(test)] mod` 作边界），借此暴露并修正 3 处 c276 遗漏的 session/mod.rs 类型 import（PromptTemplate/SessionEntry×2），新增 1 项 session/mod.rs trust（-> c279）
- [x] T16 重标 13 项：SessionManager(5)+EventBus(2) → c278；config::value+bash(4)+resource(1) → c279
- [x] T17 创建 c279 draft 提案（bash executor 迁移 + config::value SecretResolver + resource loader 下沉）

## 收尾

- [x] T18 全量 QA：`cargo fmt && cargo clippy --all-targets && cargo nextest run --profile ci && cargo test --test bdd -- --test-threads=1`
- [x] T19 确认 arch_guard 白名单 21 → 18，全绿
- [x] T20 接受 API baseline 快照更新（set_sandbox_engine 移除、构造签名变）
- [x] T21 `llman sdd validate c277-sink-assembly-to-composition-root --strict --no-interactive` 通过
