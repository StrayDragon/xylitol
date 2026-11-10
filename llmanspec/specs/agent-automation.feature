# language: zh-CN
# purpose: 会话级自动化：schedule/trigger/run 事实、interval/cron 触发、单写者准入与失败关闭权限。

功能: agent-automation

  @req:r3288
  规则: automation-facts
    System 须将 schedule / trigger / run 事实以 `custom_type = "automation"` 的 CustomEntry 追加到会话分支；状态 须由事实投影重建，不得覆盖历史；事实 须按 branch_id 过滤；fork 子分支 不得继承父 schedule；未知 kind 须忽略并告警，不得阻塞旧 session 加载。

  @req:r3289
  规则: schedule-cadence
    schedule cadence 须支持 interval 与 5-field cron；cron 使用本地时区且不支持秒；cron 不得自动过期；schedule 须受 max_ticks 硬上限（缺省 256）与手动 pause / resume / cancel 约束；ticks_used 只计 RunAdmitted，不计 skipped / coalesced。

  @req:r3290
  规则: automation-admission
    AutomationEngine 须按 session 槽运行；到期 tick 须经同一 admission 启动 run；session busy 时 须排 follow-up，单 pending，重复到期 coalesce，不得 steer；重启后 不得补跑，须按当前时间重算 next_fire。

  @req:r3291
  规则: automation-fail-closed
    无人值守 tick 须 fail-closed：需要交互确认但无 client 时 须跳过并记录 RunSkipped，不得自动批准；RunSkipped 须写原因，不得静默丢 tick；pause / cancel 后 pending trigger 须丢弃并记录。

  @req:r3292
  规则: automation-tools
    模型工具 schedule_create / schedule_list / schedule_pause / schedule_resume / schedule_cancel 须只追加事实并投影当前 branch 状态，不得直接启动 run；非法 cadence 须返回可分类错误；schedule_* 工具 不得扩大核心 7 工具闭集。
