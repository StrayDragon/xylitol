# Design — c456-demo-session-nav-keys

## 分层

| 能力 | 落点 | 理由 |
|---|---|---|
| 增量搜索、←→ 翻页、`(i/n)` + 可选 suffix | 包 `TreeSelector` | 通用；与 SelectList 翻页对称 |
| FilterMode 五档 + 键位 | `agent_demo`（日后 app） | 产品语义（tool/user/label）；包只吃 `include_node` |
| 双 Esc 槽替换 | 已在 c454 | 本变更不重做 |

## Ctrl+O 冲突

- **树关闭**：Ctrl+O = 工具详情视口（ExpandableOutput）
- **树打开**：Ctrl+O = 循环 filter（对齐 pi `app.tree.filter.cycleForward`）

## 搜索 vs Esc

1. 有 `search_query` → Esc 清空搜索并 `apply_filter`
2. 无搜索 → Esc → `on_cancel` / demo 关树

## Filter 谓词（demo 假数据约定）

标签前缀约定（预渲染进 `TreeNode.label`）：

- `user:` / `assistant:` / `tool:`
- 含 `[label]` 视为 labeled

| Mode | 谓词 |
|---|---|
| default | 隐藏纯 tool 叶？对齐 pi default（通常藏部分噪声）— demo：**显示全部非虚拟噪声**，与 `all` 接近；或隐藏无 children 的 tool 行。简化：**default = 全部**（与 all 同），toggle 键仍可演示切换。 |
| no-tools | label 不含 `tool:` |
| user-only | label 含 `user:` |
| labeled-only | label 含 `[label]` |
| all | 全部 |

> pi default 更复杂（跳过部分 tool 中间态）。demo 以可测谓词为准，不追求与 pi 字节级一致。

## 非目标

fold / branch jump / label 编辑 / 水平平移 → 后续 change。
