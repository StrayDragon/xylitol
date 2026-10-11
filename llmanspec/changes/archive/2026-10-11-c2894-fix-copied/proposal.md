---
depends_on: []
branch: sdd/c2894-fix-copied
base_branch: main
base_sha: f18d473da069110b9d1c23171044bb3c545b4d9d
---

# Copied 通知栈贴视口右上角；导航瞬时改回滚动提示

## Why

产品词汇：**通知栈** = 视口**右上角**浮层（r1930 / C′）；**滚动提示** = 主滚动区尾插、跟底时在输入框上方（B 导航瞬时）。

真机两条错位：

1. Copied 跟在**对话内容最顶一行**，一滚就离开屏幕。用户要的是当前终端可视范围的右上角。
2. 切 session 的 `switched →` 进了同一条通知栈（TTL、会消失、还会一起滚走）。用户要的是输入框上方、不消失的滚动提示。词汇表 B 与 r1330 已这样写；代码 `apply_switched_session` 却 `push_toast_info_notice`（resume-render 注释把它们当成 ephemeral 栈）。树 travel 的 `history @`、树 fork 的 `forked →` 仍是尾插，一家分裂。

**根因（Copied / 栈）：**

1. `UiRoot::render` 把栈写进完整内容缓冲 `lines[0..]`，再交给 AO `project_frame` 按 `scroll_top` 裁。overlay 在裁切之前。
2. 滚轮走 `reproject_frame`，不再调 `Component::render`。通知烤在内容行 0。

既有单测 / 主机泵 BDD 只看未裁切 `UiRoot::render()[0]`，门禁绿、真机错。resume-render 单测还**禁止** `switched →` 进 transcript，和词汇表对着干。

## What Changes

- 通知栈（Copied / Copied Last Msg / Heuristic ◆）MUST 叠在**已裁切视口**右上角；滚 transcript（含只投影的滚轮帧）时钉住。MUST NOT 写入 ScrollView 内容第 0 行。
- `/history-copy-last` 成功 MUST 通知栈 `Copied Last Msg`（✓）；MUST NOT 再尾插「Copied last assistant message」。无内容 / 失败仍滚动提示。
- 导航瞬时 MUST 回滚动提示尾插：`switched →` / `new session →` / `cloned →` / `imported →` / `restored →`。MUST NOT 再进通知栈或通知条。`forked →` / `history @` 保持尾插。busy 拒切仍走 `Error: ` 通知条（r1189）。
- 栈仍不计入下缘 footprint。
- 单测 / BDD：栈要带「内容高于视口 + 未停在内容顶」；导航 note 要断言 entries 末尾、栈里没有。改写 resume-render「不得进 transcript」断言。

## 非目标

- 不改通知条落点 / Copied 文案 / TTL / 最多 3。
- 不改 xylitol-tui content-end / follow-bottom。视口叠层只动裁切后的可见行，不改滚动语义。
- 不在本票做栈内键盘或展开侧板。
- 不改 LINK_DOWN / LINK_RECOVERED、busy `/reload` 等 **E 运行态** 通知条。

## Capabilities

- `app-tui-fixed-zone`（r1930 视口钉住；栈 MUST NOT 装导航瞬时）
- `app-tui-host`（r1261 Copied；**r1953** 导航尾插）
- `app-tui-session-tree`（r1330 已假设 session 切换有尾插，本票把代码拉回）
- `app-tui-commands`（r1193 `/history-copy-last` 成功改栈）
- `package-tui-engine`（**r1954** 裁切后视口叠层）

## Impact

- 长对话滚到底再复制：✓ Copied 仍在屏幕右上。
- 切 session：`switched →` 跟底贴输入框上方，不 TTL，不进右上角。
- `/history-copy-last` 成功：右上角 `✓ Copied Last Msg`，不再往 transcript 堆一行。
- 通知栈 C′：Copied、Copied Last Msg、首次 Heuristic。

## Open Questions

- **[已定]** 视口顶 = transcript 窗当前可见最顶 1–3 行的右边格子（盖住当时在那儿的正文）；不盖 dock。
- **[已定]** 切 session 一家回滚动提示尾插，本票做（含 new/clone/import/CLI restore）。
- **[已定]** `/history-copy-last` 成功进通知栈，文案 `Copied Last Msg`（不是鼠标那条 `Copied`）；失败仍滚动提示。
- **[已定]** 引擎 **视口叠层**（裁切之后，ingest 与 `reproject_frame` 都走）：先 `composite_overlays`（屏格锚点），再默认空的 `Component::overlay_viewport(&self, lines, width, transcript_rows)`。产品只在 transcript 行画栈，不盖 dock。库仍只发 copy-notice 信号。调研：`research/viewport-overlay.md`。

## Further Notes

- 视口叠层：`research/viewport-overlay.md`。现状 overlay 在 `UiRoot::render`；AO 裁切在 `application_owned_runtime.rs`。
- 导航误入栈：`src/app/tui/host/session_ops.rs` `apply_switched_session` → `push_toast_info_notice`。树 fork 仍 `push_scroll_notice("forked → …")`。
- 主机泵 BDD：`render_frame` → `UiRoot::render`（无 AO 裁切）。
- 词汇：B 导航瞬时 vs C′ 通知栈；勿称「系统消息」。
