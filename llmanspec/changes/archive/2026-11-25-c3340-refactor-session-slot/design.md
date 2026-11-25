# Design: c3340-refactor-session-slot

## 目标

立即驱逐和宽限到期走同一次转移。这次转移记下水位、放开写占用、停掉 automation、释放 driver，并从会话表移除。

## 两种形状

| | 形状 A | 形状 B |
|---|---|---|
| 转移放在 | `host.rs` 里一个函数，两个调用点都进它 | `SessionSlot` 的方法，方法再去拿 `HostState` 上的水位表和 store |
| 水位 | 仍由 `evict_slot` 写到 `HostState.seq_watermarks` | 槽上再留一份水位，驱逐时抄回 `HostState` |
| 前置条件 | 回合进行中的跳过、宽限到期时的订阅复查，留在现在的调用点 | 前置条件收进槽方法 |

## 选择

用形状 A。

`evict_slot` 已经记下水位、停 automation、释放 driver 并移除槽。它不放开写占用。两个调用点今天各自拿 `eviction_gate`，再 `release_write_lock`，再 `evict_slot`。收成一个函数，里面按这个顺序做后两步。`release_and_evict` 和 `linger_evict_if_idle` 在自己的前置条件之后调用它。

水位表留在 `HostState`。槽不复制一份。

unary 方法表、`envelope` 与宿主的调用、`oauth` 手里的 `SessionSlot` 都不动。`host.rs` 的行数不作为完成判据。

## 完成时的读法

`research/model.md` 里的写锁释放、水位保留、回合进行中不驱逐，在收成这一处之后仍然成立。`tests/occupancy/` 里已有的释放、宽限和水位场景保持通过。
