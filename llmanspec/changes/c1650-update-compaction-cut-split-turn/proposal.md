---
change_id: c1650-update-compaction-cut-split-turn
title: 切点与 split-turn 双摘要对齐 pi
status: purpose-draft
priority: 1650
depends_on:
  - c1630-update-compaction-reserve-formula
author: agent
---

# c1650-update-compaction-cut-split-turn

> **流程**：仅 `purpose-draft`；禁止提前改 live specs。可与 c1640 **并行** promote（共依赖 c1630）。
> **对照**：`../pi/.../compaction/compaction.ts` — `findCutPoint` / `prepareCompaction` / `compact` split-turn 双摘要；`TURN_PREFIX_SUMMARIZATION_PROMPT`。

## Why

xylitol 切点几乎只认 user → `is_split_turn` 半死；超长单轮会丢 turn 前缀。pi：assistant 等可切 + history/turn-prefix **双摘要合并**。

## 需求锁定

### R1 — 合法切点（已决，对齐 pi）

合法（context-visible）：**user、assistant、bashExecution、custom、branchSummary**（及产品已有等价 entry）。
**永不**在 tool result / tool 结果条目切断。
assistant 带 tool_calls 时：切在 assistant，其后 tool results 留在 kept 侧（pi 语义）。

### R2 — `find_cut_point`（已决）

- 自新向旧累加至 `keep_recent_tokens`，落到最近合法切点。
- mid-turn 切在非 turn-start 时：MUST 填 `turn_start_index` 且 `is_split_turn=true`。
- 切在 turn-start 时：`is_split_turn=false`，`turn_start_index` 无效哨兵（现有 `-1` 可保留）。

### R3 — Split-turn 双摘要（已决）

当 `is_split_turn`：

1. `messagesToSummarize` = `[boundary_start, turn_start)`
2. `turnPrefixMessages` = `[turn_start, first_kept)`
3. 若两者皆空 → 不 compact（prepare 失败）
4. 否则：history 摘要（可带 previousSummary）+ turn-prefix 摘要（专用 prompt，对齐 pi `TURN_PREFIX_SUMMARIZATION_PROMPT`）
5. 合并格式（锁定，防漂）：

```text
{historyText}\n\n---\n\n**Turn Context (split turn):**\n\n{turnPrefixText}
```

history 为空时 historyText 用 `"No prior history."`（对齐 pi）。

**禁止**：只放开 assistant 切点却不写 turn-prefix 摘要。

### R4 — `tokens_before`（已决倾向）

MUST 按 **重建后将送入模型的上下文** 估计（对齐 pi `estimateContextTokens(buildSessionContext(...).messages)`），优先走既有同源估计入口；MUST NOT 仅用 boundary 内 entry `len/4` 总和作为唯一权威（可作降级但须可测）。

### R5 — 迭代边界

存在先前 CompactionEntry 时：摘要 span 自上一 `firstKeptEntryId`（找不到则 compaction 后一条）起——与现有/pi 一致；本 change 不改该边界语义，只修切点与 split 摘要。

### R6 — 非目标

| 禁止 | 归属 |
|---|---|
| turn 后 auto / force 接线 | c1640 |
| overflow retry | c1660 |
| compact instructions | c1670 |
| travel 时 LLM 分支摘要 | **A01 保留** |
| 百分比触发 | 禁止 |

## 验收锚点

| id | Then |
|---|---|
| cut-assistant | 超长单轮可切在 assistant；`is_split_turn=true` |
| never-tool-result | 切点索引永不落在 tool result |
| split-dual-summary | fake 模型可观察两次摘要或合并后含 `Turn Context (split turn)` |
| keep-budget | `keep_recent_tokens` 下保留近期约量（与现有 find-cut 精神一致） |
| tokens-before | `tokens_before` 与重建上下文估计一致（或单测钉入口） |

## Capabilities

`domain-compaction`（修订 c8；新增 split-turn / 双摘要 MUST）

## Open Questions

- （已决）assistant+tools 切点跟 pi。
- （已决）BashExecution 等角色：代码路径有则规则覆盖，避免日后漂移。
- AgentMessage 角色映射表：promote 时在 design 附一张 xylitol entry → 是否合法切点对照即可。

## Ethics

- risk_level: medium
- prohibited_actions: 半套 split-turn；提前改 live specs；推翻 A01
- required_evidence: 上表单测/BDD
- refusal_contract: 不做 travel LLM 分支摘要
- escalation_policy: 角色映射歧义先 design 对照表
