# 打字机 / 流式性能对照

## xylitol 现状（已够用的部分）

- 合帧：`TICK_BUSY_MS = 16` + 引擎 `MIN_RENDER_INTERVAL_MS = 16`；host 合并连续 Text/Thinking delta，stream 帧 DEFER 到 tick。
- 前缀：`find_stable_markdown_prefix_end`（最后一个不在围栏内的 `\n\n`）→ 只重 Markdown 不稳定后缀。
- 已落地：c1500 paint cache、c1508/c1509 wrap 快路径、c1510 streaming prefix、A+B finalize 行复用。
- thinking：每帧 **整段** plain wrap（无 Markdown）。
- 抖动主因：开着的围栏 / 表格 / 列表让已可见行高度变；半截闭合围栏无修剪。

## 对照

- **pi 1.1.0**：同样 16ms；`trimPartialClosingFences`；产品每次 `message_update` 仍 `clear()` 重建 Markdown 子树。xylitol 的 fingerprint cache + 稳定前缀 **强于** pi 产品路径。
- **opencode**：稳定 markdown 块 `commitRows` 进终端 scrollback，只留最后一块可变。架构与「AO 全量逻辑 transcript」不相容；可借鉴块稳定行范围。
- **crush**：list 视口虚拟化，但 stream 时清缓存、整消息 wrap。

## D12 / ratatui

继续 `render → Vec<String>` ANSI。块稳定 = 冻结前缀行切片，不是新行类型。ratatui-markdown 每帧画 `Line`，无稳定前缀缓存，不能当打字机模型。本票与 c2858 鼠标管线无关。
