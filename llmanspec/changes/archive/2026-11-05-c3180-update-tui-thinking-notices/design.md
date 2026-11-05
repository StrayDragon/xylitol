# Design — 修复摘要通知、Thinking 预览与时长

## 通知

`click_footer_action(FoldSummary)` 无论正文是否为空，都 push `显示等级: <摘要|最近|完整|安静>`。

## Thinking 预览

折叠 Thought 头：`Thought <dur> · <latest>`；latest 取该 cluster 最后一个 Thinking 条目的末行；live streaming 用 `streaming_thinking` 末行。

## 时长

- Host 在 `AgentStart` 记录 `OffsetDateTime`，`AgentEnd` 给最后一个 segment 写 `SegmentClock`。
- `format_duration` 允许 0s；`secs_between` / `elapsed_from_persist_ms` 不再过滤 0。
- `thought_duration_label` 以“存在 Thinking 条目”为条件，0s 也显示。

## 配色

- `❯` = `paint_user`。
- `[▤ 摘要]` / 未激活 `◨` = `paint_muted`。
- 边框/角 = `operation_zone_paint`。
