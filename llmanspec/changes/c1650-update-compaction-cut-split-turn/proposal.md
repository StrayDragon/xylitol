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

## Why

pi 允许在 user **或** assistant（及 bash/custom 等）处切断；超长单轮会 `isSplitTurn`，并对 history + turn prefix **各生成摘要再合并**。xylitol 刻意只认 user 为切点，导致 `is_split_turn` 字段半死、超长单轮易丢前缀上下文。要「完全跟进」实现，必须把切点规则与双摘要路径对齐。

## What Changes

- `is_valid_cut_point`：对齐 pi——user / assistant / bashExecution / custom / branchSummary 等合法；**永不**在 tool result 切断。
- `find_cut_point`：mid-turn 时正确填 `turn_start_index` / `is_split_turn`。
- `prepare`/`compact`：split-turn 时收集 `turn_prefix_messages`，生成 turn-prefix 摘要并与 history 摘要合并（prompt 对齐 pi `TURN_PREFIX_SUMMARIZATION_PROMPT`）。
- `tokens_before`：尽量按重建会话上下文估计（对齐 pi `estimateContextTokens(buildSessionContext(...))`），而非仅 boundary 内 entry heuristic 求和（可与现有 provenance 入口共存）。
- 单测对齐 pi 关键用例：split-turn 指示、keep 预算、先前 compaction 边界再摘要。
- **本 change 不做**：overflow retry、auto 接线（可依赖已归档的 c1640，但不阻塞本算法落地）、instructions、A01 分支 LLM。

## Capabilities

| Capability | 变更 |
|---|---|
| `domain-compaction` | c8 切点规则；新增/修订 split-turn 与双摘要 MUST |

## Impact

- **破坏性**：同会话在超长单轮下切点索引可能变化；依赖「只切 user」的测试需改。
- **默认体验**：超长工具回合不再静默丢掉 turn 前缀。
- **非目标**：branch travel LLM 摘要（A01 仍保留）。

## Depends / 后续

```text
c1630 ──► c1650 (本) ──► c1660（overflow 依赖可靠切点）
```

可与 c1640 **并行**开发（共同依赖 c1630）。

## Open Questions

- （倾向）assistant 带 tool_calls 时切点语义跟 pi：tool results 跟在 kept 侧。
- BashExecution entry 若产品面未全量使用，仍实现切点规则以免日后漂移。

## Ethics

- risk_level: medium
- prohibited_actions: 只放开 assistant 切点却不实现 turn-prefix 摘要（会丢上下文）
- required_evidence: 单测覆盖 split-turn 双摘要合并；tool result 不可切；BDD 或等价场景
- refusal_contract: 不推翻 A01（travel 自动 LLM 分支摘要）
- escalation_policy: 若消息模型与 pi AgentMessage 角色差导致切点映射歧义，先 design 对照表
