---
depends_on: []
needs_specs_change: false
title: 驱动物理化：门面投影与 occupancy 上收（c3415 原 T5/T6 移入）
---

## Why

c3410 重写的执行合约已由 c3415-c3430 落地且场景全绿，但两项内部物理化因所有权改造未完成而移出：

1. XyInProcessDriver 仍是"直接持有 AgentRuntime"的胖对象；合约要求的 handle 形态（命令 Sender + watch 快照）未落地。
2. serve occupancy 槽状态机（linger / eviction / turn_inflight 守卫）与 runtime 内 coordinator 存在双份生命周期管理。

本 change 不承载新用户语义（needs_specs_change: false）；价值是单写者物理保证、生命周期收敛与后续并行拓扑的基座。

## What Changes

- 路线 B 先行：AgentCapabilities 可变语句内部锁化 → AgentRuntime 全面 `&self` 化 → 驱动 handle 化（watch 快照 + 控制命令邮箱）。
- 路线 A 后评估：actor task 驱动 turn（eager 流）仅在路线 B 落地后仍存在真实消费者时立项。
- occupancy 上收：槽生命周期收敛到 handle 引用计数；linger / eviction 语义重述为 handle 观察。

## 实测证据基线（2026-11-21，main @ f7922a2）

- `submit_turn(&mut self)` 是唯一的热路径可变门；`start_root_stream` 与 `abort` 已是 `&self`（react/mod.rs:471/535/116）。
- turn 运行期不触碰 `&mut` 能力集：`freeze_root_config`（react/mod.rs:512-533）在提交时快照 tools/hooks/batch/compaction/permission，内环只读快照与 Arc。
- 能力集 28 个字段中已共享 12 个（store/sink/queues/sampler/model_manager/permission/hook_bus/loop_registry 等）；`&mut` 语句约 30 个，聚集在约 8 组字段（tools 三件、hooks、batch_mode、system_prompt 两件、session_id、cwd/workspaces、stage_name 两件、compaction 两件、loop_scope）。
- 驱动侧 `self.agent.` 调用 64 处——路线 A 的消息化样板量即此规模。
- Arc 化直改曾实测 66 处 E0596（c3415 分支试验，已回退）。

## Impact

- 现有 BDD 全绿为回归地板；occupancy 测试组（48 例）为 T6 验收面。
- 嵌入面（thoughtdag）经 path 依赖编译验证 lockstep。
