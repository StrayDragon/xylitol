---
depends_on: []
title: "Fix workflow step state transitions"
---

# Workflow 步骤状态转移修复

## Why

WorkflowStepStatus 五态枚举存在，但转移表缺失：写入点散落 6 处（middleware 4 + tools 2），Pending→Completed 直跳、Failed→InProgress 回退均无校验；容器级 WorkflowState 用 9 个字段组合出 7 个执行阶段，任一漏重置即进入错误阶段（扫描证据见 research/implicit-state-machines.md，C3/C4；Lead 已复核 WorkflowState 结构与 step_to_fail）：

- 字段群：plan Option + stack Vec + approved HashSet + replan_count + contract_injected + injected_step + summary_injected + summary_done + pending_workflow_error——无单一 transition，reload_workflow 重置 6 字段与 after_tool 置 Failed 是两条独立路径。
- Blocked 态由 requires_human + approved 集合隐式表达；approved 永不清空（步骤重跑后仍算已批准）。
- SDK 侧 validate_workflow_replan 只管重规划场景，普通写入无守卫；workflow_fail 仅拦 Completed→Failed。

## What Changes

- SDK 引入 workflow 步骤合法转移表（单一 transition(step, event)）：Pending→InProgress→{Completed|Failed|Blocked}，Completed 锁定（对齐 src/AGENTS.md「已完成步骤锁定，只有失败步骤及其下游可改写」），回退显式拒绝。
- 容器状态收敛为显式阶段枚举（契约期/执行期/阻塞待批准/重规划/子流程/总结期/完成），字段群降级为阶段附属数据；reload 与 after_tool 两条重置路径合一。
- approved 批准集合与步骤重跑生命周期绑定；Blocked 态显式化。
- LLM 工具与中间件的每次 status 写入经转移表校验，非法转移显式报错（可观测）。

## Capabilities

- agent-workflow（步骤图、局部重规划与审批）。

## Impact

- 范围：crates/xylitol-plugin-sdk/src/session/workflow.rs、plugins/xylitol-official-plugin-workflow/{middleware,tools}.rs。
- 行为变更：模型对已完成步骤的改写从静默生效变为拒绝——对齐既有产品规则，属纠偏而非新契约。
- 验证：转移矩阵单测（全组合）；重规划 BDD 回归；非法写入报错场景。

## Open Questions

1. 存量会话中的非法序列（历史事实已写入）如何投影：容忍投影 + 新写入拦截（推荐）还是加载时修复？
2. Blocked 显式化是否顺带进 CustomEntry 持久形状（涉及旧条目兼容）？
