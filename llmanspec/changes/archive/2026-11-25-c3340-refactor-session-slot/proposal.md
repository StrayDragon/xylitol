---
depends_on: []
title: 会话槽的占用、宽限和驱逐由槽自己完成
needs_specs_change: false
branch: sdd/c3340-refactor-session-slot
base_branch: main
base_sha: 3990b6dbc4c5b1c019519b05a6a934cfe08a6280
---

# 会话槽的占用、宽限和驱逐由槽自己完成

## Why

`server-core` 的 r1793、r1806、r2932 已经规定写占用释放、序号水位续接，以及回合进行中不驱逐。现有转移在模型里守住这三条。

转移写在 `host.rs` 的自由函数里。放写锁和 `evict_slot` 在立即驱逐和宽限到期两处各写一遍。水位记在 `HostState` 上。再添一种生命周期事件时，调用方可以只做其中一步。

## What Changes

立即驱逐和宽限到期调用同一个转移。该转移在同一次里记下水位、释放写占用、停掉 automation、释放 driver，并从会话表移除。

水位表仍在 `HostState`。它比槽活得久。槽不复制一份水位。

本 change 不拆 unary 方法表，不改 `envelope` 与宿主的调用关系，也不把 `oauth` 手里的整个 `SessionSlot` 收成发送端。

## Capabilities

- `server-core`：r1793、r1806、r2932 已写明这些行为。本 change 不改规则文本。

## Impact

- 代码：`crates/xylitol-serve/src/occupancy/host.rs` 里的生命周期函数，以及 `tests/occupancy/` 里已有的释放、宽限和水位场景。
- 验证：这些已有测试保持通过。不新增 BDD 场景。`research/model.md` 里的四条规则在转移收成一处之后仍然成立。
- 与另外两份草案无文件重叠，无 `depends_on`。

## Further Notes

模型假设宽限大于 0。宽限为 0 时，非干净断开走立即驱逐，与干净关闭同一条转移。模型不含他进程的内核锁，也不含心跳判死。心跳判死进入同一条非干净断开路径。细节在 `research/model.md`。
