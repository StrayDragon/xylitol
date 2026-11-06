# Tasks — Remove plan mode (Option B)

> 执行顺序按 T1 → T8。大范围机械删除；每完成一项立即勾选。收口（verify / finalize / archive）不在本文件。

- [x] T1 protocol/wire：删除 `SetPlanMode`、`PlanModeChanged`、`PlanDocUpdated`、`METHOD_SET_PLAN_MODE` 与 wire 映射；`PROTOCOL_VERSION` 5 → 6；更新 registry / message 测试。验证：`cargo test -p xylitol-protocol -p xylitol-wire`。
- [x] T2 SDK：删除 `session/plan_mode.rs`、`session/plan_doc.rs`、`ports/plan.rs` 与导出；删除 `BeforeLlmRequestCtx.plan_mode`、`PlanModeChanged` / `PlanDocUpdated` 事件；保留 workflow legacy `plan` custom_type 迁移。验证：`cargo test -p xylitol-plugin-sdk`。
- [x] T3 runtime/kernel：删除 `latest_plan_mode` 注入、`PlanGateway`、`ToolFeatures.plan`、`SessionPlanGateway`、`compose_tools` plan 参数/工具、`SetPlanMode` 命令、`/plan`、`DispatchOutcome::PlanMode`、`PlanPlugin` 注册、boot plugin rows 与 contract 断言。验证：`cargo test -p xylitol-runtime -p xylitol-kernel`。
- [x] T4 plan plugin/workspace：删除 `plugins/xylitol-official-plugin-plan`；根 `Cargo.toml` member/dependency、kernel/config 依赖、`Cargo.lock`、`scripts/check_crate_graph.py` 同步。验证：`cargo check --workspace`。
- [x] T5 config/compaction：删除 `PlanConfig`、`plan` 配置节、`sections.rs` plan schema/default、`configs/config.schema.json` plan 节；loader 对旧 `plan` / `loop_policies.plan_mode` 返回「plan mode 已移除」错误；compaction 不再保留 plan_mode / plan_doc / plan_ref。验证：`cargo test -p xylitol-official-plugin-config -p xylitol-official-plugin-compaction`。
- [x] T6 TUI/Print/Serve：删除 `/plan` 解析与 pending、ModeStrip 状态/渲染/固定区 footprint、bridge handler/state、input policy、remote unary、harness 命令；serve 删除 plan 快照；print 删除 plan 事件忽略。验证：`cargo test -p xylitol-app-tui -p xylitol-app-print -p xylitol-serve`。
- [x] T7 Specs/BDD/docs：删除 `llmanspec/specs/agent-plan/`；更新 protocol-app、app-tui-commands、app-tui-fixed-zone、app-tui-bridge、app-tui、agent-runtime、agent-session-store、server-core、domain-compaction、domain-security、runtime-config、agent-tools、workspace-assembly；删除 plan BDD bindings/steps；更新根 `AGENTS.md`、`src/AGENTS.md`、`llmanspec/AGENTS.md`、`crates/xylitol-app-tui/src/AGENTS.md`。验证：`llman-sdd validate --specs`。
- [x] T8 门禁/残留：`rg -n "PlanMode|plan_mode|ModeStrip|plan_write|plan_submit|plan_doc" crates plugins src tests llmanspec/specs` 应为空（排除 archive/delayed/research 与本 change）；跑 `just fmt` / `just lint` / `just test` / `llman-sdd validate c3197-remove-plan-mode --strict --check`。不得把 workflow legacy `plan` custom_type 当 plan mode 删除。
