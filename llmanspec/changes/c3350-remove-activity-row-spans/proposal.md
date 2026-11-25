---
depends_on: []
title: 删掉折叠状态里只写不读的行区间
needs_specs_change: false
---

# 删掉折叠状态里只写不读的行区间

## Why

`ActivityFoldState.row_spans` 在绘制分段头时写入，在滚动区刷新时清空。全仓没有读者。每次绘制都在维护一份 nobody 使用的区间表。

## What Changes

删除 `SegmentRowSpans`、`row_spans` 字段、`live.rs` 里的 `insert`，以及 `scrollback/mod.rs` 里的 `clear`。

折叠级别、命中表和展开状态不动。它们才有读者。

## Capabilities

- `app-tui-fixed-zone` 与折叠呈现的既有规则不改。行区间不是产品行为。

## Impact

- 代码：`crates/xylitol-app-tui/src/activity_fold/state.rs`、`widgets/scrollback/live.rs`、`widgets/scrollback/mod.rs`。
- 验证：现有折叠与滚动区测试保持通过。不新增 BDD 场景。
- 与 `c3345-refactor-ui-model-writes` 的文件不重叠。无 `depends_on`。

## Further Notes

这是删除。不把测试脚手架收成宏。
