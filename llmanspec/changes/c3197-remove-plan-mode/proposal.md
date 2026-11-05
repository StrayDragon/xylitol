---
depends_on: []
title: "Remove plan mode"
---

## Why

Plan mode 是会话状态 + 工具面 + 提示注入 + 只读闸 + 审批/交接 + ModeStrip 的组合。它不引入新的执行能力：计划只是 turn 之间的意图转换，模型在普通 turn 里也能用 todo / plan 文档 / 对话完成。把“计划”建模成模式，带来额外状态、命令、事件、UI 和工具限制；只读闸又太粗，用户仍需要 preview 写入等受限写操作，导致特例。

## What Changes

- 移除 plan mode 状态：PlanMode / PlanModeKind、custom_type=plan_mode、latest_plan_mode、SetPlanMode 命令、/plan slash、PlanModeChanged 事件、ModeStrip 的 plan 状态源。
- 移除 plan 模式中间件：read-only 闸、tool surface 隐藏、plan 模式提示注入。
- 移除 plan_mode 工具；plan_submit 的审批/模式切换/todo 交接如何处理见 Options。
- plan 文档工具去留见 Options。
- Specs：删除或重写 agent-plan；更新 protocol-app（SetPlanMode / PlanModeChanged）、app-tui-commands、cli-entry、app-tui-fixed-zone（ModeStrip）、agent-runtime、agent-tools、agent-todo、workspace-assembly。
- 文档/AGENTS：移除 plan 插件与 plan mode 描述；保留 plan 文档（若保留）。
- 不提供替代 plan 机制；后续单独设计。

## Options

- A（推荐）：只删 mode；保留 plan 文档工具 plan_write/plan_read/plan_list 作为普通工具；删除 plan_mode 与 plan_submit 的审批/模式切换/todo 交接。
- B：删除整个 plan 插件（mode + 文档 + 工具）；后续再设计计划工件。
- C：保留 plan 文档与 plan_submit 审批，但去掉 mode 状态/命令/UI；需要重新定义审批语义。

## Capabilities

- agent-plan：删除或重写。
- protocol-app：删除 SetPlanMode / PlanModeChanged。
- app-tui-commands / cli-entry：删除 /plan。
- app-tui-fixed-zone：删除或保留 ModeStrip（无 consumer 则删）。
- agent-runtime：删除 plan 模式提示注入/中间件。
- agent-tools：plan_mode/plan_submit/plan_* 去留。
- agent-todo：plan_submit 的 todo 交接去留。
- workspace-assembly：plan 插件去留。

## Impact

plan 插件、sdk session、runtime loop、kernel commands、protocol/wire、TUI、specs、docs。旧会话的 plan_mode/plan_doc custom entries 需兼容（未知 CustomEntry 忽略或迁移）。

## Risks

- 用户依赖 /plan 与 ModeStrip；删除是行为回退。
- 旧会话含 plan_mode/plan_doc 条目；需兼容/清理策略。
- plan_submit 的 todo 交接被移除；用户可能依赖。
- 删除 ModeStrip 影响固定区 layout；保留则成 dead UI。
- 与 c3106 codemode 的 plan mode catalog 交互消失；c3106 parked，恢复时不再有 plan mode。

## Open Questions

- 选 A/B/C：plan 文档与 plan_submit 去留。
- ModeStrip 删除还是保留通用壳。
- 旧会话兼容/迁移。
- 替代计划机制后续如何设计。

## Further Notes

- 用户判断：plan mode 是 turn 转换，不引入新能力；纯工具限制是错误设计。
- c3196 plan preview draft 已删除；c3106 codemode parked。
- 本 change 为 draft；design/tasks 未开始。
