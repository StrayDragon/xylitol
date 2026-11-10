---
depends_on:
  - c3206-refactor-session-graph
  - c3207-add-loop-scheduler
title: "Add goal tracking"
branch: sdd/c3208-add-goal-tracking
base_branch: main
base_sha: 694349a19403bb10828ef4e5f77a3a4f0d9502d6
---

## Why

/goal 是任务定义与终止标准：围绕一个目标规划步骤、持续迭代，直到完成或失败。它不是 /loop；/loop 管怎么重复，/goal 管要达成什么。

必须有硬上限，不能无限迭代：goal 事实带 max_iterations（缺省 256，可配置），每轮结束检查是否达成或触达上限。

## What Changes

- Goal 事实：id、branch、objective、acceptance、status（open / running / done / failed / blocked / paused / cancelled）、max_iterations、iterations_used、evidence。
- 每轮结束由独立 evaluator 判定：met / not_met / impossible；第一版 evaluator 继承当前 session model；agent 须附 evidence；未达成则继续，达到 max_iterations 标记 failed / blocked，不得无限续跑。
- 续跑在同一 run 内通过 DecideContinue 完成；不新开 run。
- 用户可 pause / cancel / 修改 acceptance；状态变更只追加事实，不覆盖。
- goal 活跃时跳过同 branch 的 loop tick；goal done 时自动暂停关联 schedule。
- 一个 branch 同时最多一个 active goal。
- LLM 工具：goal_set / goal_status / goal_pause / goal_resume / goal_clear。
- TUI 后置；nsh 不做。

## Automation Core 依赖

- 复用 c3207 的 Automation Core：事实模型、host 派生索引、单一 admission。
- c3208 追加 GoalDefined、GoalEvaluated；TriggerFired 的 source 可以是 turn_end。
- goal 续跑不绕过 runtime 的 turn 边界、取消与权限终裁。

## Scope

- 不在 c3206 内实现；c3206 只提供 branch 与图边基础。
- 不做自动改写 acceptance 或无限自我迭代。
- 不做 TUI 侧栏；不做 nsh。

## Related

- #28（roadmap 索引；当前无 /goal FR）
- #71（决策复用；可作为 goal 推进的省 token 手段）
- #19（loop 管理，已关闭）

## Open Questions

- acceptance 的结构化格式与证据类型。
- evaluator 后续是否切换小快模型；是否允许用户 override。
- 与 todo / workflow 的边界：goal 是终止标准，不是步骤清单。

## Research

- [Automation Core 架构与 harness 选型](../c3207-add-loop-scheduler/research/automation-architecture-2026-11.md)
