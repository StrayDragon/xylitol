---
depends_on: ["c3415-refactor-session-actor-mailbox"]
needs_specs_change: false
title: 驱动 actor 物理化：门面投影与 occupancy 上收（c3415 原 T5/T6 移入）
---

## Why

c3410 重写的执行合约（三通道路由、逐项撤销、单写者语义）已由 c3415 落地且场景全绿，但两项内部物理化因所有权改造未完成而移出：

1. XyInProcessDriver 仍是"直接持有 AgentRuntime"的胖对象；合约要求的 handle 形态（命令 Sender + watch 快照）未落地。
2. serve occupancy 槽状态机（linger / eviction / turn_inflight 守卫）与 runtime 内 coordinator 状态机存在双份生命周期管理。

## 移入范围（原 c3415 T5/T6 原文）

- T5 门面投影：XyInProcessDriver → handle（Sender + watch 快照）；同步 getter / 异步语句 / abort / run 流五类改造；get_state 默认实现语义回归。
- T6 occupancy 上收：serve 槽状态机删除，journal / tape 保留对接；槽生命周期归 actor（handle 全 drop 即自杀语义）。

## 实测阻塞证据（2026-11-21，c3415 分支）

- AgentCapabilities 有 60+ 处 &mut self 语句（select_model / set_thinking_level / generate_session_name 等）；把 AgentRuntime.inner 换成 Arc 会让这些语句全部失效（实测 66 处编译错误）。
- 排队 turn 须由 actor task 独占可变能力集驱动；当前 XyEventStream 为调用方轮询式（async_stream 惰性 + TurnLease::Drop 收 turn），actor 化须先决定事件驱动权归属，否则 abort / drop 语义漂移。
- serve 槽经 Arc<Mutex<Option<Box<dyn OccupancyEngine>>>> 持有驱动；OccupancyEngine 表面约 50 方法，逐一改命令 + oneshot 为大样板改造。

## 实施前置

- 先把 AgentCapabilities 可变语句改为内部锁或消息式（独立小步，可先行验证）。
- 决定 XyEventStream 轮询权：调用方轮询 + actor 转发通道，或 actor 驱动 + 调用方仅订阅（须保 TurnLease 语义）。
- BDD 全绿为回归基线；occupancy 测试组（48 例）为 T6 验收面。

## Further Notes

- 本 change 不承载新用户语义；specs 无改动（needs_specs_change: false）。
- c3415 头部"范围重定"记录了移出决策与理由。
