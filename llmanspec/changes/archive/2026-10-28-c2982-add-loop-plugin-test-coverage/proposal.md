---
depends_on: []
title: 补齐 plan/todo 中间件与压缩状态保留的测试覆盖
needs_specs_change: false
branch: sdd/c2982-add-loop-plugin-test-coverage
base_branch: main
base_sha: 4e5d2d2f2fbe3554793329cdb022d1aa42fa807f
---

# 补齐 plan/todo 中间件与压缩状态保留的测试覆盖

## Why

- PR #30 的 CRAP 报告新增了多个高值函数，且覆盖率为 0%：
  - `PlanSubmitTool::execute_typed` / `PlanModeTool::execute_typed`（CRAP 240，CC 15，0%）；
  - `todo_items_from_parts`（CRAP 240，CC 15，0%）；
  - `TodoReminderMiddleware::before_llm_request`（CRAP 72，CC 8，0%）；
  - todo 中间件其余 7 个函数（CRAP 20–56，0%）。
- `plugins/xylitol-official-plugin-todo/src/middleware.rs` **整个文件没有测试**；plan 工具虽有测试文件，但审批/模式切换路径未被执行。
- `c2975` 的状态保留只测了 `plan_mode`，未覆盖 `plan_doc` / `plan_ref` / `workflow` / `todo_checkpoint`。
- 还有一批低风险薄函数 0% 覆盖：`map_bootstrap_error`、`build_ports`、`exec_export_html/jsonl`、`slash/misc.rs::plan`、`same_workflow_structure`。

## What Changes

- 为 todo 中间件补单测：progress（记录/续跑）、reminder（触发条件/同一 turn 只提醒一次）、checkpoint（记录/注入/落盘）。
- 为 plan 工具补单测：`plan_submit` 批准/拒绝/未知 plan、`plan_mode` 切换/已批准跳过 ask、`plan_write`/`plan_read`/`SessionPlanGateway::read_file`。
- 为压缩状态保留补测试：`ensure_state_entries_after_compact` 覆盖 `plan_doc` / `plan_ref` / `workflow` / `todo_checkpoint`。
- 为低风险薄函数补测试（按性价比，不追求 100%）：
  - `map_bootstrap_error`、`build_ports`；
  - `XyRemoteDriver::exec_export_html` / `exec_export_jsonl`；
  - `slash/misc.rs::plan`；
  - `same_workflow_structure`。
- 测试若暴露与 spec 不一致的行为，按“小偏差同 change 修、大偏差另开 change”处理，不把错误行为固化成断言。

## Capabilities

- `agent-plan`：plan 工具行为（测试覆盖）。
- `agent-runtime`：todo 中间件行为（测试覆盖）。
- `domain-compaction`：压缩后状态保留（测试覆盖）。
- `app-tui-*`：远端导出 / `/plan` slash（测试覆盖）。

## Impact

- `plugins/xylitol-official-plugin-todo/src/middleware.rs`：新增 `#[cfg(test)]`。
- `plugins/xylitol-official-plugin-plan/src/tools.rs`：补审批/模式路径测试。
- `plugins/xylitol-official-plugin-compaction/src/lib.rs`：补状态保留测试。
- `crates/xylitol-kernel/src/occupancy/runtime.rs`、`crates/xylitol-app-tui/src/driver/tests.rs`、`crates/xylitol-app-tui/src/effects/slash/misc.rs`、`crates/xylitol-plugin-sdk/src/session/workflow.rs`：薄函数测试。
- 无生产代码改动（除非测试暴露缺陷）。

## Out of Scope

- 追求 CRAP 数值归零；只覆盖行为分支与高风险路径。
- 为 `page_from_segments`、`WorkflowMiddleware::before_llm_request` 等已 100% 覆盖的高 CC 函数做重构。
- 新增 BDD 场景（除非某条 spec 明确要求且现有绑定缺失）。

## Further Notes

- 由 PR #30 CRAP 报告与 review F14 记录。
- 测试分层：组件边界用单测；端到端编排才用 BDD（见 `xylitol-dev` skill `references/test-layering.md`）。
