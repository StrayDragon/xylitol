# Design notes — Session Tree vs pi（c454 之后增强清单）

> 调研源：`../pi/packages/coding-agent/.../tree-selector.ts` + `interactive-mode.ts`。
> **不归档进 c454**；后续由 c456 / 新 change 分批落地。冒烟（双 Esc 槽替换）已在 c454 验证。

## 槽模型（已对齐）

pi / xylitol：树 **替换 editor 槽**（`showSelector`），非居中 overlay。

## 操作差距（图 2）

| 能力 | pi | xylitol c454 现状 |
|---|---|---|
| ↑↓ / Enter / Esc | ✓ | ✓（`tui.select.*`；Esc 先清搜索） |
| ←→ / PgUp/PgDn 翻页 | ✓ | ✓（c456：←→ 绑 page） |
| 增量搜索 + Esc 清搜索 | ✓ | ✓（c456） |
| Filter：default / no-tools / user / labeled / all | ✓ Ctrl+D/T/U/L/A | ✓ demo（c456；包仅 `include_node`） |
| Cycle filter Ctrl+O | ✓（树内） | ✓ 树开时循环；关树 = 工具视口 |
| Fold ⊞/⊟ + Ctrl/Alt←→ 分支跳转 | ✓ | ✗ |
| Shift+L label / Shift+T 时间戳 | ✓ | ✗ |
| 选中行水平平移（深 indent） | ✓ | ✗ |
| 状态行 `(i/n) [filter]` | ✓ | 有 `(i/n)` |
| 路径 `•` / 选中 `›` | ✓ | ✓ |

## 建议落地顺序

1. ~~**c464**~~：SBS Diff 去整行红绿底 — 已归档。
2. ~~**c456**~~：搜索 + filter 五档 + ←→ 翻页 + 状态标签 — 本批。
3. 其后：fold / branch jump → label → 水平平移 → 产品 c491。

包继续只吃 `TreeNode { id, label, children }`；展示文案由 demo/app 预渲染进 `label`。
