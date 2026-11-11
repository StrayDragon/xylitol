# Workflow 状态扫描证据（2026-11-10）

## W1 WorkflowState 九字段组合（高，explicit-scattered）

- 位置：plugins/xylitol-official-plugin-workflow/src/middleware.rs:77-89（Lead 已复核 :77-110 结构）、:164-193、:265-472
- 载体：WorkflowState{session_id, plan: Option<Workflow>, stack: Vec<(Workflow,String)>, approved: HashSet<String>, replan_count, contract_injected, injected_step: Option<String>, summary_injected, summary_done, pending_workflow_error} + loaded AtomicBool(:111)
- 状态集合：契约期(plan=None) / 执行期 / 阻塞待人工 / 失败待重规划 / 子流程嵌套 / 总结期 / 完成（summary_injected×summary_done 两 bool 编码「总结中/总结完」）
- 转移点：:265-267 loaded 一次性置位；:277-281 注入契约；:286-297 blocked→注入阻塞指令；:299-311 Failed→重规划指令；:313-335 全完成弹栈或 plan_complete；:339-356 sub_workflow push+换 plan；:387-399 工具报错置 Failed 清 approved/injected_step；:415-419 新用户消息→approved.insert；:434-445 重规划耗尽弹栈标容器 Failed；:449-465 summary_done 后 stop；:164-193 reload 重置 6 字段并可能 replan_count+=1
- 缺失：无单一 transition；approved 永不清空（步骤重跑仍算已批准）；reload 的 had_failed 判定与 after_tool 置 Failed 是两条独立路径

## W2 WorkflowStepStatus 写入散落（高，explicit-scattered）

- 位置：crates/xylitol-plugin-sdk/src/session/workflow.rs:20-29（枚举）+ :51 requires_human、:47/:55 verification/output_ref、:68 resume_cursor 旁路字段
- 写入点：middleware.rs:319（子流程完成→Completed）、:347（容器步进子流程→InProgress）、:393（工具失败→Failed）、:439（重规划耗尽→容器 Failed）、tools.rs:325（workflow_fail→Failed）
- 守卫：tools.rs:318-323 仅拦 Completed→Failed；validate_workflow_replan（workflow.rs:309-399）只管重规划不改已完成步骤
- 缺失：Pending→Completed 直跳、Failed→InProgress 回退无校验；Blocked 由 requires_human+approved 隐式表达
