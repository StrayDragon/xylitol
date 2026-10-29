# Design

## Context

`LoadedHistory.entries` 保存从盘上加载的所有条目，`ui_model.entries` 只保留最新 800 条。点击“已收起”时 `expand_history_window` 从缓冲重建 UI。缓冲无上限时，长会话持续翻页会让内存随加载量增长。

## Decision

采用 **A. 固定缓冲上限 + 淘汰回退**：

- `HISTORY_BUFFER = HISTORY_WINDOW + HISTORY_PAGE`（1000）。
- `LoadedHistory::new` / `merge_older` 在合并后调用 `enforce_buffer_cap`：
  - 超出时 `drain(..drop)` 淘汰最旧条目；
  - 重算 `first_entry_id`；
  - `has_more = true`（被淘汰条目仍在盘上，可按锚点回源）。
- `prepend_history` 与 `apply_switched_session` 的 `show_notice` / `trim_to_window` 使用缓冲的真实 `has_more`，而不是调用参数。
- `expand_history_window` 保持同步：先展开缓冲；若 UI 仍溢出且 `has_more=true`，提示回落为“加载更早”，用户点击后按 `first_entry_id` 回源。

这样“已收起”只在全量历史可放进缓冲（≤1000）且 UI 溢出（>800）时出现；缓冲淘汰后回到“加载更早”，语义与可达性一致。

## Alternatives

- **B. 异步展开回源**：`expand_history_window` 改 async，缓冲上限更小；改动面更大，且需要新的 effect 路径。
- **C. 保持无上限**：内存随加载量增长，不解决本问题。
- **D. 只裁 UI 不裁缓冲**：现状，等价于 C。

## Risks

- 缓冲淘汰后再次“加载更早”会多一次盘上读取；这是用 IO 换内存的显式取舍。
- 淘汰最旧条目时可能丢掉无 id 的头部条目；滚动区不需要头部，`first_entry_id` 仍从有 id 的条目重算。
