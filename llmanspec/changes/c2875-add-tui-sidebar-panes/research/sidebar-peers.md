# 边栏对照

## xylitol

- DESIGN：无双栏。槽 = editor 替换。待办栏 = 下缘占用行。通知栈 = 视口右上、0 dock 行。
- 引擎：垂直栈，无 2D layout tree。`Panel`/`Container` 不是边栏。

## 对照

- **pi**：TUI 无边栏；HTML export 才有 CSS sidebar。
- **opencode**：`SESSION_SIDEBAR_WIDTH = 42`；宽屏 occupancy + `config.session.sidebar=auto`；窄屏 absolute drawer + 70% 黑遮罩。右栏与 terminal/panel 互斥。
- **crush**：`sidebarWidth := 30` 水平拆；`<120` 或矮屏隐藏，改 header / compact overlay。`toggle_sidebar` 命令。

占用列动画 = 每帧改 wrap 宽 = 全量 scrollback 失效（D19）。不要做。

## ratatui

侧栏 = `Layout::horizontal([Length(30), Fill(1)])` 两个 `Rect`。xylitol 无 Constraint；`dock_rows` 是行带不是列。overlay 抽屉若盖 transcript，必须 c2858 capture，否则点到选区。
