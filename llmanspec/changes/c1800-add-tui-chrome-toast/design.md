# Design: c1800 chrome toast

## 渲染序（操作区上方 · 示意）

```text
[ optional chrome toast · 1 line · muted · auto-clear ]
[ status: spinner + short · optional next-turn cue ]   # busy only; idle = spacer
[ editor / overlay slot ]
[ footer ]
```

Toast **不**进入 `UiModel.entries`，不参与 scrollback rebuild。

## API 意向

```text
HostSession::push_chrome_toast(text)
  → 设 toast 文本 + deadline（now + TTL）
  → request_render

idle_tick / step
  → 过期则 clear toast
```

新 toast 覆盖旧 toast（单槽）。

## 与 c1780

`BUSY_SESSION_SWITCH_NOTICE` 常量保留；`pending_ui` 拒闸改为 `push_chrome_toast`，删除对该路径的 `push_scroll_notice`。

## 非目标

- 多行 toast 栈
- 鼠标关闭
- Web 面实现（语义可同源，本波只 TUI）
