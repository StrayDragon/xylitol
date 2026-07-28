# Tasks: c1640-add-compaction-turn-auto-trigger

> 验收一致：每条 task 完成 = 对应 seam 绿 + 不越界进 c1650/c1660/c1670。
> 垂直切片：合约 → force 拆开 → turn 后 auto + stale → 面/Driver 接线 → 验收。

## 1. 合约

- [x] 1.1 修订 live `domain-compaction`：新增 force/auto 分流与 stale 守卫 requirements；`.feature` 锚点 auto-over / auto-under / auto-disabled / manual-force / stale-guard（`@req`）
- [x] 1.2 修订 `agent-runtime`：assistant 回合落定后 MUST 调 threshold 检查（abort 跳过）；扩展 `valid_scope` 若需覆盖 session/runtime 挂点
- [x] 1.3 修订 `app-tui-commands` / driver 相关陈述：`/session-compact` 与 `XyDriver::compact` = force；对齐 `agent-session-store` `r41` 措辞为 reserve + turn 挂点（非百分比）
- [x] 1.4 更新 architecture「理想 vs 现状」中 turn 后 auto 为落地（可与归档同批）

## 2. Force 路径

- [x] 2.1 `CompactionOrchestrator::compact`（或 prepare 前置）对齐 pi：末条 compaction → `Already compacted`；无可压 → `Nothing to compact (session too small)`；Start reason=`manual`
- [x] 2.2 `XyDriver::compact` / dispatch `Command::Compact` / REST 改走 force，**MUST NOT** 调用 `maybe_auto_compact`
  `[blocked-by: 2.1]`

## 3. Auto 挂点与防抖

- [x] 3.1 实现 stale 守卫（assistant/usage 时间戳 ≤ 最新 CompactionEntry → skip）；无可信估计 → skip
- [x] 3.2 ReAct/session：非 abort 的 assistant 落定后调用 threshold auto（c1630 `should_compact` + `maybe_auto_compact`）；Start reason 含 `threshold`
  `[blocked-by: 3.1]`
- [x] 3.3 `enabled=false` / 未超闸 / abort → MUST NOT auto；单测覆盖
  `[blocked-by: 3.2]`

## 4. 验收

- [x] 4.1 跑通新增/修订 BDD 与相关单测；Driver 至少一处 force 集成断言
  `[blocked-by: 2.2]` `[blocked-by: 3.3]`
- [x] 4.2 `llman sdd validate c1640-add-compaction-turn-auto-trigger --strict`；确认无 overflow/split-turn/instructions 实现
  `[blocked-by: 4.1]`
