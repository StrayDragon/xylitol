---
depends_on: []
title: 清理 TUI 验收尾项
needs_specs_change: false
branch: sdd/c3155-update-tui-acceptance-cleanup
base_branch: main
base_sha: f5452d96350c456d0e2792987e303dbb0bf1354f
---

## Why

`c3150` 已补齐行为验收，但留下三项实现尾项：详情 popup 的 `session_popup` 状态 / pending 方法不可达；`UiEntry::Tool` 没有独立 `args_full`；CPU lab 在 macOS 无法采样，且没有 1000 delta/s 预算场景。本 change 只做内部清理与证据补强，不改产品行为。

## What Changes

- 删除 `session_popup_open/id`、pending 字段、root/host/effects 方法、BDD step 与测试。
- `UiEntry::Tool` 增加 `args_full`；bridge 填充完整参数；paint 用 `args_full`，header 仍用 `args_preview`；缓存指纹纳入。
- CPU lab 改用 `getrusage`；新增 1000 delta/s、33 delta/tick 场景并断言 <20%。
