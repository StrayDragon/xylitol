---
change_id: c1595-fix-tui-abort-keep-partial
title: abort 保留已上行 partial（UI+落盘）且 LLM 投影过滤 aborted（对齐 pi）
status: ready
priority: 1595
depends_on: []
author: agent
branch: feat/c1595-abort-keep-partial
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1595-fix-tui-abort-keep-partial

> **决策已锁定（方案 A）**。正交 c1570（Ctrl+C≡Esc latch）；修订 c670/c720「清缓冲」语义。

## Why

Esc/Ctrl+C 中断流式回复时，xylitol 清空 `streaming_*` 只留 System `Aborted`，用户已看到的正文消失；pi 保留 partial + `Operation aborted` 脚注，落盘但下一轮 LLM **不**重放 aborted 助手行。

## Decisions（已锁）

### D1. UI（对齐 pi）

agent abort（非 bang）时 MUST：
1. 将已累积的 streaming thinking/text（及已闭合 tool 意图若有）**flush** 为正式 scrollback 条目；
2. 追加 abort 脚注（文案：`Operation aborted` 或现有 `Aborted`——实现时与 pi 对齐优先 `Operation aborted`）；
3. **MUST NOT** 仅用 System 行替换整段上行内容。

迟到 Xy（c670）仍 MUST NOT 在 flush 后再追加假复活正文。

### D2. 落盘

ReAct 在用户 abort 中断 generate 时 MUST 将 partial assistant 以 `stop_reason: aborted` 持久化（`MessageEnd` 或等价 persist 路径）；空 content 的 aborted MAY 跳过落盘（对齐 pi empty filter）。

### D3. 下一轮 LLM context

`project_for_llm`（或紧邻投影）MUST **跳过** `stop_reason ∈ {aborted, error}` 的 assistant 行（对齐 pi `transformMessages`）。Session 树 / resume / export 仍可见 aborted 行。

### D4. Bang abort

`!`/`!!` bang Esc 保持现有 `(cancelled)` 语义；本 change **不**改 bang。

## Open Questions

（已清空。）

## Related

- `app-tui-input`（ati2/ati14/ati31）、`agent-runtime`、`docs/architecture/插话续跑与中止.md`、`PI_DELTAS`
- 调研：[pi abort UX](ca47ea81-9320-466b-b990-e9084c1e37c0)

## Ethics

- `ethics.risk_level`: medium（改变 abort 可观测与 history）
- `ethics.required_evidence`: harness mid-stream abort 保留 partial；project_for_llm 单测跳过 aborted；persist 有 stop_reason
- `ethics.escalation_policy`: 无
