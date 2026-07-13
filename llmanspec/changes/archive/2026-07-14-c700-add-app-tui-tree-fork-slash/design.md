# Design — c700-add-app-tui-tree-fork-slash

## 决策

| 点 | 选择 |
|---|---|
| `/tree` | 置 `pending_tree_open`，复用双 Esc 装载路径 |
| `/fork` | fork **当前 leaf**（非 pi 的 user 选择器）；无 leaf → 短提示 |
| busy | 与 `/model` 一致：拒绝 + note |
| 补全 | SlashCommandSource 描述即可（无参命令） |

## 与 pi 差异

pi `/fork` → user message selector；xylitol → leaf fork。选任意节点仍：`/tree` 或双 Esc → Shift+F。
