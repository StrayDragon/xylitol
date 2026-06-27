# Tasks: c275-refactor-layer-enforcement

## P1 — 收紧 arch_guard（la9 扩范围 + la12 白名单）

- [x] T1 重写 `src/tests.rs::arch_guard::agent_does_not_import_infra_in_production`：
  全量扫描 `src/agent/` 生产代码（剔除 `#[cfg(test)]` 区域与 `//` 注释行）的 `crate::infra::` 导入
- [x] T2 引入 `AGENT_INFRA_ALLOWLIST` 白名单：`&[(rel_path, import, follow_up)]`，每项强制
  标注 c276/c277/c278；白名单内豁免，外则 `assert!` 失败并报告 file:line + 提示注册或消除
- [x] T3 录入当前 38 处生产违规（baseline）：c276（18 项类型性耦合）、c277（15 项装配）、c278（5 项 export 转发）
- [x] T4 验证守卫现状：白名单精确覆盖 38 处，守卫跑通；注入 `crate::infra::config::loader::load_app_config`
  到 `agent/model/registry.rs` 确认被拦截（报错 `model/registry.rs:15: ... not in allowlist`），移除后恢复

## P2 — auth 引导归位（la13）

- [x] T5 `git mv src/agent/auth/guidance.rs src/interactive/cli/provider_guidance.rs`；删除
  `src/agent/auth/` 与 `agent/mod.rs::pub mod auth`
- [x] T6 改 `interactive/cli/mod.rs`：新增 `mod provider_guidance;`，3 处 `auth::format_*` →
  `provider_guidance::format_*`；函数名/签名不变（`user-experience` ux1–ux4 按名锁定）
- [x] T7 验证：build + nextest + BDD 全绿；`grep -rn "agent::auth\|agent/auth" src/ tests/` 为空

## P3 — 文档修正（清腐化）

- [x] T8 `AGENTS.md`：层列表补 `server/`（常驻核心，第二组合根）；`protocol/` → `protocol.rs`
  对齐实际；`interactive/` 补 `cli/provider_guidance.rs` 与依赖规则；删 `agent/` 的 `auth/`
- [x] T9 `src/server/mod.rs`：删 "Currently under construction via c270-add-server-process"（c270 已归档）
- [x] T10 `src/tests.rs` arch_guard 头注释：删 "tracked for cleanup by c270 / T2 c270" 等过期 TODO；
  改为白名单机制说明（la9 全量扫描 + la12 allowlist）
- [x] T11 `agent/facade.rs`：清 "future RemoteDriver will mirror it" 注释，改为指向已实现的 `server::ws`/`server::rest`

## P4 — 收尾验证

- [x] T12 全量 QA：`cargo fmt`（已应用）+ `cargo clippy --all-targets`（exit 0）+
  `cargo nextest run --profile ci`（629 passed）+ `cargo test --test bdd -- --test-threads=1`（88 passed）
- [x] T13 确认 arch_guard 全绿（3/3：infra→agent、agent→infra 全量+白名单、interactive 隔离）
- [x] T14 `llman sdd validate c275-refactor-layer-enforcement --strict --no-interactive` 通过
