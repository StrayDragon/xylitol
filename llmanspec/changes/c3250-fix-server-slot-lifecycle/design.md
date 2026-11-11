# Design — Host 会话槽生命周期（全部已决 2026-11-11）

## 0. 槽是什么

serve 为每个会话维护一个 SessionSlot（host.rs:82-110）：事件 journal、driver 引擎、订阅者表、幂等台账、以及会话独占标志（run_inflight）。它是「一个 runtime 绑一个 session、单写者」硬约束在 host 侧的载体。

## 1. 缺陷清单（已核实；泄漏路径为静态推断，标注在案）

1. **run_inflight 无 RAII 守卫**：CAS 占位两处（prompt :1670-1672、automation :1290），清位三处（:1699/:1728/:1315）全部位于「正常收尾路径」。spawn 的 run 任务 panic、或事件流未走到 AgentEnd 即被丢弃时，标志永久 true——后续 prompt 恒拒 session_busy，不可自愈，只能重启进程。（泄漏路径静态推断，运行时复现未验证；按「不可达」标准修复。）
2. **writer 死标志**：materialize_writer_at（:1225）是全仓唯一写点，无任何读点；独占语义实际由 driver Option 承载。
3. **幂等淘汰破坏幂等**：LEDGER_CAP 淘汰直接 remove 条目，进行中的 Reservation 仍持 Arc——同 id 重放拿到新条目，重复执行。
4. **ExternalClient 僵尸态**：子进程 EOF / 管道破裂只返回错误，state 仍是 Some；后续请求报「exited before responding」而非「不可用」。
5. **automation 双放行路径**：tick 产 Run 与 drain_pending 各自放行，靠下游 CAS 兜底而非单一入口。

## 2. 修复（定稿）

| 修复 | 内容 |
|---|---|
| RunInflightGuard | CAS(false→true) 成功即构造，Drop 复位；持有方 = run 事件流消费循环；tokio abort 走 unwind 同样触发 Drop。eviction 检查点（:1478/:1504/:1546）改读 guard 存在性 |
| 看门狗 | 不加——RAII 已覆盖全部退出路径；超时阈值属新魔法数 |
| writer 标志 | 直接删除；独占语义由 driver Option 单一承载 |
| 幂等淘汰 | 淘汰时对进行中 Reservation 显式 finish_conflict（复用 watch 通道）；重放方拿到 Conflict 而非重复执行 |
| ExternalClient | EOF / 破管 → state = None + dead_reason；请求遇 None 报 unavailable（含死因）；不自动重启（配置驱动原则；重启策略另立 change） |
| automation 放行 | 合并为单一 admit() 入口，tick 与 drain 共用；busy 判定来源随之唯一 |

## 3. 泄漏场景走查（附录）

~~~mermaid
sequenceDiagram
  participant P as prompt 请求
  participant S as run_inflight
  participant R as spawn 的 run 任务
  participant E as 事件流
  P->>S: CAS false→true（占位）
  P->>R: 启动运行
  R-->>E: 流式产出事件
  Note over R: 任务 panic 或流被提前丢弃<br/>（未走到 AgentEnd 清位分支）
  S-->>S: 永久 true（现状）
  P->>P: 之后每次 prompt：session_busy，拒绝<br/>不可自愈
  Note over S: 修复后：guard 随消费循环 Drop<br/>任何退出路径（含 unwind）自动复位
~~~

## 4. spec delta（landing 时）

- server-core：增补「槽 inflight 泄漏不可达（守卫随流生命周期）」与「幂等淘汰须终态化进行中预留（Conflict）」条款。
- infra-plugin：增补「外进程插件进程消亡后请求须显式报 unavailable（含死因）」。

## 5. 测试边界（seam）

- 单测：guard Drop 复位（含 catch_unwind panic 路径）；幂等淘汰时重放得 Conflict；ExternalClient kill 后 call 报 unavailable；admit 单一入口的 busy 判定。
- BDD：server-core busy 恢复（泄漏注入后可再次 prompt）；infra-plugin 进程消亡报错。

## 6. 风险与回滚

- 风险：guard 生命周期若跨 await 挂到过长任务会拉长 busy 窗口——guard 只包事件流消费循环，不包整个 session。
- 回滚：guard / writer 删除 / 幂等 / ExternalClient / admit 五片独立。
