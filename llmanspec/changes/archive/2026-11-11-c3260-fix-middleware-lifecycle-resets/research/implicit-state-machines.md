# 中间件生命周期扫描证据（2026-11-10）

## M1 generator-critic 相位（高，explicit-scattered）

- 位置：plugins/xylitol-official-plugin-loop-policies/src/middleware.rs:117-151、:255-359（Lead 已复核 :255-365 原文）
- 载体：CriticState{phase: CriticPhase(Idle/Critiquing/Revising/Approved/Exhausted), rounds, revision, unparsed, instruction_injected}
- 转移点（复核确认）：:288 Critiquing→Approved（APPROVE）；:293 →Revising（REVISE）；:301 →Exhausted（连续 2 次解析失败）；:303 首次失败→Revising；:315 Idle|Revising→Exhausted（rounds>=max）；:318 →Critiquing（新助手回复无 tool_call）；:268/:274 注入置 instruction_injected；:296/:305/:320 清零；decide_continue(:344-358) Approved/Exhausted→stop
- 缺失：Approved/Exhausted 无回 Idle 转移、rounds 永不复位——首轮 APPROVE 后后续轮次被立刻 stop（第二问起模型得不到执行机会）；instruction_injected 三处分别清零无单一 reset

## M2 todo checkpoint 一次性闸门（高，implicit）

- 位置：plugins/xylitol-official-plugin-todo/src/middleware.rs:161-176、:427-449（Lead 已复核 :425-455 原文）
- 载体：CheckpointState{steps,last_verified_artifact,last_failure,attempts,dirty} + restored/injected 两个 AtomicBool
- 转移点（复核确认）：:432 restored = !swap(true) && ctx.session_id.is_some()——swap 先于条件执行；:436 injected = !swap(true) 无条件
- 缺失：首次调用无 session_id → restored 永久 true 且 restore 永不发生，无自愈；injected 同理（无 session 时仍注入）；dirty 与 steps 无版本号

## M3 goal/schedule 转移全开放（高，explicit-scattered）

- 位置：crates/xylitol-plugin-sdk/src/session/automation.rs:64-115（枚举）；plugins/xylitol-official-plugin-tools/src/automation.rs:128-224（Lead 已复核 :170-230）
- 载体：GoalState 7 态、ScheduleState 5 态 + iterations_used/ticks_used 计数
- 转移点：tools/automation.rs:131-143 新建目标把 Open|Running 改 Cancelled；:174-197 goal_set_state 只校验存在；:199-224 set_state 同样；:586-603 pause/resume/clear 宏固定写；投影 automation.rs:275-279 Active+耗尽→Exhausted；:337-345 非 Active 清 pending_trigger
- 缺失：无终态不可逆约束（Cancelled→Running 可达，Done/Failed 之后仍可 Pause）；Exhausted 派生与持久两套真值
- 关联（F3）：SDK 投影层 Paused 后 TriggerFired 仍写 pending_trigger（automation.rs:355 无 state 判定）；Cancelled 仍有 next_fire（:273-274）

## M4 automation 待办与忙判（中，implicit）

- 位置：crates/xylitol-runtime/src/agent/automation.rs:63（详见 c3250 research L3）；tick 与 drain_pending 两条放行路径靠下游 CAS 兜底。

## M5 todo reminder 三原子量（中，implicit）

- 位置：todo/middleware.rs:470-475、:522-553
- 载体：stale_tool_calls AtomicUsize + last_error AtomicBool + last_reminder_turn AtomicU32（哨兵 u32::MAX）
- 缺失：三原子量非原子组合，读时可拼出中间态；turn 回绕或从 0 重开时哨兵语义错

## M6 todo 进度字符串重解析（中，implicit）

- 位置：todo/middleware.rs:38-98
- 载体：WorkflowProgress 从工具输出 JSON 的 "status" 字符串解析，未知值静默归 pending(:94)
- 缺失：SDK 另有 TodoStatus 枚举，两处词汇表分叉；改状态名即静默失效
