---
depends_on: []
branch: sdd/c1750-fix-compaction-aware-session-context
base_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
checkpointed: true
checkpoint_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
---

## Why

压完后 footer/Api 估计 ~101k，但实际请求仍 `n_prompt_tokens=167065` 超 131k。根因：

1. `load_conversation_history` 用 `load_entries`（整文件）且 **不按 `firstKeptEntryId` 裁切**（pi `buildContextEntries` 会裁）。
2. overflow compact-and-retry 只 `pop` 错误 assistant，**不重载** store 上的裁切后上下文。
3. OpenAI Responses 错误体 `code: 400`（整数）导致 SDK 反序列化失败，盖住真实 `exceed_context_size_error` 文案。

## What Changes

- 对齐 pi：`build_context_entries`（最新 compaction + firstKept… + compaction 之后）
- `build_session_context` / `_v2` / `load_conversation_history` 走 leaf branch + 裁切
- overflow `will_retry` 后从 store 重载 history
- Responses `map_err`：从 `content:{...}` 抽出 `error.message`（及 type），避免只见 deserialize 噪音

## Evidence

Session `460ad16e-…`：leaf 若整链投影 → msg json ~109k tok + 多份 summary；裁切后应接近 keep_recent + 一份 summary。

## Capabilities

- `agent-session`（as45 收紧）/ `agent-session-store` 或 `domain-compaction` 交叉
- `package-ai-bridge`（错误文案）

## Out of scope

- 改 context_window 配置
- TUI compaction 块（c1730）
- token trailing 双计（c1740 已修）
