# Tasks: c2820-refactor-iteration-settle

- [ ] 1. Specs：`agent-runtime` / `domain-compaction` 写清 run vs iteration vs settle；`max_turns` 只在 Settle 后计数；otel skipped 次数随预检，不改 span 形状 `[blocked-by: none]`
- [ ] 2. `turn_end`：穷举 ContinueTools vs Settle；ContinueTools 只配对 TurnEnd；Settle 才 settlement + 预检 + should_stop `[blocked-by: 1]`
- [ ] 3. `react`：工具窗口结束后走 ContinueTools 再进下一轮 generate；无 tool_calls 走 Settle；overflow 错误路径仍 Settle `[blocked-by: 2]`
- [ ] 4. 测：tool 续跑 → 一次 skipped / 一次 settlement；`max_turns=1` 不在首批 tool 后停；无工具单轮回归 `[blocked-by: 3]`
