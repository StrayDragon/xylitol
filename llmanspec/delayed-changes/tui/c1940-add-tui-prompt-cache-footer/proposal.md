---
depends_on:
  - c1885-add-responses-cache-usage-honesty
apply_band: P9-deferred
summary: "TUI footer 显示 Prompt Cache 命中/未回报/不适用三态读数；刻意延后"
---

# TUI footer：Prompt Cache 三态读数（延后）

> **⚠️ deferred（2026-08-05）**：移入 `llmanspec/delayed-changes/`，避免污染本期 SDD graph。前置已落地/归档：`c1885`（cache usage）、`c1890`/`c1930`/`c1925`/`c1900`；`tool_search` → `c1960`。Todo/状态栏等扩展后置。


> **占位草案**：避免遗忘。实现前置依赖 [`c1885`](../../../changes/archive/2026-08-05-c1885-add-responses-cache-usage-honesty/proposal.md)（Responses usage 三态 + 观测）。**本草案不做** Specs landing / apply，直至 c1885 归档且产品确认要固定区展示。

## Why

c1885 把 Prompt Cache 尺子落在 bridge + trace/Langfuse，**刻意不做 TUI**。若日后排障或日常会话需要在固定区一眼看见「命中 / 未回报 / 不适用」，需要独立 change：对齐 [`TUI信息呈现与固定区词汇`](../../../../docs/architecture/TUI信息呈现与固定区词汇.md)，避免临时塞文案。

## What Changes（意向，未钉）

- TUI footer（或等价固定区）展示 `PromptCacheRead` 三态：`Tokens(n)` / `NotReported` / `NotApplicable`。
- 文案粒度（精确数字 vs 「有 cache」）与 固定区词汇表对齐后再定。
- MUST NOT 伪造命中；MUST NOT 把 hit rate 当 KPI。

## Capabilities（意向）

- `app-tui-fixed-zone`（或等价）
- 跨面：跟 [`跨端同源`](../../../../docs/roadmaps/跨端同源.md) 约束

## Impact

- 用户在 TUI 可见缓存诚实读数；排障不必只开 Langfuse。

## Out of scope

- usage 映射 / WirePolicy（→ c1885）
- Assembler / 状态栏子系统 / tool_search

## Parallel / depends

- `depends_on: [c1885-add-responses-cache-usage-honesty]`（语义；c1885 归档后 validate 会降为 INFO）

## Open Questions

- footer 精确数字 vs 短标记？
- 仅 streaming Done 后更新，还是回合汇总？

## Ethics

- risk_level: low
- prohibited_actions: 伪造 cache hit；把启发式标成 Api cache
- required_evidence: 固定区词汇对齐 + 三态 fixture 驱动的 TUI 测（若落地）
- refusal_contract: 不把 hit rate 写成产品 KPI
- escalation_policy: 产品确认要固定区展示 后再 propose
