# Tasks: c1650-update-compaction-cut-split-turn

> 验收一致：切点对齐 pi + 真 split 双摘要；不越界 c1660/c1670/A01。

## 1. 合约

- [x] 1.1 修订 live `domain-compaction`：`c8` 合法切点表（含 assistant / bashExecution；永不 toolResult）；新增 split-turn 双摘要与 `tokens_before` 同源估计 requirements；`.feature` 锚点
- [x] 1.2 design 切点对照表已定稿（本 change `design.md`）

## 2. 切点

- [x] 2.1 重写 `is_valid_cut_point` / turn-start 判定对齐 design 对照表；单测：可切 assistant、永不 toolResult、keep 预算
  `[blocked-by: 1.1]`
- [x] 2.2 mid-turn 切点填 `turn_start_index` + `is_split_turn=true`；切在 turn-start 时哨兵 `-1`
  `[blocked-by: 2.1]`

## 3. Split-turn 双摘要

- [x] 3.1 新增 `TURN_PREFIX_SUMMARIZATION_PROMPT` 与 `generate_turn_prefix_summary`（或等价）
- [x] 3.2 `compact_session` / prepare：split 时 history + turn-prefix 双摘要并按锁定格式合并；history 空 → `"No prior history."`
  `[blocked-by: 2.2]` `[blocked-by: 3.1]`
- [x] 3.3 `tokens_before` 改走重建上下文同源估计（可测）
  `[blocked-by: 3.2]`

## 4. 验收

- [x] 4.1 单测/BDD：cut-assistant、never-tool-result、split-dual-summary、keep-budget、tokens-before
  `[blocked-by: 3.3]`
- [x] 4.2 `llman sdd validate c1650-update-compaction-cut-split-turn --strict`；确认无 overflow/instructions/travel-LLM
  `[blocked-by: 4.1]`
