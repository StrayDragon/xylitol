# Design notes — Session Tree vs pi（c454 之后增强清单）

> 调研源：`../pi/packages/coding-agent/.../tree-selector.ts` + `interactive-mode.ts`。
> **不归档进 c454**；后续由 c456 / 新 change 分批落地。冒烟（双 Esc 槽替换）已在 c454 验证。
> **刻意不做 / 不得回退**：见 [`../PI_DELTAS.md`](../PI_DELTAS.md)（如 travel 分支摘要 A01）。本文件是能力差距清单，不是「全部要对齐 pi」。

## 槽模型（已对齐）

pi / xylitol：树 **替换 editor 槽**（`showSelector`），非居中 overlay。

## 操作差距（图 2）

| 能力 | pi | xylitol 现状 |
|---|---|---|
| ↑↓ / Enter / Esc | ✓ | ✓（`tui.select.*`；Esc 先清搜索） |
| ←→ / PgUp/PgDn 翻页 | ✓ | ✓（c456：←→ 绑 page） |
| 增量搜索 + Esc 清搜索 | ✓ | ✓（c456；c595 起含 kind） |
| Filter：default / no-tools / user / labeled / all | ✓ Ctrl+D；T/U/L/A **toggle** | ✓ demo set；**c635** 产品对齐 pi toggle + default 藏 meta |
| Cycle filter Ctrl+O | ✓（+ Shift+Ctrl+O 反向） | ✓ forward **c635**；✓ backward **c685** |
| Fold ⊞/⊟ + Ctrl/Alt←→ 分支跳转 | ✓ | ✓（c467 / 产品 **c640**） |
| Shift+L label / Shift+T 时间戳 | ✓ | ✓ demo；✓ 产品 **c690** |
| 选中行水平平移（深 indent） | ✓ | ✓（`render_horizontal_viewport`） |
| 状态行 `(i/n) [filter]` | ✓ | ✓ |
| 路径 `•` / 选中 `›` | ✓ | ✓ |
| Kind / role 前缀 | ✓ 产品树按 entry.role 着色 | ✓ **c595**：`TreeNode.kind` + `kind_prefix` 主题；label 纯正文 |
| Enter travel → user 预填 editor | ✓ `editorText`；leaf=父 | ✓ **c615** 产品 `travel_session_tree` |
| Enter travel → 非 user | ✓ leaf=target | ✓ **c615** 产品 |
| 树内 / 会话 fork | ✓ `/fork`（新会话文件；user 选择器） | ✓ 产品 Shift+F / `/session-fork` **c645/c1005**（Driver 新 session；非 user 选择器）；demo 同会话保留 |
| TreeHelp + Search 行 | ✓ 动态键位 | ✓ 产品 **c685**（树槽头行 / layout） |
| 流中 Enter steer / Alt+Enter follow-up | ✓ | ✓ demo 队列（不打断当前轮） |
| 提交/工具写入活树 | ✓ | ✓ `session_tree` 增长 |

## 建议落地顺序

1. ~~**c464**~~：SBS Diff 去整行红绿底 — 已归档。
2. ~~**c456**~~：搜索 + filter + ←→ — 已归档。
3. ~~**c467**~~：fold / 分支跳转 / annotation — 已归档。
4. ~~水平平移~~ + demo history travel — 包 pan + `agent_demo` 路径重建。
5. ~~**c595**~~：`TreeNode.kind` 库机制 + DESIGN/playground SSOT。
6. ~~**c600**~~：demo travel 对齐 pi（user → 父 leaf + editor 预填）。
7. **c615** 产品 `Driver::session_tree` + `travel_session_tree`（MessageHistory）。
8. 产品 filter / fold / Shift+F fork → **c635 / c640 / c645**（设计闸 **c625**；demo 已有）。
9. 产品树槽 Search/Help + cycleBackward → **c685**；label 持久化 **c690**；E2E **c705**（强制）。

包吃 `TreeNode { id, label, children, annotation?, kind? }`；**kind 前缀由主题画**，host **不**把 role 字符串烘焙进 `label`。
