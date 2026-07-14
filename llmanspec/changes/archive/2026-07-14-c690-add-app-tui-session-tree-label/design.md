# Design — c690-add-app-tui-session-tree-label

1. Label 编辑替换树列表头下方（demo：`Label edit · Enter save · Esc cancel` + Input）。
2. Persist：`SessionEntry::Label { target_id, label }` via Driver；再 `set_annotation` 本地刷新（不必整树 remount）。
3. Shift+T：`TreeSelector::toggle_annotation_timestamps`；有 annotation 时 `annotation_at = "just now"`（展示用）。
4. labeled-only filter 依赖 annotation 有值（既有）。

## 验证

| 谁 | 命令 |
|---|---|
| Agent | harness Shift+L/T；`--strict` |
| 人类 | 正式 model：双 Esc → Shift+L → 输入 → Enter → 见 `[label]` |
