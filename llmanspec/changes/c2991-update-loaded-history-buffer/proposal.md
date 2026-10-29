---
depends_on: []
title: 给 TUI 历史缓冲设置上限
needs_specs_change: true
---

# 给 TUI 历史缓冲设置上限

## Why

- c2981 为支持“更早内容已收起（点击展开）”，`LoadedHistory` 保留所有已加载条目；`HISTORY_WINDOW = 800` 只裁剪 UI 模型，缓冲本身无上限。
- 用户持续向上翻页时，缓冲会逐渐接近全量历史；长会话下是持续增长的内存峰值。
- 缓冲无上限也让“已收起”与“加载更早”两种状态失去明确边界。

## What Changes

- 给 `LoadedHistory` 设置上限 `HISTORY_WINDOW + HISTORY_PAGE`（1000 条）。
- 合并/初始化超出上限时淘汰最旧条目，并把 `has_more` 置为 `true`（被淘汰条目仍在盘上）。
- `prepend_history` / `apply_switched_session` 的提示使用缓冲的真实 `has_more`，避免提示与可达性不一致。
- 更新 `app-tui-fixed-zone` 的历史窗口规则并补场景。

## Capabilities

- `app-tui-fixed-zone`：历史窗口缓冲上限与提示状态。

## Impact

- `crates/xylitol-app-tui/src/history_window.rs`：缓冲上限、淘汰与 `first_entry_id` 重算。
- `crates/xylitol-app-tui/src/host/session_ops.rs`：提示读取真实 `has_more`。
- `tests/bdd/steps_app_tui_fixed_zone.rs` + `bindings_app_tui_fixed_zone.rs`：场景。

## Out of Scope

- 不改分页 API 形状。
- 不改 `HISTORY_PAGE` / `HISTORY_WINDOW` 的既有取值。
- 不把展开改成异步回源（由淘汰 + `has_more` 回退覆盖）。
