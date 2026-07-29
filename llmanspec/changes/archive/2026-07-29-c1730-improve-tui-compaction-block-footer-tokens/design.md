# Design: c1730 TUI compaction block + footer tokens

## Pi alignment

- Busy：`CompactionStatusIndicator` → 单行 `Compacting`
- Transcript：`CompactionSummaryMessageComponent` → `[compaction]` + 默认折叠摘要
- Footer：**不**堆压缩状态；只刷 token 数字

## Product shapes（静图已过目视 2026-07-29）

见 `src/app/tui/design/compaction-status.md` + playground 槽 `compaction`。

## Wiring sketch

```text
CompactionStart
  → busy Compacting
  → insert UiEntry placeholder (Compaction { pending })
CompactionEnd(ok)
  → replace placeholder → Compaction { summary, tokens_before, collapsed }
  → refresh footer tokens
CompactionEnd(fail/abort)
  → replace → aborted/error line
Rebuild/resume
  → CompactionEntry → collapsed block (not System full dump)
```

Footer 额外时机：mid-turn Api usage（节流）、TurnEnd、CompactionEnd（`footer.md`）。

## Spec touch

- `app-tui-bridge` atb5：从「System 行」改为 transcript compaction 块
- `app-tui-chrome` atc14：补 CompactionEnd / TurnEnd / mid-turn Api 节流

## Non-goals

- estimate_context 公式（c1740 已修）
- OTEL turn 终态（c1720 已归档）
- auto-compact 触发策略
