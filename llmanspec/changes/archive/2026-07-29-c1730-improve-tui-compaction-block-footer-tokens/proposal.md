---
depends_on: []
branch: sdd/c1730-improve-tui-compaction-block-footer-tokens
base_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
checkpointed: true
checkpoint_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
---

## Why

大会话上 auto-compact / 手动 compact 在 TUI 里几乎不可见：busy 只有短词 `Compacting`，完成后退回 muted System 全文 summary，footer token 又常在 turn 中途不刷新。需要对齐 pi 的 transcript compaction 块，并补齐 footer 数字刷新时机。

## What Changes

- **Transcript compaction 块**（对齐 pi `CompactionSummaryMessageComponent`）：
  - `CompactionStart`：插入占位块 `[compaction]` + `Compacting…`
  - `CompactionEnd` 成功：就地变成完成块；**默认折叠** `Compacted from N tokens (… to expand)`；展开后显示 summary
  - 失败/aborted：短失败态；resume/rebuild 用完成块，不再整段 System dump
- **Busy**：保留单行 `Compacting`；**MUST NOT** 把压缩细节堆进 footer
- **Footer tokens**：turn 中 Api usage 节流刷新 + `CompactionEnd` / `TurnEnd`；仍不阻塞输入（见 `design/footer.md`）

## Design SSOT

- `src/app/tui/design/compaction-status.md`（目视通过）
- `src/app/tui/design/footer.md`
- playground 槽 `compaction` + fixture `compaction.collapsed.yaml`

## Capabilities

- `app-tui-bridge`（atb5）
- `app-tui-chrome`（atc14）

## Impact

- TUI host / bridge / session_tree 重建路径
- 不改 compact 触发公式

## Out of scope

- auto-compact 连打策略
- token estimate 双计（c1740 已归档）
- c1720 otel（已归档）

## Open Questions

- [x] id / busy / 占位→完成 / 默认折叠 / playground 目视通过
- [x] depends_on c1720 已满足并清空（归档后）
