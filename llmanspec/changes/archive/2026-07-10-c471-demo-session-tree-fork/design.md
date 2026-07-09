# Design — c471-demo-session-tree-fork

## 与 pi / travel 的区别

| | Enter travel | Shift+F fork |
|---|---|---|
| 显示路径 | root→id + 线性 assistant/tool 链 | 仅 root→id |
| leaf | 链末端 | **选中 id** |
| 编辑器 | 不清（或空） | user 节点则预填原文 |
| 目的 | 回看/续写该回复之后 | 从该点改写，旧子树成兄弟分支 |

下次 `commit_user_turn` 仍 `grow_session_tree` 挂在 leaf 下 → 自然分叉。
