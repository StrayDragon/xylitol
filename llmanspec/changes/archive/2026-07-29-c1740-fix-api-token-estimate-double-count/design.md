# Design: c1740 Api trailing scope

## Pi alignment

pi `estimateContextTokens` (`pi/.../compaction.ts`):

1. Find last message with assistant usage → `usageTokens = calculateContextTokens(usage)`.
2. `trailingTokens` = sum `estimateTokens(messages[i])` for `i > lastUsageIndex` only.
3. `tokens = usageTokens + trailingTokens`.

xylitol bug: `heuristic_tokens(messages)` over **all** messages while still labeling `Api`.

## Decision

- Match pi: trailing = messages strictly after the usage-bearing assistant.
- Keep provenance `Api` when anchor valid（与 pi / 现有 paa 一致；trailing 为增量启发式，不当作改 provenance）.
- Wire `last_usage_index` to real index (not hardcoded `Some(0)`).
- `estimate_from_session_entries` / footer / compact `tokens_before` share this path — no separate formula.

## Non-goals

- Changing Ornith `context_window` config.
- TUI compaction block (c1730).
- Abort/stale anchor rules already in paa2 (unchanged).
