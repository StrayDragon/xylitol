---
depends_on: []
branch: sdd/c1860-refactor-context-token-settlement
base_sha: 36271222a6221e1edf59c09adb79d41711451acc
checkpointed: false
---

# 合流上下文 token settlement（一轮一次估计）

## Why

Langfuse / 本地 `provider-trace.jsonl` 显示：单次用户 turn 收尾常在 ~10ms 内打出 **3×** `token.estimate`（同数 Api tokens），其中 2 条挂 `agent.turn`、1 条为 **独立根**（新 trace）。根因不是计量公式双计，而是 **同源入口被多处独立调用，且每次调用都 emit**：

1. Agent `try_turn_end_compaction` → `maybe_auto_compact` 预检 estimate
2. TUI `TurnEnd` → `footer_token_refresh` → 再 estimate
3. TUI `on_run_stream_closed` → 再 estimate（此时 turn parent 已清 → `SpanContext::random` 独立根）

产品合约早已要求「footer 与 compact 触发同源」（`domain-compaction` c2/c16、`docs/architecture/压缩与上下文.md`），但装配仍是「同函数、多次请求、观测绑死在入口」。需要把 **settlement（一次结果）** 从 **调用方** 里抽出来，并给后续失效提示等扩展留口（本 change **不**加 CacheHint 消费桩，仅注释约定）。

证据 trace：`8fb672438123f15efd4c83995b13e6a8`（`.903`/`.909` 同 turn + `.912` 独立根 `6622c97a…`）；本地簇模式稳定复现。

## What Changes

- 引入 **Context Token Settlement** seam（命名以实现为准）：一次 `ContextTokenEstimate` + `reason` + generation；算数 SSOT 仍是 `estimate_from_session_entries`。
- Turn 收尾路径：**只 estimate 一次**；compact 决策与 footer **消费同一 snapshot**；`token.estimate` OTel **至多 1 次 / 该次 settlement**（挂活跃 turn；禁止本路径再开独立根）。
- TUI：`TurnEnd` + `stream close` **不得**再叠第二次 estimate；stream close 在本轮已有 settlement 时 **跳过**重算（fallback 仅当本轮无 settlement）。
- `CompactionEnd` / 换叶 / mid-turn usage 等 **真实失效** 仍可触发新 settlement（合法第二次，不是同秒重复）。
- 观测与纯算解耦：`estimate_*` 默认可静默；settlement 路径显式 emit。
- 扩展性：`InvalidationReason`（或等价）+ 维护者注释说明如何加 cache-policy / hint 失效；**不**落地 CacheHint UI/桩。

## Capabilities

- `domain-compaction` — settlement 与 compact 触发共用 snapshot（强化 c16）
- `app-tui-chrome` — 收紧 atc14 刷新语义
- `infra-otel` — 收紧 otel13（settlement 路径禁止无意义独立根）
- `protocol-*` / `agent-runtime` — 若经 `XyEvent` 或 driver 缓存暴露 snapshot（实现选一，design 定案）

## Impact

- **TUI footer**：行为仍更新；少浪费 encode；消除双/三刷。
- **Langfuse**：同 turn 收尾不再出现空壳独立 `token.estimate` trace（idle 换叶等 MAY 仍独立根）。
- **Harness**：`TurnEnd` / `stream_closed` footer 测需改「消费 settlement / 不双 kick」。
- **非目标**：改 Api/Heuristic 公式；CacheHint UI；拆第二套 accounting。

## Ethics

- risk_level: low
- prohibited_actions: 为 footer/compact 引入第二套 token 尺子；默认把敏感上下文写入 observation I/O
- required_evidence: CollectingReporter 或等价证明收尾 `token.estimate`×1；harness 证明 TurnEnd+stream close 不双 estimate
- escalation_policy: 若必须扩 `XyEvent` 闭集，实现前在 design 锁定变体与 wire 兼容策略
