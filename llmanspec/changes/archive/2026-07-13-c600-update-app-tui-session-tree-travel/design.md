# Design — c600 travel 对齐 pi

## 语义对照

| 选中 | pi | 本 change（demo） |
|---|---|---|
| `kind=user` | leaf=parent；`editorText`→editor（空时） | 同：leaf=父 id（根 user → 空会话/虚拟根策略与 demo 一致）；`set_text` 用户正文 |
| 非 user | leaf=target；无 editorText | leaf=选中 id；不清/不强制预填（demo：清空 editor 以免残留） |
| 回复链 | 无「强制跟 spine」 | **废除** `travel_path_with_replies` 作为 Enter 默认 |

## 与 fork 分工

| | Enter travel | Shift+F fork |
|---|---|---|
| 目的 | 导航到历史点（user=改写再发） | 同会话分叉，leaf 停在选中 |
| user 预填 | ✓ | ✓ |
| 含子回复 | user：否 | 否 |

## SSOT

`session-tree.md` MUST 写清上述表；`session-tree-vs-pi.md` 将「Enter travel → 回复链」改为「对齐 pi editorText」；playground 注脚与之一致。c491 stub 仍只记 `travel → id`，**不**在 stub 上扩语义。
