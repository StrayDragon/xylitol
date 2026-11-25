# Design: c3355-fix-model-select-visible

## 目标

模型槽更换可见行时，`SelectList` 的源列表、可见行和选中下标一起更新。

## 两种形状

| | 形状 A | 形状 B |
|---|---|---|
| 接口 | `SelectList::replace_visible(&mut self, items: Vec<SelectItem>, selected_index: usize)` | 把 `filtered_items` 改成私有，模型槽把滤完的行放进 `items`，再调用 `set_filter("")` |
| 写入 | 方法同时写 `items` 和 `filtered_items`，下标夹在可见行长度内 | `set_filter` 从 `items` 再滤一次，并把下标归零 |
| 调用方 | `models.rs` 的 `apply_filter` 算出要保留的下标，然后只调用这个方法 | 模型槽还要在 `set_filter` 之后再改下标，才能留住当前焦点 |

## 选择

用形状 A。

`apply_filter` 已经按模型 id 做完模糊过滤，并按当前焦点算出下标。`replace_visible` 接收这份可见行和这个下标。方法把同一份向量写入 `items` 和 `filtered_items`，再用现有的夹取规则写下标。

`filtered_items` 保持公开。MCP、主题、能力槽和自动补全继续读它。

## 完成时的读法

替换之后，`items` 与 `filtered_items` 一致，`selected_index` 小于可见行长度。空列表时下标为 0。
