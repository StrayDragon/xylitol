# Design: c1640-add-compaction-turn-auto-trigger

## 对照 pi（一手）

| pi (`agent-session.ts`) | xylitol 目标 | 本 change |
|---|---|---|
| 回合落定后 `_checkCompaction(msg)`（默认 `skipAbortedCheck=true`） | agent/session 或 ReAct 在 assistant 落定后调 reserve 闸 | ✅ |
| 下一 prompt 前 `_checkCompaction(last, false)`（可含 aborted） | 可选补洞；本 change **最小**：仅 post-turn + abort 跳过 | 延后可跟 |
| `_checkCompaction` Case1 overflow → `_runAutoCompaction("overflow")` | — | ❌ c1660 |
| Case2 threshold → `shouldCompact` → `_runAutoCompaction("threshold")` | c1630 公式 + `maybe_auto_compact` | ✅ |
| stale：assistant / usage 时间戳 ≤ 最新 CompactionEntry → skip | 防抖 | ✅ |
| `compact()` force：`prepareCompaction` 失败 → `Already compacted` / `Nothing to compact`；`reason: "manual"` | `XyDriver::compact` / slash / REST = force | ✅ |
| auto `prepare` 失败 → 静默 `false`（不抛） | 同构 | ✅ |
| extension `session_before_compact` | — | ❌ 不做 |

公式 SSOT 仍在 c1630：`should_compact(tokens, window, &CompactionSettings)`。

## 挂点（已决）

```text
ReAct / session：assistant 回合持久化完成（可区分 aborted）
  → check_compaction_threshold（或等价）
       → enabled? abort? stale? 有可信 tokens?
       → should_compact? → orchestrator.maybe_auto_compact (reason≈threshold)
```

- MUST NOT 仅靠 TUI host 轮询。
- `XyDriver::compact` / `Command::Compact` / `/session-compact` / REST → **`orchestrator.compact`（force）**，MUST NOT 再走 `maybe_auto_compact`。

## API 形状

| API | 语义 |
|---|---|
| `maybe_auto_compact` / `_with` | 仅内部 auto；过 reserve 闸 + stale 守卫 |
| `compact`（orchestrator / Driver） | force；不过闸；prepare 失败返回明确错误串 |

Force 错误文案对齐 pi（英文即可，与现有 `String`/`XyDriverError` 一致）：
- 末条已是 compaction → `"Already compacted"`
- 无可摘要边界 → `"Nothing to compact (session too small)"`

`CompactionStart.reason`：
- auto：含 `threshold` 可区分语义（可附占用说明）
- manual：`manual`

## Stale 守卫

对齐 pi：若用于判定的 assistant（或 estimate 所锚 usage 消息）时间戳 ≤ 最新 `CompactionEntry` 时间戳 → MUST NOT threshold-auto。
无任何可信 usage/估计 → MUST NOT 盲目 compact。

## 非目标

overflow retry · split-turn · custom instructions · extension 替换 · 恢复 `compaction_threshold` · travel 分支摘要

## 验收 seam

| 锚点 | 建议覆盖 |
|---|---|
| auto-over / under / disabled | 单测 orchestrator/session 或 BDD `domain-compaction` |
| manual-force / manual-wired | Driver/`Command::Compact` 单测或 BDD；断言不经 maybe |
| stale-guard | 单测 |
| 事件 | CompactionStart/End reason 诚实 |

## 文档

归档后更新 `docs/architecture/压缩与上下文.md`「Turn 后自动」行。
