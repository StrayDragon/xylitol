---
depends_on:
  - c80-add-tui
  - c90-refactor-tui-core
---

# c105-fix-tui-initial-render

## Why

TUI 启动后可能出现“黑屏直到按键才显示 UI”的体验问题，或者首帧短暂出现后又被下一帧清空。这通常是因为在 ratatui 的 `Terminal::draw` 中使用了“只渲染 dirty 组件”的增量渲染：每一帧都会从空缓冲开始，如果某个组件本帧未渲染，它在屏幕上会被 diff 为空白覆盖。

另外，当前输入事件读取使用了 `tokio::task::spawn_blocking` 的常驻循环，可能导致退出（例如 Ctrl+D）后 Tokio runtime shutdown 等待 blocking task 结束，从而出现“需要 Ctrl+C 才能完全退出”的问题。

## What changes

- 在进入事件循环前执行一次初始绘制（clear + draw），确保 UI 立即可见。
- 修正渲染策略：每次 `Terminal::draw` 都完整渲染所有可见组件；dirty flag 仅用于决定“是否需要 draw”，而不是“draw 内是否渲染某组件”。
- 调整输入读取线程实现，确保 Ctrl+D 可以干净退出并恢复终端状态。

## Capabilities

- `tui-interface`

## Impact

- 影响 TUI 绘制与退出路径：避免黑屏/清帧，Ctrl+D 可干净退出；不改变 agent 逻辑与工具调用行为。
