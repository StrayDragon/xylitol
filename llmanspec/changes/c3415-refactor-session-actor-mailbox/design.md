# Design：SessionActor 与三通道信箱

真源：c3410 design.md §2 / §4.1-4.2 / D1-D3；本文只写本 change 特有形状。

## SessionActor 解剖

- 每会话槽一个 actor task：select 主循环（信箱接收 → 路由）+ turn 任务（同 runtime 内 spawn，持 AbortHandle / JoinSet）。
- 命令信箱：有界 64（runtime-config 可调），满 → 发送方 await（内部背压，用户无感）。
- 路由表：Queued → 既有 PendingMessageQueue（收编，升级 QueuedItem{id, message}）；Exclusive → 忙碌即拒；Inline → 立即处理回执。
- turn 空闲晋级：AgentEnd 后弹下一条（r3304 语义保持）。

## 门面投影

XyInProcessDriver 重构为 handle：命令 Sender + watch::Receiver<StateSnapshot> + 流接收端工厂。同步 getter（session_id / current_model / thinking_level / get_commands / leaf_entry_id / has_active_turn 等）读快照；异步语句发命令 + oneshot；abort try_send 永不等待；run / run_turn 入队 turn 并返回事件流接收端（XyEvent 形状不变）。

## occupancy 上收

serve occupancy 的占用 / 宽限 / 驱逐状态机删除，槽生命周期归 SessionActor（handle 全 drop 即自杀语义）；journal / tape 机制保留；槽与 actor 所有权边界【细化】。

## seam

既有 BDD harness（主机泵 / ScriptedDriver / occupancy 测试组）复用；撤销等新场景 step 定义落 tests/bdd/steps_*。

## specs / BDD 绑定方案

- 无 live specs/** 编辑（合约已由 c3410 重写）；本 change 落地 c3410 新场景的 step 定义与 bindings。
- 落地清单（对应 c3410 design §5.1 / §5.7）：撤销两场景、「会话槽单写者物理化」场景、r1060 items 步骤改写、多端验收①②⑤——文件：tests/bdd/steps_*（复用主机泵 / ScriptedDriver 词表）+ bindings_agent_runtime.rs 追加。
- 红名单清零：c3410 T6 清单（r1060 / r1719 步骤改写 + 新场景绑定）在本 change 分支完成并实测 nextest。
- 相邻既有语义：app-tui-host r3368（MCP 准备期提交排队）为端侧排队先例，与本 change 的 host 侧信箱并存不冲突；occupancy 上收时保持该语义不变（T1 核对项）。
