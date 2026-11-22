# Tasks：驱动物理化（路线 A 分阶段）

> 依赖：无。needs_specs_change: false。
> seam：runtime/kernel 单测 + occupancy 测试组 + 全量 BDD 回归地板。
> 前置：c3433（驱动同步面 async 化）已归档，trait 面具备 actor 串行条件。

- [x] T1: A0 分桶与快照骨架——`self.agent.` 调用三分类分桶（只读走 `DriverSnapshot`、共享走 `SharedSubsystems`、独占走 mailbox）；`DriverSnapshot` 投影接入。
- [x] T2: A0 实测——惰性流 drop 中途的 steer 去向实测，结论回写 design 第 1 点。
- [x] T3: A1 actor 脊柱——懒启动 actor + 有界信箱（64）；`run`/`run_turn` 经 mailbox 独占化；非 Idle 快速拒绝保持（`concurrent_run_rejects_second_with_busy` 绿）；`run_actor` 命令间把 runtime 交回 `RuntimeSlot`，使 `&mut self` 路径仍可独占。
- [x] T4: A1 回归——四家消费方（serve/print/TUI/BDD）全量绿；abort 经 `SharedSubsystems` 直达 coordinator。
- [x] T5: A2 独占操作迁移——模型选择/循环/思考级别/compaction 经 mailbox；会话绑定/切换/fork/标题因依赖 driver 侧状态（`reload_cwd`/`session_profile`/`prepare_hub`）保持 inline。
- [x] T6: A3 occupancy 上收——`SessionSlot.driver` 改为 `SessionDriver` handle；`evict_slot` 走 `release_and_join`，linger/eviction 收敛为「丢 handle + 等 actor 退出」；occupancy 48 例与 serve 157 例全绿。
- [x] T7: 收尾——同步 getter 切快照；`just qa` 与 `just qa-e2e` 实跑绿。
