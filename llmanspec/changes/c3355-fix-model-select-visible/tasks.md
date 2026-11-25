# Tasks: c3355-fix-model-select-visible

## 1. 可见行与源列表一起换

- [ ] 1.1 按 `design.md` 给 `SelectList` 加上 `replace_visible`。它写入 `items` 和 `filtered_items`，并把 `selected_index` 夹在可见行长度内。
- [ ] 1.2 `models.rs` 的 `apply_filter` 和 `rebuild_keep_selection` 改走这个方法，不再给 `filtered_items` 和 `selected_index` 赋值。`select_row_at` 对可见行的读取保留。`filtered_items` 保持公开。

## 2. 现有过滤测试

seam：`SelectList` 已有的过滤测试，以及现有 `/model` 槽场景。不新增 BDD。

- [ ] 2.1 组件测试覆盖：替换可见行之后，源列表与可见行一致，下标不越界。空列表时下标为 0。
- [ ] 2.2 现有 `/model` 槽场景保持通过。

## 3. 临时评判（deslop-please）

不进 `just qa`。基线是 `.tmp/deslop-please.allow`。测试路径诊断见 `.tmp/deslop-please-deferred-tests.md`，本 change 不改。全仓基线只由不带 crate 参数的 `deslop-please --update-allow .tmp/deslop-please.allow` 重写。只检查一个 crate 时，其它 crate 的条目不在这次输出里，它们仍留在基线中。

- [ ] 3.1 在本 change 分支上跑 `deslop-please xylitol-app-tui` 和 `deslop-please xylitol-tui`。`SelectList::selected_index` 从报告里消失。`ModelsSlot::apply_filter` 不再出现在 `SelectList::filtered_items` 的记录里。MCP、主题、能力槽和自动补全的读取留在基线里。
- [ ] 3.2 对这两个 crate 跑 `deslop-please <crate> --allow .tmp/deslop-please.allow`。没有基线以外的新诊断。
