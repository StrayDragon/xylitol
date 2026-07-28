# Tasks: c1630-update-compaction-reserve-formula

> 验收一致：每条 task 的「完成」= 对应 seam 绿 + 不越界进 c1640+。
> 垂直切片：合约 → API/装配 → BDD 接线 → 文档/残留闸。
> Propose 已完成 §1；§2–§4 留待 `llman-sdd-apply`。

## 1. 合约与文档心智

- [x] 1.1 修订 live `domain-compaction`：`c2` 改为 reserve 公式（含 `enabled=false` 永不触发）；`c14` 运行时类型名 `CompactionSettings`；`c16` 措辞改为「reserve 触发决策」仍同源估计、禁止独立 len/4
- [x] 1.2 修订 `domain-compaction.feature`：`need-compact` / `no-compact` 用 reserveTokens+window 数字；新增 `disabled-no-compact`；`threshold-shares-footer-estimate` 改写为 `reserve-trigger-shares-footer-estimate`；场景挂 `@req:c2`（及 c16）；bindings 同步
- [x] 1.3 修订 `runtime-config`：`rc15` 澄清映射字段为 enabled/reserveTokens/keepRecentTokens，MUST NOT 暴露 threshold；`runtime-config.feature` `@req:rc15` 断言 `keep_recent_tokens` 而非 `.threshold`
- [x] 1.4 更新 `docs/architecture/压缩与上下文.md`：去掉「默认约 80%」；写明 reserve 心智；百分比仅派生展示

## 2. 核心 API 与装配去阈

- [ ] 2.1 实现 `should_compact(tokens, window, &CompactionSettings)`（合并/删除百分比版与仅测试的 `should_compact_by_reserve`）；单测覆盖边界与 `enabled=false` / `window=0`
- [ ] 2.2 `CompactionOrchestrator` 去掉 `threshold: f64`，`maybe_auto_compact` 走新公式；`get_context_usage` 改吃 `&CompactionSettings`，`percent` 仍派生
- [ ] 2.3 删除 `compaction_threshold` 全装配链（builder / composition / bootstrap / `AgentCapabilities` / 测试 helpers）；调用点一律持 `CompactionSettings`
  `[blocked-by: 2.1]` `[blocked-by: 2.2]`

## 3. BDD / 夹具对齐验收

- [ ] 3.1 将 propose 阶段已改写的 BDD steps（reserve Given / `shouldCompact` 步内公式）**改接到**生产 `should_compact(..., &CompactionSettings)`；清理 fixtures 上遗留 `compaction_threshold`
  `[blocked-by: 1.2]` `[blocked-by: 2.1]`
- [ ] 3.2 跑通 `domain-compaction` 与 `runtime-config` 相关 BDD；`need-compact`/`no-compact`/`disabled-no-compact`/rc15 与单测一致
  `[blocked-by: 3.1]` `[blocked-by: 1.3]`

## 4. 收尾闸

- [ ] 4.1 全仓确认无生产 `compaction_threshold` 残留；示例 config 可显式三字段；`llman sdd validate c1630-update-compaction-reserve-formula --strict`
  `[blocked-by: 2.3]` `[blocked-by: 3.2]`
- [ ] 4.2 确认未实现 c1640+（无 turn 后 auto 新接线、无 split-turn/overflow/instructions/TUI % 条）
