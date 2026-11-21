# Tasks：驱动物理化（路线 A 分阶段）

> 依赖：无。needs_specs_change: false。
> seam：runtime/kernel 单测 + occupancy 测试组 + 全量 BDD 回归地板 + thoughtdag lockstep。

- [ ] T1: A0 分桶与快照骨架——64 处 self.agent. 调用三分类分桶；DriverSnapshot + watch 通道接入；同步 getter 双读（快照优先回落直读）。
- [ ] T2: A0 实测——惰性流 drop 中途的 steer 去向实测（局部 pending 变量追踪），结论回写 design 第 1 点。[blocked-by: T1]
- [ ] T3: A1 actor 脊柱——懒启动 actor + 有界信箱；RunTurn 独占化（非 Idle 拒绝语义保持）；事件有界通道转发；订阅端流替换 XyEventStream 外壳；drop→取消路由 + 专项测试。[blocked-by: T2]
- [ ] T4: A1 回归——四家消费方（serve/print/TUI/BDD）全量绿；abort 经 coordinator 直达验证。[blocked-by: T3]
- [ ] T5: A2 独占操作迁移——会话绑定/切换/fork、标题生成、hooks 置换、compaction 安装逐个命令化；reload 钩子重入审计与签名改造。[blocked-by: T4]
- [ ] T6: A3 occupancy 上收——serve 驱动容器改 handle；linger/eviction 改丢 handle 语义；48 例 occupancy 测试改造。[blocked-by: T5]
- [ ] T7: 收尾——同步 getter 切纯快照；E0596 全清；just qa + e2e + thoughtdag lockstep 实跑；决策与实测结论回写本 change。[blocked-by: T6]
