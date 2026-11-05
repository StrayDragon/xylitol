---
depends_on:
  - c3206-refactor-session-graph
title: "Add loop scheduler"
---

## Why

/loop 是执行方式：按固定间隔或条件反复触发 run，直到手动停、达到停止条件或触达最大次数。它不等于 /goal；/goal 是任务定义与终止标准，/loop 是执行节奏。

当前没有调度器：run 只由用户提交触发。c3206 提供 branch 身份、图边与单写者准入，但没有 trigger 事实与 scheduler。

## What Changes

- Schedule 事实：id、branch、cadence（interval / cron / condition）、next_fire、status（pending / running / paused / failed）、max_ticks、stop_condition。
- Trigger 事实：由 host scheduler 追加，含 schedule_id、fired_at、reason；经单写者准入启动 run。c3206 未落 trigger 字段，本 change 需新增事实类型。
- 停止：手动 pause / delete；max_ticks 上限（缺省 256，可配置）；stop_condition 满足；连续失败退避。
- 每次 run 追加事实，不覆盖历史；loop 状态可由事实投影重建。
- UI：下次触发时间、状态、上次结果（TUI / nsh 同源）。

## Scope

- 不在 c3206 内实现；c3206 只提供 branch 与事件基础。
- 不做条件 DSL 的工作流引擎化；条件先限定为简单谓词。

## Related

- #19（Loop 管理与触发可视化，已关闭；需新 FR 或本 draft 兑现时新开）
- #71（Guarded Decision Reuse；相关但不是 /loop）
- #73（TUI 使用痛点）
- #74（nsh 原生端）

## Open Questions

- cadence 是否支持 cron；条件触发的最小谓词集合。
- 安静时段 / 勿扰自动暂停。
- 连续失败退避与告警。
- 与 /goal 的组合：loop 负责触发，goal 负责判定完成。
