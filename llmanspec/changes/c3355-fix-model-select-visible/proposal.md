---
depends_on: []
title: 模型槽更换可见行时同时更新列表源和选中下标
needs_specs_change: false
---

# 模型槽更换可见行时同时更新列表源和选中下标

## Why

`SelectList` 用 `items` 做过滤源，用 `filtered_items` 做当前可见行，用 `selected_index` 指向可见行。`set_filter` 会同时改可见行并把下标归零。

`layout/slots/models.rs` 的 `apply_filter` 自己滤完之后，只写 `filtered_items` 和 `selected_index`，不更新 `items`。下一次若走组件自己的 `set_filter`，可见行会回到旧的源列表。

其余槽对 `filtered_items` 的使用是读取当前可见行。那些读取不必改。

## What Changes

在 `SelectList` 上增加一个方法。它同时写入源列表、可见行，并把选中下标夹在可见行长度内。模型槽的 `apply_filter` 改走这个方法，不再直接写这两个字段。

本 change 不把 `filtered_items` 改成私有，也不改 MCP、主题、能力槽和自动补全的读取。

## Capabilities

- `app-tui-input` 已要求模型槽过滤和方向键选择。本 change 不改规则文本。

## Impact

- 代码：`crates/xylitol-tui/src/components/select_list.rs`，`crates/xylitol-app-tui/src/layout/slots/models.rs`，以及该组件已有的过滤测试。
- 验证：组件测试覆盖「替换可见行之后，源列表与可见行一致，下标不越界」。现有 `/model` 槽场景保持通过。不新增 BDD 场景。
- 与 `c3345-refactor-ui-model-writes` 的文件不重叠。那份草案明确不改 `SelectList`。无 `depends_on`。
