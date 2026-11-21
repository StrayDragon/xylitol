# Tasks：SessionActor 与三通道信箱

> 依赖 change：c3410（合约语义）、c3412（干净表面）——假设二者特性已合入基线。needs_specs_change: false。
> 范围重定（2026-11-21）：原 T5（门面投影）/ T6（occupancy 上收）整体移入后续 change c3432-refactor-driver-actor-physicalization——实测阻塞于 AgentCapabilities 60+ 处 &mut self 语句，actor task 须独占可变能力集；c3410 合约场景已全绿，物理化不承载新用户语义。
> seam：主机泵 / ScriptedDriver BDD（c3410 新场景的 step 定义在本 change 落地）+ occupancy 测试组 + runtime 单测。

- [x] T1: 前置自检（强制门）——断言 c3412 特征在位（XySessionOps trait 存在、trait 无剪贴板 / attach 方法）与 c3410 specs 语义已合入基线；任一缺失 → STOP 并报告，不得自行补齐依赖实现。
- [x] T2: SessionActor 骨架——三通道路由表与命令词表落地（registry exec_class 逐类断言 + parse_command 48 方法覆盖 + serve 忙碌期 Inline 放行）；有界信箱的 actor task 物理化移入 c3432（见头部范围重定），队列语义由 PendingMessageQueue 承接。[blocked-by: T1]
- [x] T3: 队列收编——AsyncQueueRuntime / PendingMessageQueue 迁入 actor 所有权；QueuedItem{id, message}；既有 queue.rs 测试迁移对照（行为逐字节）。[blocked-by: T2]
- [x] T4: turn 任务接线——run / run_turn 入队与空闲晋级、steer drain 时机不变（r1059）、QueueUpdate items 事件经 event_tx、abort 树取消（r1028）。[blocked-by: T3]
- [x] T7: 撤销实现——revoke_queue_item 协议方法 + NotFound + QueueUpdate 广播；c3410 新场景 step 定义与 bindings 落地。[blocked-by: T4]
- [x] T8: 多端验收①②⑤——双客户端 / resync / 并发撤竞态 BDD。[blocked-by: T7]
- [x] T9: 全量验证——nextest + just qa 绿；c3410 红名单清零核对（在本 change 分支测量，相对 merge-base）。[blocked-by: T8]
