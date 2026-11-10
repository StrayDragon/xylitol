---
depends_on: []
title: "Fix serve session slot lifecycle flags"
---

# Host 会话槽生命周期标志修复

## Why

xylitol-serve 的 SessionSlot 用一组无守卫的原子 / Mutex 字段拼出会话独占相位，存在**永久卡死**与**幂等失效**两类缺陷（扫描证据见 research/implicit-state-machines.md，A2/A10/A11/A17；Lead 已复核 host.rs 原文）：

1. **run_inflight 无 RAII 守卫**：prompt 路径 CAS(false→true) 占位（host.rs:1670-1672），若 spawn 的 run 任务 panic 或事件流被提前丢弃而未走到 AgentEnd 清理（:1699/:1728/:1315），标志永久 true，后续 prompt 恒拒 session_busy，**不可自愈**。
2. **writer 标志只写不读**：materialize_writer_at（:1225）store(true) 后全仓无任何 load——死标志，与 driver Option 状态重复。
3. **幂等台账淘汰失效**：idempotency ledger 容量淘汰直接 remove 条目，而进行中的 Reservation 仍持 Arc；同 id 重放拿到新条目重复执行。
4. **外进程插件僵尸态**（kernel/external_plugin.rs:82）：子进程 EOF / 管道破裂后 state 仍是 Some，后续请求报「exited before responding」而非「不可用」。

## What Changes

- run_inflight 改 RAII guard（Drop 复位）或与事件流生命周期绑定；流丢弃 / panic 路径自动释放槽。
- 移除 writer 死标志，独占语义由 driver Option 单一承载（或 writer 变为有读取方的真断言）。
- 幂等台账：淘汰时对进行中 Reservation 显式终态化（Conflict 或等待），容量边界不破坏幂等保证。
- ExternalClient EOF / 破管时显式迁移到 shutdown 态并上报不可用；可选自动重启策略。
- linger / eviction 路径回归：inflight 期间不得逐出槽（现有 :1478/:1504/:1546 检查保留）。

## Capabilities

- server-core（每 session 槽独立 journal 与写者）；关联 infra-plugin（外进程插件桥）。

## Impact

- 范围：crates/xylitol-serve/src/occupancy/{host,idempotency}.rs、crates/xylitol-kernel/src/external_plugin.rs。
- 行为变更：无外部契约变化；run_inflight 泄漏修复后 busy 语义恢复正确。
- 验证：模拟流丢弃 / 任务 panic 的槽回收测试；幂等重放在容量边界的行为测试；外进程插件 kill 后工具调用报 unavailable 的 BDD。

## Open Questions

1. 槽泄漏兜底：仅 RAII 是否足够，还是加 inflight 看门狗（超时强制释放）？
2. 幂等淘汰语义：进行中条目淘汰时重放应 Conflict 还是等待原结果？
3. 外进程插件崩溃是否自动重启（当前仅报错）？
