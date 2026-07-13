# Design — c615 live MessageHistory tree

## 依赖

必须先归档 **c610**（`SessionTreeKind` + Driver/REST）。本 change 只接线产品面。

## 数据流

```text
双 Esc
  → Driver.session_tree(MessageHistory)
  → map SessionTreeNode → package TreeNode (id/label/kind/children)
  → EditorSlot::Tree

Enter
  → Driver.travel_session_tree(MessageHistory, id)
  → apply SessionTreeTravel { leaf_id, editor_text }
  → 关树；editor.set_text(editor_text?)；重建/刷新 scrollback
```

## 与 stub 的差异

| | c491/c605 stub | c615 |
|---|---|---|
| 数据 | `sample_session_tree` | Driver MessageHistory |
| Enter | 本地 label 预填 + `travel → id` 注记 | 真 `travel_session_tree` |
| leaf | 不改 session leaf | Driver/store leaf 更新 |

假树 helper 可删或仅测用；生产路径 MUST NOT 依赖它。

## Fork（明确不做）

demo Shift+F = 同 session 兄弟分叉；`Driver::fork_session` = 新 session 文件。产品键位待产品拍板，记入 future.md。
