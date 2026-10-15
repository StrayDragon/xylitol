# c2924 设计：RuntimePhase 迁移表与守卫编码

## 转移真值（源码勘察结论，`src/agent/runtime/state.rs`）

```
Idle           --begin(guard=phase_idle)        --> Starting
Starting(id)   --stream(guard=run_id_match)     --> Streaming
Starting(id)   --cancel(guard=always)           --> Cancelling
Streaming(id)  --cancel(guard=always)           --> Cancelling
Starting(id)   --finish(guard=run_id_match)     --> Idle
Streaming(id)  --finish(guard=run_id_match)     --> Idle
Cancelling(id) --finish(guard=run_id_match)     --> Idle
Idle           --begin(guard=phase_busy)        --> Idle        [拒绝相: None, 相不变]
Starting(id)   --stream(guard=run_id_stale)     --> Starting    [守卫失败相不变形]
```

## E1-E4 映射要点

- **E1**：given 绑 `<from>`、when 绑 `<event>` 与 `<guard>`、then 绑 `<to>`（`validateTransitionScenario` 的 guard 占位符规则：given 或 when 持 guard）。
- **E2 确定性**：`(from,event,guard)` 三元唯一——拒绝相/守卫失败相靠 guard 列区分（`phase_busy` vs `phase_idle`、`run_id_match` vs `run_id_stale`），同键不冲突。
- **E3 可达**：首行 `Idle` 为 initial，全态可达。
- **E4 死锁**：`Idle` 无出边？——否，Idle 有 begin 出边；终态判定走「Idle 可达即可」：所有态经 finish 回 Idle，`Idle→Starting` 成环，无孤岛无死锁。

## BDD 步骤形态

```rust
#[given("运行协调器处于 {from:string} 阶段")]   // 造初态; run_id 固定 R1, 另造 R2 供 stale guard
#[when("协调器事件 {event:string} 且守卫 {guard:string}")]  // begin/stream/cancel/finish 按 guard 分支
#[then("协调器阶段为 {to:string}")]             // phase() 观测, Debug 小写
```

- `RuntimePhase` 含 `Debug`（已有 derive），无需改产品码。
- stale 守卫相需双 run_id：given 预置「R1 活动 + R2 备用」，when 按 guard 选 R1/R2。
- binding sync fn（c2921 铁律）。

## 权衡记录

- 备选「经 agent 全链路驱动」被否：turn 级事件太粗，无法精确停在对 `Starting→Cancelling` 等中间相，且 mock 时序脆弱；单元直驱与 r1786 同构，守卫相才可表达。
- 拒绝相编码为自环（`Idle→Idle`）而非独立断言：E2 视角下「事件发生但相不变」就是合法转移行，避免表外特殊 case。
