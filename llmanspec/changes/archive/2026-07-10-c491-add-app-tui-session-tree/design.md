# Design — c491 app-tui session tree（首切片）

## 切片边界

| 做 | 不做 |
|---|---|
| 假 `TreeNode` + 槽替换 | 从 `Driver::get_messages` 建树 |
| 双 Esc / Esc / Enter stub | 真 `navigate_tree` / fork API |
| Ctrl+C 仍清/退 | fold/filter 全套（包已有；产品可后开） |

## 槽模型

对齐 demo / pi：`tree_open` 时 `render` 在 editor 上下边框之间画树，不 blit 到 transcript 顶。

## Esc 优先级（树开）

1. 关树（本切片无 label 编辑 / 搜索也可后加）
2. （日后）清搜索 → 关树

空 editor 双 Esc（<500ms）开树；非空不触发。

## Travel stub

Enter：在 transcript 追加一行 `travel → {id}`（或等价 Text），关树。真 travel 等 Driver seam。
