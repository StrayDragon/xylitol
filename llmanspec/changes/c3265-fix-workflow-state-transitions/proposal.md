---
depends_on:
  - c3240-fix-react-loop-run-state
title: "Fix workflow task state transitions"
---

# Workflow 任务状态转移修复（含 Step→Task 更名）

> 2026-11-11 定稿。依赖 c3240：词汇定稿中 step 已被运行时层占用（模型调用周期），workflow 域的 Step 是同名异物——本 change 一并更名收口 SSOT。

## Why

- WorkflowStepStatus 五态枚举存在但无转移表：写入点散落 6 处（middleware 4 + tools 2），已核证 tools.rs:305-330 仅拦 Completed→Failed——Pending 可直跳 Failed、任意非完成步可按 id 置败、Failed→InProgress 回退无校验。
- 容器级 WorkflowState 九字段组合出七个执行阶段（已核证 :77-110），无单一 transition；reload 与 after_tool 是两条独立重置路径；approved 永不清空（步骤重跑仍算已批准）。
- SSOT 冲突：运行时层 step（c3240 定稿）与 workflow Step（任务图步骤）同名异物。

## What Changes

- **WorkflowStep* → WorkflowTask*（一步到位不留别名）**：sdk 词表（WorkflowStepStatus → WorkflowTaskStatus 等）、中间件、工具、agent-workflow spec、BDD——step 归运行时层。
- WorkflowTaskStatus 转移表（Pending→InProgress→{Completed, Failed, Blocked}；Blocked→InProgress 用户批准；Failed→InProgress 重规划重启；Completed 锁定，仅 replan 显式入口可改下游）：全部写入经转移表校验，非法拒绝（LLM 可见错误）。
- 容器阶段枚举 WorkflowPhase { Contract, Executing, AwaitingApproval, Replanning, SubWorkflow, Summarizing, Done }：九字段降为阶段附属数据；reload 与 after_tool 失败标记合一 mark_failed。
- approved 随步骤重跑清位（一次批准只解锁一个步骤）；Blocked 阻塞语义经转移表强制。
- 持久形状 v1 不改（CustomEntry 内字段名随 Task 更名，结构不变；Blocked 仍由 requires_human + approved 表达）。

## Capabilities

- agent-workflow（步骤图 / 局部重规划 / 审批条款改写为 Task 词汇 + 转移合法性）。

## Impact

- 范围：crates/xylitol-plugin-sdk/src/session/workflow.rs、plugins/xylitol-official-plugin-workflow。行为变更：模型对已完成步骤的改写从静默生效变为拒绝（对齐 src/AGENTS.md「已完成步骤锁定」既有产品规则，属纠偏）。

## 决策记录（2026-11-11，全部已决）

1. （已决）Step → Task 全线更名（SSOT：step 归运行时层；不留别名）。
2. （已决）转移表拒绝非法 + Completed 锁定（replan 显式入口唯一例外）；存量容忍投影。
3. （已决）容器阶段枚举 WorkflowPhase 收敛九字段；重置路径合一。
4. （已决）approved 随重跑清位；一次批准解锁一个步骤。
5. （已决）持久形状 v1 不改（仅字段随 Task 更名）。
