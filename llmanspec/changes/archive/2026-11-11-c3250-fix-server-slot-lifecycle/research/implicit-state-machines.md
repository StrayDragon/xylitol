# Host 槽生命周期扫描证据（2026-11-10）

## L1 SessionSlot 会话独占相位（高，implicit）

- 位置：crates/xylitol-serve/src/occupancy/host.rs:82（Lead 已复核 :82-110 字段定义与全部 run_inflight/writer 触点）
- 载体：run_inflight AtomicBool(94) + writer AtomicBool(86) + resync_offered(93) + mcp_watch_started(98) + linger_task(89) + pending_model/pending_thinking(95-96)
- 状态集：idle / run-inflight / writer-materialized / linger-scheduled / resync-offered / settings-deferred
- 转移点：1670-1672 prompt CAS(false→true)；1290 automation CAS；1699/1728/1315 store(false)；1225 writer.store(true)（全仓唯一写点，无任何读点——grep 复核）；554/562 resync_offered；771 mcp_watch swap；1512 linger_task=Some
- 守卫：两处 CAS 防重入；1478/1504/1546 读 inflight 挡 eviction
- 缺失：run_inflight 无 RAII guard——spawn 任务 panic 或流未到 AgentEnd 即丢弃则永久 true（后续 prompt 恒 session_busy，不可自愈）；writer 死标志与 driver Option 状态重复

## L2 幂等台账预留（中，implicit）

- 位置：crates/xylitol-serve/src/occupancy/idempotency.rs:46
- 载体：Reservation.done Cell<bool>(48) + Entry.tx watch(14) + Admission 枚举(69)
- 状态集：reserved(pending) / completed / abandoned
- 转移点：52 complete；58-67 Drop 兜底 internal_error；94-128 admit 判 Proceed/Replay/Conflict；114-119 LEDGER_CAP 淘汰
- 缺失：淘汰直接 remove 条目而 Reservation 仍持 Arc，同 id 重放拿到新条目重复执行；无超时取消

## L3 自动化待办与忙判（中，implicit）

- 位置：crates/xylitol-runtime/src/agent/automation.rs:63
- 载体：pending Option<PendingTrigger>(63) + started_at(64) + AutomationIndex.goal_active/goal_done bool(41-42) + 外部注入 busy/permission_required bool
- 转移点：238 入队；266 drain_pending 取出；197 goal_active 跳过；211 permission 跳过；225 busy 分支；消费方 host.rs:1269 drain、1277 run_automation 用 run_inflight CAS 再挡
- 缺失：tick 产出的 Run 与 drain_pending 是两条独立放行路径，靠下游 CAS 兜底而非单一入口

## L4 外进程插件客户端存活态（中，implicit）

- 位置：crates/xylitol-kernel/src/external_plugin.rs:82
- 载体：state Mutex<Option<ClientState>>(82) + next_id AtomicU64
- 状态集：running(Some) / shutdown(None) / 已死但仍在 Some（僵尸）
- 转移点：111 spawn→Some；143-146 None 报 not running；179-181 读到 EOF 只返错不转 None；201-206 shutdown take
- 缺失：EOF/管道破裂后状态未迁移，后续请求报 exited before responding 而非不可用
