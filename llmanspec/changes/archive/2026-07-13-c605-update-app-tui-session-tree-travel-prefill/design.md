# Design — c605 stub travel prefill

## 决议

| | stub（本 change） | demo（c600） | 真 Driver（后置） |
|---|---|---|---|
| user Enter | 预填 `label`；关树；`travel → id` 注记 | leaf=父 + payload 预填 + 重建 transcript | `navigateTree` + editorText |
| 非 user Enter | 不预填（清空 editor）；关树；注记 | leaf=id；重建路径 | 同 |
| 活树 | **否** | 是 | 是 |

Stub 无 history payload：正文 = `TreeNode.label`（kind 已分离）。

## 冻结边界（更新）

仍禁止：活图增长、filter/fork、Driver reach。
**允许**：user 预填 editor（UX 对齐 pi / demo）。
