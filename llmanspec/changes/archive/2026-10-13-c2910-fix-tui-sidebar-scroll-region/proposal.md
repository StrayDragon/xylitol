---
depends_on:
  - c2905-add-tui-session-sidebar
title: 驻列滚动分区：gutter 贴内容右缘、鼠标区分驻列带
branch: sdd/c2910-fix-tui-sidebar-scroll-region
base_branch: main
base_sha: 6c9228d7994535e6c35e8879bb63e33b0392c557
---

# 驻列滚动分区：gutter 贴内容右缘、鼠标区分驻列带

## Why

c2905 驻列实机反馈：驻列开启后滚动条消失/错位，且鼠标滚轮与选区不区分驻列带——滚轮悬于驻列仍滚正文、选区可拖入驻列。根因：c2891 gutter/选区/滚轮几何全部按物理宽 `term-1` 口径，未感知宿主侧栏让宽。

## What Changes

- 包层 `ApplicationOwnedRuntime` 新增 `extra_reserved_cols`（宿主侧栏让宽，c2891 预留 seam 的兑现）：选中 rect、滚轮分区、gutter stamp/hit/drag 几何统一改按内容边界收口。
- 驻列带内滚轮不滚动正文、选区不得进入；gutter（滚动条）贴驻列左缘 = 正文右侧，带内无滚动条。
- 宿主每轮 drain 对账（`sync_session_sidebar_reserve`）：根常量 24 列 → 包层，开关切换/重建自愈；`Tui` 门面 + begin 重放同 c2891 模式。

## Capabilities

- `app-tui-fixed-zone`（r2904 场景补充：滚动条贴内容右缘 + 驻列带鼠标分区）

## Impact

仅包层 AO 几何口径与宿主对账；默认 extra=0 行为不变（c2891 回归全绿）。
