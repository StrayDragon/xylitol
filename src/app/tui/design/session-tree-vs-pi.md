# Design notes — Session Tree vs pi（c454 之后增强清单）

> 调研源：`../pi/packages/coding-agent/.../tree-selector.ts` + `interactive-mode.ts`。
> **不归档进 c454**；后续由 c456 / 新 change 分批落地。冒烟（双 Esc 槽替换）已在 c454 验证。

## 槽模型（已对齐）

pi / xylitol：树 **替换 editor 槽**（`showSelector`），非居中 overlay。

## 操作差距（图 2）

| 能力 | pi | xylitol 现状 |
|---|---|---|
| ↑↓ / Enter / Esc | ✓ | ✓（`tui.select.*`；Esc 先清搜索） |
| ←→ / PgUp/PgDn 翻页 | ✓ | ✓（c456：←→ 绑 page） |
| 增量搜索 + Esc 清搜索 | ✓ | ✓（c456；c595 起含 kind） |
| Filter：default / no-tools / user / labeled / all | ✓ Ctrl+D/T/U/L/A | ✓ demo（按 `kind` / annotation；包仅 `include_node`） |
| Cycle filter Ctrl+O | ✓（树内） | ✓ 树开时循环；关树 = 工具视口 |
| Fold ⊞/⊟ + Ctrl/Alt←→ 分支跳转 | ✓ | ✓（c467） |
| Shift+L label / Shift+T 时间戳 | ✓ | ✓ demo annotation（c467） |
| 选中行水平平移（深 indent） | ✓ | ✓（`render_horizontal_viewport`） |
| 状态行 `(i/n) [filter]` | ✓ | ✓ |
| 路径 `•` / 选中 `›` | ✓ | ✓ |
| Kind / role 前缀 | ✓ 产品树按 entry.role 着色 | ✓ **c595**：`TreeNode.kind` + `kind_prefix` 主题；label 纯正文 |
| Enter travel → user 预填 editor | ✓ `editorText`；leaf=父 | ✓ **c600** demo |
| Enter travel → 非 user | ✓ leaf=target | ✓ **c600** demo |
| 树内 / 会话 fork | ✓ `/fork`（新会话文件） | ✓ demo 同会话 Shift+F（leaf=选中；user 预填） |
| 流中 Enter steer / Alt+Enter follow-up | ✓ | ✓ demo 队列（不打断当前轮） |
| 提交/工具写入活树 | ✓ | ✓ `session_tree` 增长 |

## 建议落地顺序

1. ~~**c464**~~：SBS Diff 去整行红绿底 — 已归档。
2. ~~**c456**~~：搜索 + filter + ←→ — 已归档。
3. ~~**c467**~~：fold / 分支跳转 / annotation — 已归档。
4. ~~水平平移~~ + demo history travel — 包 pan + `agent_demo` 路径重建。
5. ~~**c595**~~：`TreeNode.kind` 库机制 + DESIGN/playground SSOT。
6. ~~**c600**~~：demo travel 对齐 pi（user → 父 leaf + editor 预填）。
7. 产品真 session / `Driver` travel（c491 假树之后）。

包吃 `TreeNode { id, label, children, annotation?, kind? }`；**kind 前缀由主题画**，host **不**把 role 字符串烘焙进 `label`。
