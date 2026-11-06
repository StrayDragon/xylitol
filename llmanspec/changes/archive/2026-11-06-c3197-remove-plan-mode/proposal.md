---
depends_on: []
title: "Remove plan mode"
branch: sdd/c3197-remove-plan-mode
base_branch: main
base_sha: b6501f5292d4b946e935c672b2e84bcfdba81dd6
---

## Why

Plan mode 是会话状态 + 工具面 + 提示注入 + 只读闸 + 审批/交接 + ModeStrip 的组合。它不引入新的执行能力：计划只是 turn 之间的意图转换，模型在普通 turn 里也能用 todo / 对话完成。

三个问题：

- 不灵活：只读闸把写操作一刀切；preview 写入等受限写需求只能做特例。
- KV cache 不友好：模式切换会改工具面、注入 plan 契约，打断 prompt/tool 前缀缓存；每次 /plan 切换都可能造成 cache miss。
- 单对话体验减分：同一对话里切模式，用户要记两套规则；模式状态/命令/事件/UI 增加认知负担。

结论：plan mode 应作为独立模式淘汰；计划意图之后可以用其他形态复用（例如 turn 级意图、工具策略、普通 todo/文档），不在本 change 设计替代。

## What Changes

- 删除整个 plan 插件：mode 中间件、plan 工具（plan_write/plan_read/plan_list/plan_submit/plan_mode）、plan 文档/审批/todo 交接。
- 删除 plan mode 状态：PlanMode / PlanModeKind、custom_type=plan_mode、latest_plan_mode、SetPlanMode 命令、/plan slash、PlanModeChanged 事件。
- 删除 ModeStrip 的 plan 状态源；ModeStrip 无其他 consumer，一并删除（固定区 footprint/layout 同步）。
- 删除 sdk session 的 PlanDoc/PlanRef/PlanScope/PlanMode 类型（确认无其他引用后）。
- Specs：删除 agent-plan；更新 protocol-app（SetPlanMode/PlanModeChanged）、app-tui-commands、cli-entry、app-tui-fixed-zone（ModeStrip）、agent-runtime（plan 提示注入）、agent-tools（plan 工具）、agent-todo（plan 交接）、workspace-assembly（plan 插件）。
- 文档/AGENTS：移除 plan 插件与 plan mode 描述。
- 旧会话：plan_mode/plan_doc CustomEntry 忽略；不迁移；现有 .xylitol/plans 文件保留但不再有工具入口。
- 不提供替代 plan 机制；后续单独设计。

## Decision

- 选 B：完整删除 plan 插件（mode + 文档 + 工具）。
- A/C 不采用：保留 plan 文档/审批会留下半套语义，和“淘汰独立模式”冲突。

## Capabilities

- agent-plan：删除。
- protocol-app：删除 SetPlanMode / PlanModeChanged。
- app-tui-commands / cli-entry：删除 /plan。
- app-tui-fixed-zone：删除 ModeStrip。
- agent-runtime：删除 plan 模式提示注入/中间件。
- agent-tools：删除 plan_* 工具。
- agent-todo：删除 plan_submit 的 todo 交接。
- workspace-assembly：删除 plan 插件。

## Impact

plan 插件 crate、sdk session、runtime loop、kernel commands、protocol/wire、TUI 固定区、specs、docs。旧会话兼容：未知 CustomEntry 忽略；plan 文件保留。KV cache：删除模式切换后，工具面/提示前缀不再因 /plan 改变。

## Risks

- 用户依赖 /plan、plan 文档、ModeStrip；删除是行为回退。
- 旧会话含 plan_mode/plan_doc 条目；需确认忽略不报错。
- 删除 ModeStrip 影响固定区 layout/footprint；需同步 spec/测试。
- 现有 .xylitol/plans 文件失去工具入口；用户可手工读取。
- c3106 codemode 的 plan mode catalog 交互消失；c3106 已迁回 active，依赖本 change。

## Open Questions

- 旧会话兼容/迁移细节（忽略 vs 清理）。
- 现有 plan 文件是否提供迁移/导出说明。
- 替代计划机制后续如何设计（不在本 change）。

## Further Notes

- 用户判断：plan mode 是 turn 转换，不引入新能力；独立模式不灵活、KV cache 不友好、单对话体验减分。
- 选 B；c3196 plan preview draft 已删除；c3106 codemode 已迁回 active。
- c3206 已归档，但实际代码仍保留 `plan_mode` / `plan_doc`（`crates/xylitol-plugin-sdk/src/session/mod.rs:47`；`plan_mode.rs`、`plan_doc.rs`）；c3206 归档提案中的“已删除模式切换”是错误表述。本 change 的删除范围不变。
- 本 change 为 designed；design 已完成，tasks 未开始。
