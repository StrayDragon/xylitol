# ReAct 内环状态机扫描证据（2026-11-10）

## R1 RunCoordinator 运行相位（高，explicit-scattered）

- 位置：crates/xylitol-runtime/src/agent/runtime/state.rs:96
- 载体：phase RuntimePhase(98) + cancel Option<(RunId,CancellationToken)>(99) + active_turn(100) + pending VecDeque(101) + RunLease.finished(291)
- 状态集：Idle / Starting / Streaming / Cancelling；pending 深度；finished 为第三份状态
- 转移点：139 try_begin_immediate；150 mark_streaming；156 enqueue_pending；174 cancel_active；218 finish_run（早退时 cancel 不清、相位可停 Cancelling）；237 activate_next_pending；308 RunLease::finish；320 Drop
- 守卫：151/195/219 比对 run_id；309 幂等 finish；187 cancel_token() 空闲时返回永不取消的默认 token

## R2 RetryState 重试代数（高，implicit）

- 位置：retry.rs:74；调用点 react/mod.rs:1007（构造）、1235-1290（Lead 已复核）
- 载体：attempt: AtomicU32 + max_retries + base_delay_ms
- 转移点：102 next_delay fetch_add；89 can_retry；1242/1279 attempt()>0 即发 AutoRetryEnd
- 缺失：无 reset；attempt 每次 run 一份而非每轮重置 → 第 1 轮重试后第 2 轮未重试也发虚假 AutoRetryEnd（事件直达 TUI 通知条，host/mod.rs:1423-1424 归类为 tape 事件）
- 复核记录：Lead 已读 react/mod.rs:1235-1290 确认两处 attempt()>0 发射点均无条件于本轮是否真的重试

## R3 ReAct 轮次控制标志（高，implicit）

- 位置：react/mod.rs:1045
- 载体：turn_aborted bool（16 处写）+ continue_after_tools + stall_count + settle_count + overflow_recovery_attempted + compaction_floor_notice_emitted
- 转移点：1061/1085/1206/1213/1294/1391/1590/1597/1668/1698/1795/1920/1923/1991/1994/2037 置 aborted；1729-1742/1800 stall 越限终止；2041 清零；1806 settle 自增
- 缺失：取消/中间件故障/模型错误/图片模态不支持混为一谈；turn_end.rs:2081 只有 Aborted|Ok

## R4 嵌套调用计数（中，implicit）

- 位置：dispatch.rs:35；nested AtomicUsize + max_nested
- 转移点：117 fetch_add；119/129/135/141/155 五处手工 fetch_sub
- 缺失：无 RAII guard，新增早退路径漏减即永久占额（codeme 全废）

## R5 最大轮次计数器（中，implicit）

- 位置：hooks.rs:24；settles AtomicU32 闭包捕获 + max_turns
- 转移点：26 fetch_add；27 达阈返回 true；消费 turn_end.rs:143
- 缺失：跨 run 不复位（hook 在 kernel/bootstrap.rs:830 只装一次），run2 立即触顶

## R6 Bang 命令取消槽（中，implicit）

- 位置：driver/bang_exec.rs:13；cancel Arc<Mutex<Option<CancellationToken>>>
- 转移点：41 execute 置 Some；59 完成置 None；68-70 abort take 后取消
- 缺失：并发执行时后者覆盖前者，前者失去取消句柄；panic 时槽残留
