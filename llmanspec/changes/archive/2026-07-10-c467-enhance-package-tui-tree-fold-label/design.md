# Design — c467 tree fold / label

## Fold

- 可折叠：有可见子节点，且为可见根，或其可见父有多个可见子（segment start）。
- Ctrl/Alt+←：若可折叠且未折 → fold；否则跳到上一分支段起点。
- Ctrl/Alt+→：若已折 → unfold；否则跳到下一分支段起点（或叶）。
- 连接符第二格：`⊞`（已折）/ `⊟`（可折）/ `─`（不可折）；无连接符的已折根前缀 `⊞ `。
- 搜索/filter 变更时清空 folds（对齐 pi）。

## Label（annotation）

- `TreeNode` 增加可选 `annotation` / `annotation_at`（展示串由 host 预格式化）。
- 渲染：`[annotation] ` +（可选）muted timestamp + `label`。
- Shift+L → `on_label_edit(id, current_annotation)`；demo 用槽内 Input 提交。
- Shift+T → 切换 `show_annotation_timestamps`。

## 键位

| id | 默认 |
|---|---|
| `tui.tree.foldOrUp` | ctrl+left, alt+left |
| `tui.tree.unfoldOrDown` | ctrl+right, alt+right |
| `tui.tree.editLabel` | shift+l |
| `tui.tree.toggleLabelTimestamp` | shift+t |
