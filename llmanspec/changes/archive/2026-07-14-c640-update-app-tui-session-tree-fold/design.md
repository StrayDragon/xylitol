# Design — c640-update-app-tui-session-tree-fold

## 现状

| 层 | Fold |
|---|---|
| **pi** | `app.tree.foldOrUp` / `unfoldOrDown`：可折则 fold/unfold，否则分支段跳转 |
| **包** | 已实现（`folded_nodes`、⊞/⊟、`tui.tree.foldOrUp|unfoldOrDown`） |
| **demo** | 已接线 |
| **产品** | 树槽只转发裸 `left`/`right`（翻页）→ **Ctrl/Alt+←→ 被丢弃** |

## 决议

1. **接线形态**：转发事件给 `tree.handle_input`，不在 app 复制 fold 集合。
2. **键**：沿用包默认 Ctrl/Alt+←→（对齐 pi / demo）。
3. **裸 ←→**：继续翻页（既有）；**MUST NOT** 改成 fold。
4. **与 filter**：Ctrl+D/T/U/L/A/O 仍由产品先消费；fold 键不冲突。
5. **Annotation Shift+L/T**：**本 change 不做**（future；包已有 `editLabel` / `toggleLabelTimestamp`，产品另开）。

## Non-goals

- fork（c645）；持久化折叠集；改包 fold 语义
