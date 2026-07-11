# Design — c560-update-package-tui-tree-edges

## 空态

无可见行 → 一行 dim/no_match 文案（可主题化），不空白死屏。

## 选中稳定

```
filter/search 变更
  ├─ 旧 id 仍在可见列表 → 保持
  └─ 否则 → selected_index = 0（或首可见）
```
