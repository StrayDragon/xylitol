# design — c575 overlay focus-restore

## 代码事实

| 已有（c445） | 缺口（D08） |
|---|---|
| OverlayHandle hide/set_hidden/focus/unfocus | 无 eligible/blocked/resume |
| `pre_focus: Option<usize>` 仅 root | nested overlay→overlay 链与 retarget |
| hide 仅当 was_focused 才 restore | 临时 `set_focus` 偷焦点后无 reclaim |
| `focus` 拒绝 non_capturing | pi 允许显式 focus NC |
| dispatch 跳过 NC 即使 focused | 显式 focus 的 NC 应收键 |

对照：`../pi/packages/tui/src/tui.ts` + `test/overlay-non-capturing.test.ts`。

## 硬决议

1. **FocusTarget** = `Root(usize)` | `Overlay(u64)`；`pre_focus` 存当前焦点目标快照。
2. **Restore**：`inactive` | `eligible{id}` | `blocked{id, blocked_by, resume}`；`resume` = restore-overlay | focus-target。
3. **set_focus(public)** → policy Clear；hide/invisible redirect → Preserve。
4. **dispatch reclaim**（listeners 之后）：焦点不在 overlay 时，eligible 夺回；blocked 且 `blocked_by ≠ current` 则 resume。
5. **NC**：show / set_hidden(false) 不抢焦点；`focus()` 可显式夺取并参与 eligible reclaim。
6. **不接** `visible(w,h)`（本切片）；不改产品壳。

## 验收矩阵（MUST）

| ID | Then |
|---|---|
| F1 | capturing show → overlay focused；hide → 回到 pre_focus |
| F2 | overlay 可见时 `set_focus(root)` → 下一 input reclaim overlay |
| F3 | `unfocus()` → 不 reclaim；root 保焦 |
| F4 | `set_focus(None)` 清 restore；不 reclaim |
| F5 | NC show 不抢焦；`focus()` 可夺焦；`set_hidden(false)` 不自动夺焦 |
| F6 | nested hide 经 retarget 回到正确祖先 / root |
| F7 | `unfocus(target)` 在 blocked 时改写 resume |

## 非目标

产品接线、`visible()`、改 host 驱动模型。
