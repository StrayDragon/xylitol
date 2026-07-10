# Design — c454-add-package-tui-tree-selector

## Decisions

1. **Generic `TreeNode`** — `{ id, label, children }`；不绑 session entry 类型。产品/demo 自己映射摘要文案。
2. **Flatten 对齐 pi** — 单子链保持扁平 indent；仅在分支点 +1；多 root 用 virtual root 语义；过滤后 `recalculate_visual_structure`。
3. **Filter = `include_node` 钩子** — 不移植 pi 五档 FilterMode。
4. **导航复用 `tui.select.*`** — 与 SelectList 同键；确认/取消回调。
5. **Demo 冒烟在本变更** — 双 Esc 打开假树，证明槽替换可行；c456 可再收紧时间窗/文档。

## Alternatives rejected

- 扩展 SelectList 加 indent 字段（连接符/gutter/过滤后重算会拧巴）。
- 把 TreeSelector 放进 `src/app/tui`（准通用，违反包边界）。
