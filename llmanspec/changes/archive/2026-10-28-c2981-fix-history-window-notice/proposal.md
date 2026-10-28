---
depends_on: []
title: 修正历史窗口提示文案与被裁条目的可达性
needs_specs_change: true
branch: sdd/c2981-fix-history-window-notice
base_branch: main
base_sha: 19d9dd85680b12cf1be6c5b797e30879b2353325
---

# 修正历史窗口提示文案与被裁条目的可达性

## Why

- `app-tui-fixed-zone` 的 `r2936` 要求：窗口超限时最旧一页回落为顶部提示行（形如 `↑ 加载更早的 200 条（还有 N 条）`）；盘上无更早条目时不得渲染该提示行。
- 当前提示文本是 `↑ 加载更早的 200 条`，缺少剩余条数；`want_notice = has_earlier || drop > 0`，在“窗口溢出但盘上已无更早条目”时仍显示提示。
- 更严重的是可达性：窗口裁剪会丢掉最旧一页，但点击提示时用 `oldest_loaded_entry_id` 查询（比所有已加载条目都旧），返回空 → 提示被清掉，被裁掉的条目再也无法显示。
- `LoadedHistory` 保留所有已加载条目，内存并未真正被 800 限制；窗口上限只作用于 `ui_model.entries`。

## What Changes

- 提示文本补齐剩余条数（或按决策改为“还有更早内容”，见 design）。
- 区分两种状态：
  - 盘上还有更早条目 → 显示“加载更早”；
  - 仅窗口溢出（盘上已无更早）→ 显示“更早内容已收起/点击展开”，不得冒充“加载更早”。
- 修复被裁条目的可达性：加载更早时以“最早已显示条目”为游标，而不是“最早已加载条目”；优先从缓冲恢复，必要时回源存储。
- 明确 `LoadedHistory` 的内存边界（保留窗口 + 一页，或按需回源）。
- 补 BDD：会话 >800 条、加载完所有分页后触发裁剪，断言提示文案、点击后条目恢复且滚动锚点不跳。

## Capabilities

- `app-tui-fixed-zone`：历史窗口提示行与加载更早语义。

## Impact

- `crates/xylitol-app-tui/src/history_window.rs`：提示文案、裁剪与游标。
- `crates/xylitol-app-tui/src/host/session_ops.rs`：`oldest_loaded_entry_id` / 新增 `oldest_displayed_entry_id`。
- `crates/xylitol-app-tui/src/effects/pending_ui/history.rs`：加载更早请求。
- `tests/bdd`：>800 条场景。

## Out of Scope

- 存储层分页 API 的重新设计（除非为“剩余条数”必须扩展，见 design）。
- 其他 ScrollNotice（通知条 / 待办栏）的语义。
- 历史窗口上限值（200/800）的调整。

## Further Notes

- 由 PR #30 review F5 记录。
- 提示文案里的 `N` 需要“盘上剩余条数”；当前分页 API 只返回 `has_more`，是否扩展存储 API 是本次决策点。
