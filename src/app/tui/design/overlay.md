# Overlay

## MUST

1. 仅用于确认框等**短交互**：居中短面板 + `OverlayHandle`。
2. 命令面板 / 设置 **MUST NOT** 做成大 overlay 仪表盘——走 editor 槽替换（见 [`editor.md`](./editor.md)）。
3. **MUST NOT** blit 弹层到内容绝对顶部遮挡 scrollback 语义。
