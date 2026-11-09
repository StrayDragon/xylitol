---
depends_on:
  - c3206-refactor-session-graph
title: "Add loop scheduler"
---

## Why

/loop 是执行方式：按固定间隔或 cron 反复触发 run，直到手动停、达到停止条件或触达最大次数。它不等于 /goal；/goal 是任务定义与终止标准，/loop 是执行节奏。

当前没有调度器：run 只由用户提交触发。c3206 提供 branch 身份、图边与单写者准入，但没有 trigger 事实与 scheduler。

## What Changes

- Schedule 事实：id、branch、cadence、next_fire、status（active / paused / exhausted / cancelled / failed）、max_ticks、stop_condition。
- Trigger 事实：由 host scheduler 追加，含 schedule_id、fired_at、reason、outcome；经单写者准入启动 run。
- Cadence：interval + 5-field cron；本地时区；第一版不做条件 DSL。
- 停止：手动 pause / cancel；max_ticks 缺省 256；连续失败退避；stop_condition 保留字段但不做条件引擎。
- busy：follow-up 队列，单 pending，重复到期 coalesce；不 steer。
- 事实 append-only；状态由事实投影重建；host 只维护派生索引；重启不补跑。
- LLM 工具：schedule_create / schedule_list / schedule_pause / schedule_cancel。
- TUI 后置；nsh 不做。

## Automation Core（与 c3208 共享）

- 共用事实：ScheduleDefined、SchedulePaused、TriggerFired、RunAdmitted、RunSkipped；c3208 追加 GoalDefined、GoalEvaluated。
- 会话分支 custom facts 是唯一真源；host 的 AutomationIndex 只是派生缓存。
- schedule tick 与 goal 续跑都走同一 admission；不另造执行路径。
- 事实按 branch_id 过滤；fork 不继承 schedule/goal。
- goal 活跃时跳过同 branch 的 loop tick；goal done 时暂停关联 schedule。

## Scope

- 不在 c3206 内实现；c3206 只提供 branch 与事件基础。
- 不做条件 DSL 的工作流引擎化；不做 TUI 侧栏；不做 nsh。
- 第一版只暴露 LLM 工具；TUI 状态展示后置。

## Related

- #19（Loop 管理与触发可视化，已关闭；需新 FR 或本 draft 兑现时新开）
- #71（Guarded Decision Reuse；相关但不是 /loop）
- #73（TUI 使用痛点）
- #74（nsh 原生端）

## Open Questions

- 安静时段 / 勿扰自动暂停。
- 连续失败退避与告警。
- cron 是否允许 7 天自动过期；还是只由 max_ticks 与手动停约束。

## Research

- [Automation Core 架构与 harness 选型](research/automation-architecture-2026-11.md)
