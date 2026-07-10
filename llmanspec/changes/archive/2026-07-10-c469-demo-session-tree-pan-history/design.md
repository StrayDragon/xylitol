# Design — c469-demo-session-tree-pan-history

## 水平 pan

对齐 pi `renderHorizontalViewport`：光标 gutter（`› `）固定；仅 body 按选中行 anchor 列平移；窄宽下仍能看到选中 label 尾部。

## Travel 回复链

`path_ids_to(target)` 后，若单子链且子节点 label 为 `assistant:` / `tool:`，继续纳入显示路径；leaf 落到链末端，便于继续提交。分支点（多子）不自动展开。

## 活树

demo 持有可变 `session_tree`；打开树时 clone 进 `TreeSelector`。产品面仍用假静态树（c491），本 change 不推进产品活图。
