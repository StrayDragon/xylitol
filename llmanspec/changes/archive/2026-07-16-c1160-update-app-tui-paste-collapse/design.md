# Design — c1160-update-app-tui-paste-collapse

## Decision

1. **包侧**：修正 `Editor::get_expanded_text`，按完整 marker 正则替换（对齐 pi `expandPasteMarkers`），避免残留 ` +N lines]`。
2. **产品侧**：`UiRoot::editor_text()` 改为返回 `get_expanded_text()`。host 发送路径（submit / steer / follow-up / history / Ctrl+G）已统一走该方法，无需逐点分叉。
3. **显示**：render 仍用 `get_text()`（经 Editor Component），折叠占位仅影响显示缓冲。
4. **不做**：paste marker 原子分段（pi `segmentWithMarkers`）——记入 `PI_DELTAS`，另开 change。

## Trade-offs

- `editor_text()` 语义从「显示文本」变为「发送文本」：对 slash/`!` 解析更正确（占位符不会误当命令）；若未来需要「读显示缓冲」再加 `editor_display_text()`。
