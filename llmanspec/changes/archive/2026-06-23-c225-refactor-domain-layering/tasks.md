# c225-refactor-domain-layering: Tasks

> 全部任务待 apply 阶段实施。每个物理迁移 chunk 完成后跑 `cargo check --all-features` 快速定位下一个漏改 import。单 chunk ≤ 2h。
> 注:strict 校验为 apply 后门禁,proposal 阶段任务待办属正常(非 strict 校验通过即可)。

## Phase 1 — 盘点与 core 层骨架

- [x] **T1** — 盘点被迁移类型的全部引用点
- [x] **T2** — 创建 `src/core/mod.rs` 声明子模块 message/traits/model/types/error
- [x] **T3** — 在 `src/lib.rs` 注册 `pub mod core;` 置于 agent/infra/interface 之前

## Phase 2 — 迁移类型到 core（逐类型，编译器驱动）

- [x] **T4** — 移动 `agent/message.rs` 到 `core/message.rs`
- [x] **T5** — 移动 XyModel/XyTool trait 及 ctx/error 到 `core/traits.rs`、`core/error.rs`
- [x] **T6** — 移动 ModelKind/ModelConfig/ModelMeta 到 `core/model.rs`
- [x] **T7** — 移动 XyChunk/XyUsage 等流/用量类型到 `core/types.rs`
- [x] **T8** — 修正 agent/ 内 import 为 `crate::core::*`

## Phase 3 — 迁移违规 infra 文件

- [x] **T9** — `infra/config/types.rs` 中 agent::model/profile/registry 改为 `crate::core::*`
- [x] **T10** — `infra/config/loader.rs` 的 ModelKind 改为 `crate::core::model`
- [x] **T11** — `infra/session/compaction.rs` 的 AgentMessage/XyModel/XyChunk 改为 `crate::core::*`
- [x] **T12** — `infra/session/manager.rs` 返回类型与 `&dyn XyModel` 改为 `crate::core::*`
- [x] **T13** — `infra/event/lifecycle.rs` 事件 payload 的 AgentMessage 改为 `crate::core::message`
- [x] **T14** — `infra/skills/mcp.rs` 改为 `impl crate::core::traits::XyTool`

## Phase 4 — 架构守卫

- [x] **T15** — 新增守卫测试 grep `src/infra/**/*.rs` 中 `crate::agent`，命中即 panic 并打印文件
- [x] **T16** — 确认守卫测试当前为绿

## Phase 5 — 验证

- [x] **T17** — `cargo fmt`
- [x] **T18** — `cargo clippy --all-features --all-targets`
- [x] **T19** — `cargo test --lib` 期望 413 passed
- [x] **T20** — `cargo test --test bdd -- --test-threads=1` 期望 79 passed
- [x] **T21** — `llman sdd validate c225-refactor-domain-layering --strict --no-interactive`
- [x] **T22** — 人工抽查 `src/agent/mod.rs` 不再 re-export 已迁出类型，避免 pub 别名 shim
