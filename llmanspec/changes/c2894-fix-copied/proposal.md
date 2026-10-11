---
depends_on: []
---

# Copied 通知栈贴视口右上角（不要贴对话内容顶）

## Why

产品词汇：**通知栈**（client notice stack）= 视口**右上角**浮层（r1930 / `TUI信息呈现与固定区词汇.md` C′）。Copied 现在看起来跟在**对话内容最顶一行**（启动卡 / 第一条 user），一滚就离开屏幕。用户要的是**当前终端可视范围**的右上角，滚 transcript 也不走。

**根因（2026-10-11 读代码，两层）：**

1. `UiRoot::render` 把 `✓ Copied` 写进完整内容缓冲 `lines[0..]`（`overlay_client_notices`），再交给 ApplicationOwned `project_frame`：内容进 `ScrollView`，按 `scroll_top` 裁可见页。overlay 发生在裁切**之前**，落点是内容顶，不是视口顶。
2. 滚轮走 `reproject_frame`：**不**再调 `Component::render`。通知被烤进缓存的内容行 0，即使把 overlay 改到 `render()` 里按 `scroll_top` 写，滚轮帧也不会重画，Copied 仍跟着旧内容行走。

现有单测 `client_notice_stack_right_aligns_copied_not_error_bar` 只断言 `UiRoot::render(80)[0]` 含 `✓ Copied`，**没有** ScrollView 裁切，也没有滚轮再投影，所以门禁绿、真机错。主机泵 BDD `render_frame` 同样只调 `UiRoot::render`，不经 AO 裁切。

## What Changes

- 通知栈 MUST 叠在 **已裁切视口** 的右上角（屏幕空间），滚 transcript（含只投影、不重绘组件的滚轮帧）时钉住。
- MUST NOT 把通知写入 ScrollView 内容缓冲第 0 行再随 `scroll_top` 滚走。
- 仍不计入下缘 footprint；仍非 `Error: ` 通知条。
- 单测 / BDD 必须带「内容比视口高 + 视口未停在内容顶」仍看见右上角 Copied；既有只断言未裁切 `render()[0]` 的用例 MUST 改到可见视口。

## 非目标

- 不改通知条（底部 `Error: `）落点。
- 不改 Copied 文案 / TTL / 最多 3 条。
- 不改 xylitol-tui 的 content-end / follow-bottom 滚动语义。
- 不在本票做栈内键盘动作或展开侧板。

## Capabilities

- `app-tui-fixed-zone`（r1930 视口右上角；本票收口「已裁切视口」）
- `app-tui-host`（r1261 Copied 落点仍指向 r1930，不另开 req）

## Impact

- 长对话滚到底再复制：✓ Copied 仍在屏幕右上，不在用户气泡顶。
- 滚轮只投影时同样钉住。
- 与 c2855 通知栈同槽；本票只修正 **视口 vs 内容原点**。

## Further Notes

- overlay 现状：`src/app/tui/layout/root/render.rs` `overlay_client_notices`（`lines[i] = left + overlay`），`Component::render` 末尾、AO 裁切之前。
- 裁切：`packages/xylitol-tui/src/application_owned_runtime.rs` `project_frame` / `reproject_frame` → `paint_visible`（`ScrollView::visible_lines` + dock）。
- 主机泵 BDD：`tests/bdd/steps_app_tui_host.rs` `render_frame` → `UiRoot::render`。
- 合约：r1930「视口右上角」；词汇表「通知栈」。
