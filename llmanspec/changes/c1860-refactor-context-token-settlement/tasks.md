# Tasks: c1860-refactor-context-token-settlement

## 1. Settlement 核心与观测解耦

- [ ] 1.1 在 compaction/token 路径增加 quiet estimate 与 `settle`（或等价）：算出 `ContextTokenEstimate`，按需 emit `token.estimate`；默认 `estimate_*` 测试/内部路径不强制打点
- [ ] 1.2 定义 `ContextTokenSettlementReason`（TurnSettled / AfterCompaction / MidTurnUsage / LeafChanged / …）；模块注释写明未来 CachePolicy/Hint 只加 reason + 订阅 snapshot，**不**加 UI 桩
- [ ] 1.3 CollectingReporter 单测：一次 TurnSettled → 恰好一个 `token.estimate`；parent 为 turn；无额外 random 根

## 2. Agent 收尾合流

- [ ] 2.1 `try_turn_end_compaction` / `maybe_auto_compact`：预检使用 **一次** settlement snapshot；决策与（若 emit）观测同源
- [ ] 2.2 调整与 `TurnEnd` 的可见顺序（先 settle 再让面依赖，或经 settlement 事件携带 estimate），消除 TUI 抢跑二次 estimate
- [ ] 2.3 Compact **实际执行**后：经 CompactionEnd 路径触发 AfterCompaction settlement（新 generation）

## 3. Protocol / Driver seam

- [ ] 3.1 按 design 落地 `XyEvent::ContextTokenSettlement`（或 TurnEnd 扩展）；更新 wire 映射若需要
- [ ] 3.2 `XyDriver` 可选缓存 `last_context_token_settlement`；`estimate_context_tokens` 保留给换叶/显式刷新，文档标明与 settlement 关系

## 4. TUI 刷新策略

- [ ] 4.1 Host：消费 settlement 事件更新 footer（异步标签规则不变）
- [ ] 4.2 `TurnEnd` 不再单独 `estimate`（若事件已覆盖）；`on_run_stream_closed` 在本 run 已有 TurnSettled generation 时 **跳过** kick
- [ ] 4.3 保留：换叶 / CompactionEnd / mid-turn throttled / resume 等真实失效路径
- [ ] 4.4 更新 `design/footer.md` 刷新时机措辞与 atc14 对齐
- [ ] 4.5 Harness：改 `c1730_turn_end` / `c1035_stream_closed`；新增「TurnEnd+stream close 不双 estimate」断言

## 5. Specs landing

- [ ] 5.1 `domain-compaction`：强化 c16 或新 req — turn 收尾 compact 与 footer 共用同一 settlement snapshot
- [ ] 5.2 `app-tui-chrome`：改 atc14 — 禁止 TurnEnd 与 stream close 对同一次 idle 收尾双 estimate
- [ ] 5.3 `infra-otel`：改 otel13 — settlement 收尾路径 MUST NOT 仅为 footer 再开独立根；真无 turn 的闲置路径仍 MAY
- [ ] 5.4 对应 `.feature` 场景措辞（Partitioned；可执行仍以 harness/单测为主处标清）

## 6. 校验

- [ ] 6.1 `llman sdd validate c1860-refactor-context-token-settlement --strict`
- [ ] 6.2 相关 `just test` / harness / CollectingReporter；`just fmt` + 相关 clippy
