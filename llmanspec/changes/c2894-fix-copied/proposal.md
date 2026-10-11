---
depends_on: []
---

# Copied 通知栈贴视口右上角（不要贴对话内容顶）

## Why

产品词汇：**通知栈**（client notice stack）= 视口**右上角**浮层（r1930 / `TUI信息呈现与固定区词汇.md` C′）。Copied 现在看起来跟在**对话内容最顶一行**（启动卡 / 第一条 user），一滚就离开屏幕。用户要的是**当前终端可视范围**的右上角，滚 transcript 也不走。

**根因（2026-10-11 读代码）：**

`UiRoot::render` 先拼整棵 upper（loaded-resources + **全文 scrollback**）再拼下缘 dock，最后 `overlay_client_notices` 把 `✓ Copied` **写进 `lines[0..]`**（内容缓冲第 0 行起）。ApplicationOwned 把这整段丢进 `ScrollView` 再按 `scroll_top` 裁可见页。overlay 发生在裁切**之前**，所以落点是内容顶，不是视口顶。

现有单测 `client_notice_stack_right_aligns_copied_not_error_bar` 只断言 `UiRoot::render(80)[0]` 含 `✓ Copied`，**没有** ScrollView 裁切，所以门禁绿、真机错。

## What Changes（propose 时落地）

- Copied / 通知栈 MUST 叠在 **已裁切视口** 的右上角（屏幕空间），滚 transcript 时钉住。
- MUST NOT 写进 scrollback 内容第 0 行再随内容滚走。
- 仍不计入下缘 footprint；仍非 `Error: ` 通知条。
- 单测 / BDD 必须带「内容比视口高 + scroll_top > 0」仍看见右上角 Copied。

## 非目标（草案）

- **不实施**（仅 proposal；c2890 之后再 propose）。
- 不改通知条（底部 `Error: `）落点。
- 不改 Copied 文案 / TTL / 最多 3 条。

## Capabilities（propose 时）

- `app-tui-fixed-zone`（r1930 视口右上角）
- `app-tui-host`（ptim15 Copied 落点）

## Impact

- 长对话滚到底再复制：✓ Copied 仍在屏幕右上，不在用户气泡顶。
- 与 c2855 通知栈同槽；本票只修正 **视口 vs 内容原点**。

## Further Notes

- overlay：`src/app/tui/layout/root/render.rs` `overlay_client_notices`（`lines[i] = left + overlay`）。
- 调用点：同文件 `Component::render` 末尾，ScrollView 裁切之前。
- 合约：r1930「视口右上角」；词汇表「通知栈」。
