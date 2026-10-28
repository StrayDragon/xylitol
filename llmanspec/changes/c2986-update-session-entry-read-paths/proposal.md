---
depends_on: []
title: 收敛会话条目全量读取路径
needs_specs_change: false
---

# 收敛会话条目全量读取路径

## Why

- `effects::session_entries` 仍以 `GetMessages { limit: None }` 全量读取；
- session tree、fork、compact 等路径都走它，长会话下会加载整条历史。
- `c2954` 的“只读所需”只覆盖了首屏与滚顶分页；树/分叉/压缩仍是全量路径。
- 全量读取本身不是错误（compact 语义上需要完整历史），但需要逐路径确认是否真的需要，避免无谓的 IO 与内存峰值。

## What Changes

- 审计 `session_entries` 的调用点，逐路径判断：
  - session tree：是否可用既有 `SessionTree` 查询或分页；
  - fork：是否只需目标条目到叶子的分支，而不是整条历史；
  - compact：是否必须全量（若是，保留并注明）；
  - 其他路径（estimate、slash 等）：按需分页。
- 对可收敛的路径改用分页/定向查询；不可收敛的路径保留全量并加注释说明原因。
- 补测试/基准：长会话下 tree/fork 的读取量下降，行为不变。

## Capabilities

- `app-tui-host`：会话条目读取路径。
- `agent-session-store`：分页/分支查询能力（若需新增）。

## Impact

- `crates/xylitol-app-tui/src/effects/mod.rs`：`session_entries`。
- `crates/xylitol-app-tui/src/effects/pending_ui/tree.rs`、`effects/slash/compact.rs`：调用点。
- 可能涉及 `XySessionStore` 查询 API。

## Out of Scope

- 历史窗口分页（`c2981`）。
- 压缩算法本身。

## Further Notes

- 由 PR #30 review F9 记录。
- 纯性能/IO 优化，产品行为不变；`needs_specs_change: false`。
