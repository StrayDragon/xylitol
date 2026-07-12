# design — c477 demo visual parity

## 问题

对照 `just demo-tui`：产品空闲底部像「空盒子」（Editor `max_vis≈5` 全空白）；busy 只有纯文本 Working、无 accent spinner；用户消息缺 `user-message-bg`。chrome 骨架（c475）与 live scrollback（c476）已齐，差在观感细节。

## 决策

### A. 空闲操作区高度

| 方案 | 说明 | 取舍 |
|---|---|---|
| **A1. 空草稿时 clamp 可见行=1** | app 层或 `Editor` 在 `is_empty` 时 `max_vis.min(1)` | ✅ 采用：改动小、对齐 demo 贴底感 |
| A2. 降低全局 `terminal_rows` | 影响有草稿时长高 | 否：多行编辑变挤 |
| A3. 空闲注入 seed transcript | 用内容「填满」视觉 | 否：产品空会话不应假数据 |

不变式：上下 `─` 始终保留；有草稿后恢复既有 `max_vis`。

### B. Busy spinner

复用包 `Loader`（或 demo 同款帧表）画在 **status 槽**，主题 `accent`；`UiRoot::tick` 转发。idle 仍清空 status → 0 行。

### C. User message bg

在 `scrollback` 用户行用 palette `user_message_bg` 做全行淡底（`apply_background_to_line` 或 theme helper）。默认开；不改 glyph 前缀语义。

## 非目标

c480 输入键位、c492 bash 边框、trust 策略、c491 活树。

## 风险

- clamp 时空草稿光标行被裁 → 单测锁「至少 1 内容行 + 边框」
- Loader tick 忘记接 → busy 静止；用 tick 单测或手动对照
