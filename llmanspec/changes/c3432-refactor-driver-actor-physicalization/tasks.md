# Tasks：驱动物理化（路线 B 分阶段）

> 依赖：无（c3415-c3430 已归档）。needs_specs_change: false。
> seam：runtime/kernel 单测 + occupancy 测试组 + 全量 BDD 回归地板。

- [ ] T1: 快照骨架——DriverSnapshot 定义 + watch 通道接入 XyInProcessDriver；同步 getter（session_id / current_model / thinking_level / get_commands / leaf_entry_id）先双读（锁优先快照回落），行为不变回归。
- [ ] T2: 字段锁化第一批——tools 三件（tools / tool_freeze / tool_fingerprint）与 batch_mode 内部锁化；对应 set_* 改 &self；每步全量单测。[blocked-by: T1]
- [ ] T3: 字段锁化第二批——system_prompt 两件 / session_id / cwd / workspaces；set_* 改 &self。[blocked-by: T2]
- [ ] T4: 字段锁化第三批——stage_name 两件 / compaction 两件 / loop_scope / hooks；hooks_mut 语义改为置换式。[blocked-by: T3]
- [ ] T5: 提交门收口——submit_turn 改 &self（admission 只依赖 coordinator + 探针）；AgentRuntime 全面 &self 化清理；E0596 全清点。[blocked-by: T4]
- [ ] T6: 快照单源——同步 getter 切纯快照读；变更点 publish 快照；staleness 单测。[blocked-by: T5]
- [ ] T7: occupancy 生命周期收敛——槽测试改经 handle 引用计数表达"全 drop 即停"；linger/eviction 测试不降级。[blocked-by: T6]
- [ ] T8: 全量验证——just qa + e2e + thoughtdag lockstep 编译测试；路线 A 挂起决策回写本提案。[blocked-by: T7]
