# Design: c1650-update-compaction-cut-split-turn

## 对照 pi（一手）

| pi (`compaction.ts`) | xylitol 现状 | 本 change |
|---|---|---|
| `isCutPointMessage`：user / assistant / bashExecution / custom / branchSummary / compactionSummary；**永不** toolResult | `is_valid_cut_point` **仅 user**（+ branch/custom）；assistant 显式排除 | ✅ 对齐合法切点 |
| `isTurnStartMessage`：user / bash / custom / branch / compactionSummary；assistant **不是** turn-start | 仅 user（+ branch/custom） | ✅ |
| mid-turn 切 → `isSplitTurn` + `turnStartIndex` | 结构有，但因切点几乎总是 user，`is_split_turn` 半死 | ✅ 放开后可真触发 |
| split：history + turn-prefix **双摘要**，合并 `---\n**Turn Context (split turn):**` | split 时 `history_end=turn_start`，**丢弃** `[turn_start, first_kept)` 不摘要 | ✅ 双摘要 |
| `TURN_PREFIX_SUMMARIZATION_PROMPT` | 无 | ✅ 新增常量 |
| `tokensBefore` = `estimateContextTokens(buildSessionContext)` | boundary 内 `estimate_tokens_entry` 累加（len/4） | ✅ 改同源估计 |

## 合法切点对照表（xylitol）

| 条目 / 角色 | 合法切点 | turn-start |
|---|---|---|
| `Message` role=`user` | ✅ | ✅ |
| `Message` role=`assistant` | ✅ | ❌ |
| `Message` role=`toolResult` | ❌ | ❌ |
| `BashExecution` 或 message role=`bashExecution` | ✅ | ✅ |
| `CustomMessage` / `Custom`(custom_message) | ✅ | ✅ |
| `BranchSummary` | ✅ | ✅ |
| `Compaction` | 不作 cut 候选（边界） | — |

assistant 带 tool_calls：切在 assistant → 其后 toolResult **留在 kept**（永不切 toolResult）。

## Split-turn 摘要流

```text
is_split_turn:
  historyMsgs   = entries[boundary_start .. turn_start)
  turnPrefixMsgs = entries[turn_start .. first_kept)
  historyText   = summarize(history) 或 "No prior history."
  turnPrefixText = summarize_turn_prefix(turnPrefix)  // 专用 prompt
  summary = "{historyText}\n\n---\n\n**Turn Context (split turn):**\n\n{turnPrefixText}"
else:
  既有单次 history 摘要
```

**禁止**只放开 assistant 切点却不写 turn-prefix 摘要。

## `tokens_before`

优先：`estimate_from_session_entries`（或同源 helper）对「压缩后将送入模型的上下文」估计；禁止仅以 boundary `len/4` 总和为唯一权威（可降级但须可测）。

## 非目标

c1640 auto/force · c1660 overflow · c1670 instructions · A01 travel LLM 分支摘要

## 验收 seam

| 锚点 | 覆盖 |
|---|---|
| cut-assistant / never-tool-result / keep-budget | `find_cut_point` 单测 |
| split-dual-summary | fake 模型 / 单测断言合并标记 |
| tokens-before | 单测钉估计入口 |
