# Tasks — Host 会话槽生命周期

> 实施者导读：design §1 缺陷与 §2 修复一一对应；§3 泄漏走查图是验收叙事。收口走 finalize，不列为任务。

- [x] T1 spec 增补（design §4）
  server-core 增补 inflight 守卫与幂等淘汰终态化条款；infra-plugin 增补进程消亡 unavailable 条款。验证：llman-sdd validate server-core infra-plugin --strict。
- [x] T2 RunInflightGuard（design §2 第 1 条）
  CAS 构造 + Drop 复位；prompt / automation 两条占位路径接线；eviction 检查点改读 guard。单测：正常收尾、panic（catch_unwind）、提前 Drop 三路径均复位。
- [x] T3 writer 死标志删除
  移除 store(true) 与字段；grep 验证无残留引用；driver Option 为独占唯一载体。
- [x] T4 幂等淘汰终态化
  Reservation 增 finish_conflict；LEDGER_CAP 淘汰路径接线。单测：淘汰时进行中预留的重放得 Conflict；容量边界重复执行为零。
- [x] T5 ExternalClient 存活态
  EOF / 破管 → state=None + dead_reason；call 遇 None 报 unavailable。单测 + BDD（kill 后工具调用报错含死因）。
- [x] T6 automation 单一 admit 入口
  tick 产 Run 与 drain_pending 合并走 admit()；busy 判定来源唯一。单测：双路径行为等价。
- [x] T7 全量回归 [blocked-by: T1, T2, T3, T4, T5, T6]
  just fmt / just lint / just test；llman-sdd validate --all --strict；server-core busy 恢复 BDD。
