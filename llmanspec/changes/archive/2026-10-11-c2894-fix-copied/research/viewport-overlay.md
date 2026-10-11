# 视口叠层：裁切之后，ingest 与再投影同一条缝

## 问题

通知栈若写进 `Component::render` 的内容缓冲，会被 `ScrollView` 裁掉；滚轮只走 `reproject_frame`，连 `render` 都不调。这不是 Copied 特例，是 **钉在视口上的装饰** 整类的坑。

## 引擎里已经有的同类东西

| 缝 | 何时 | 坐标 |
|---|---|---|
| `dock_rows_hint` | `render` 之后、`project_frame` 之前 | 把尾部行划成 dock |
| `offer_pointer` / `set_screen_origin` | dispatch | 屏格；默认空操作 |
| `paint_visible` 里选区反色、悬停 tint | **裁切之后** | 可见 transcript 行 |
| `composite_overlays` | **裁切之前**（`do_render` 先 overlay 再 `project_frame`） | 用终端高宽算 TopRight，却 blit 到未裁切长缓冲 |

选区 / 悬停已经叠在可见视口上。通知栈应跟它们一伙，不跟 `render()` 的内容行一伙。

`OverlayAnchor::TopRight` 看起来能钉视口，但定位用 `term_height`、写入却是裁切前的 `lines`：长 session 时 TopRight 落在内容行 0，照样滚走。产品面板走 EditorSlot，没用 `show_overlay`；包测 overlay 多在 Inline（无 project）。AO + 库 overlay 在滚轮路径上根本不会再 composite。

ptim15 / r1640：库 **MAY 只发 `take_copy_notice` 信号、不画文案**；若库自己画，SHOULD 在 dock。产品 r1930 已定为视口右上角栈。继续：**库只发信号，产品在视口叠层里画栈**。不把文案塞进引擎。

词：引擎缝叫 **视口叠层**（叠在已裁切视口上）。产品槽仍叫 **通知栈**。不要用「屏幕遍」。

## 选定：视口叠层

`do_render` 在可见帧拼好之后（AO：`project_frame` **或** `reproject_frame`；Inline：组件 render 原样）做同一道视口叠层：

```text
ingest:    Component::render → project_frame? → 视口叠层
滚轮:      reproject_frame（不 render）      → 视口叠层
Inline:    Component::render                  → 视口叠层
```

顺序：

1. `composite_overlays`（modal 用屏格锚点；AO 滚轮也要跑，否则 overlay 从缓存里消失）
2. 对每个根 `Component` 调默认空的 `overlay_viewport(&self, lines, width, transcript_rows)`（产品画 Copied / Copied Last Msg / Heuristic；只动 `lines[0..transcript_rows]`，不盖 dock）

`&self`：TTL 用 `Instant::now()` 只读，再投影不必 `&mut`。先例：`dock_rows_hint` / `offer_pointer` 默认空。

后人加视口 HUD / tab 角标：实现 `overlay_viewport`，不要写进 scrollback，不要为装饰把 `ao_components_stale` 拉起来。

## 不选

| 方案 | 为何不如 |
|---|---|
| 只在 `render()` 按 `scroll_top` 写内容行 | 滚轮不 render |
| 每次滚轮整棵 stale 重绘 | 与 AO 再投影性能目标对着干 |
| 把栈做成 `show_overlay(TopRight)` 而不把 composite 挪到裁切后 | 同一坑 |
| 引擎内置 Copied 文案 | 产品词表（Copied Last Msg、Heuristic）会污染 xylitol-tui |
