# Tasks: c1680-add-tui-compaction-percent-display

> 验收一致：footer 派生 %；触发仍 reserve；无百分比闸配置；不越界 `(auto)`/色阶/cache-hit。

## 1. 合约

- [x] 1.1 修订 live `app-tui-chrome`：atc2/atc13（或新 atc21）要求有 `context_window>0` 时追加派生 `p%/fmt(window)`；Heuristic `~`；window=0 不追加；MUST NOT 用 % 触发。同步 `design/footer.md`；`.feature` 锚点 derived-only / no-percent-when-no-window / heuristic-tilde
- [x] 1.2 design 文案公式 / 非 MVP 边界已定稿（本 change `design.md`）

## 2. 实现

- [x] 2.1 实现 `format_compact_tokens` + 扩展 `footer_token_label(…, context_window)`（或并列 helper）；单测钉格式与 provenance
  `[blocked-by: 1.1]`
- [x] 2.2 `refresh_footer_tokens` / `kick_footer_token_refresh` 传入当前模型 `context_window`；harness 覆盖有窗/无窗/Heuristic
  `[blocked-by: 2.1]`

## 3. 隔离验收

- [x] 3.1 确认 `should_compact` / CompactionSettings / 配置 schema 无百分比闸；既有 compaction 单测仍绿
  `[blocked-by: 2.2]`
- [x] 3.2 `llman sdd validate c1680-add-tui-compaction-percent-display --strict`；确认无 `(auto)`/色阶/cache-hit 范围蔓延
  `[blocked-by: 3.1]`
