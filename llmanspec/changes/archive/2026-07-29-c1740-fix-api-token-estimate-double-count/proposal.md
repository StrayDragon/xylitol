---
depends_on: []
branch: sdd/c1740-fix-api-token-estimate-double-count
base_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
checkpointed: true
checkpoint_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
---

## Why

大会话 `460ad16e-874f-418f-8ca0-dabc58f89320` 跑完后 footer 显示 `used 215686 tokens · 164.6%/131k`。百分比公式本身没错（215686/131072≈164.6%），但 **215686 不是单次 API usage**，而是 **Api 锚点 + 对整段消息再跑 heuristic 后相加**，被标成 `TokenProvenance::Api`，造成双计与「>100% window」观感。

## Evidence（调研草稿，2026-07-29）

- **pi 对照**：`estimateContextTokens` 仅对 `lastUsageIndex` **之后**的消息累加 `estimateTokens`；xylitol 对全量 `messages` 做 heuristic → 双计（本 change 修此漂移）。
- 配置：`.xylitol/config.yaml` 中 `Ornith-1.0-35B-MTP-APEX/I-Quality` → `context_window: 131072` → footer `131k`
- Session 最后 assistant usage：`input=100490 + output=980 = 101470`（无 `total_tokens`）；compaction×20，末 `tokensBefore=215686`
- `xylitol.log`：`token estimate backend=Api tokens=215686 usage_tokens=101470 trailing=114216`
- `token.estimate` span 同数、`provenance=Api`（trace `a128da5e717e5956e7d4d70bbbe93071`）；Langfuse REST 本次无凭据，以本地为准
- 代码 SSOT：`packages/xylitol-ai-bridge/src/accounting/mod.rs` → `estimate_context`：

```text
// bug: trailing = heuristic(ALL messages)
// fix (pi): trailing = heuristic(messages after last usage index)
```

- 164.6%：**按设计允许 >100%**（footer 仅展示）；问题在分子双计
- auto-compact：`215686 > 131072 - reserve(16384)=114688` 恒触发；与连打同源

## What Changes

- Api 锚点有效时：`trailing_tokens` **只**计 last usage 消息之后的增量（对齐 pi）
- 正确填充 `last_usage_index`
- 单测 + `package-ai-bridge-accounting` 场景覆盖双计回归
- 回归：footer / compact `tokens_before` / `should_compact` 共用同一 estimate

## Out of scope

- c1730 TUI compaction 块 UI
- 改模型 `context_window` 配置值本身（131k 是配置真值）

## Open Questions

- [x] 与 pi 对齐：只计 last-usage 之后 trailing（2026-07-29 确认）
- [x] 混计小 trailing 时 provenance 仍标 Api（对齐 pi / 既有 Api 优先）
- [x] id = `c1740-fix-api-token-estimate-double-count`
- [x] playground / 目视与本 change 无关（c1730）
