---
change_id: c1640-add-compaction-turn-auto-trigger
title: Turn 后自动 compaction 与手动 force 路径（对齐 pi）
status: purpose-draft
priority: 1640
depends_on:
  - c1630-update-compaction-reserve-formula
author: agent
---

# c1640-add-compaction-turn-auto-trigger

## Why

公式对齐后仍不够：当前 `maybe_auto_compact` **只**经 `XyDriver::compact()`（slash / REST）调用，且该路径还要先过阈值闸。pi 在 agent 回合结算后 `_checkCompaction`；手动 `/compact` 是强制摘要。xylitol 文档已标「阈值自动压缩 ✅」，与代码事实漂移——本 change 补上触发器与 manual/force 分离。

## What Changes

- **Auto**：在 turn 结算后（对齐 pi：assistant 落定 / agent_end 前可判定处）调用 reserve 公式；超阈则跑 compaction，发 `CompactionStart(reason=threshold|…)` / `CompactionEnd`。
- **Manual force**：`/session-compact` 与 REST compact 走 `CompactionOrchestrator::compact`（或等价 force API），**不过**阈值闸；无可摘要内容时明确错误（如 already compacted / session too small）。
- `XyDriver::compact` 语义改为 force（或拆 `compact` / `maybe_auto_compact` 两 API，面只调 force）；禁止再把 manual 接到 `maybe_*`。
- 防抖：紧接 compact 后勿用压缩前 stale usage 立刻再触发（对齐 pi 对 compaction 边界后 usage 的处理）。
- **本 change 不做**：overflow compact-and-retry（c1660）、split-turn（c1650）、optional instructions（c1670）、abort 细控可最小可用。

## Capabilities

| Capability | 变更 |
|---|---|
| `domain-compaction` | agent 集成：auto 触发 + force 手动 |
| `agent-runtime` | turn 结算与 compaction 钩子顺序 |
| `app-tui-commands` / driver | slash / Driver compact = force |
| `cli-entry` / server | REST compact 与 TUI 同语义 |

## Impact

- **破坏性**：手动 compact 在未超阈时也会尝试压缩（与今日 maybe 行为不同）。
- **默认体验**：长会话可在无需 slash 时自动压；生命周期事件可感知。
- **非目标**：overflow 重试、自定义 instructions、extension 替换摘要。

## Depends / 后续

```text
c1630 ──► c1640 (本) ──┬──► c1660 overflow retry
                       └──► c1670 optional instructions
```

## Open Questions

- （倾向）挂点优先对齐 pi：session/agent 层在 turn 结束后检查，而非 TUI host 轮询。
- abort：本批可只保证 lifecycle `aborted` 字段诚实；完整 AbortController 可随 c1660。

## Ethics

- risk_level: medium
- prohibited_actions: 仅改文档标 ✅ 而不接线；manual 继续走 maybe 闸
- required_evidence: BDD/单测覆盖「超阈 auto」「未超阈 manual 仍 force」「无可压时报错」；ReAct/Driver 集成测至少一处
- refusal_contract: 不在本 change 实现 extension 可替换 compaction
- escalation_policy: 若挂点与 steer/follow-up 队列竞态不清，先 design 钉顺序再 apply
