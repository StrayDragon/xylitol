# 设计：通知栈叠在已裁切视口

## 绘制顺序（现状）

```text
UiRoot::render
  upper = 启动卡 + 全文 scrollback     ← 内容缓冲
  dock  = queue + 待办栏 + 通知条 + status + editor + footer
  overlay_client_notices(lines[0..])   ← 写进内容第 0 行
Tui::render
  Component::render 整段
  runtime.project_frame：去掉 dock，内容进 ScrollView，visible = content[scroll_top..]
滚轮
  reproject_frame：只切 ScrollView，不再 render 组件
```

用户看见的「视口顶」是 `visible[0]`，不是 `content[0]`。

## 选定做法

通知栈是**屏幕空间**装饰，每一帧（含滚轮再投影）叠在已裁切的 transcript 可见行上，不进 ScrollView 内容。

落点：`paint_visible` 得到 `visible`（+ dock）之后、终态宽度归一化之前。产品侧提供「把当前未过期通知画进这一段可见行」的入口；引擎在 `project_frame` / `reproject_frame` 之后调用，避免滚轮跳过 `Component::render`。

不要：

- 只在 `UiRoot::render` 里按 `fold_hits.scroll_top` 写 `lines[scroll_top]`——滚轮帧不走 render，Copied 仍粘在旧内容行。
- 为了挪 Copied 而在每次滚轮把 `ao_components_stale` 拉起来重绘整棵 upper。
- 把通知栈改成 xylitol-tui 的 modal overlay 槽（语义不同，且会碰包级 overlay 合约）。

引擎若需调用产品绘制，用 `Component` 上默认空的视口叠层钩子（产品 `UiRoot` 实现）。不改 content-end / follow-bottom。

## 测试

| 缝 | 要覆盖 |
|---|---|
| `UiRoot` 单测 | 内容明显高于视口 + `scroll_top > 0`：可见第一行有 `✓ Copied`，内容缓冲第 0 行（启动卡/第一条 user）不必再当唯一落点 |
| AO `paint_visible` / 产品钩子 | 先 ingest 再只 `reproject_frame`（模拟滚轮）：Copied 仍在可见顶 |
| 主机泵 BDD | 不得只靠未裁切的 `UiRoot::render`；新场景走已裁切视口 |

现有 `client_notice_stack_right_aligns_copied_not_error_bar` 断言 `render()[0]` 是假绿，必须改。

## 明确不做

- 改 TTL / 最多 3 / 文案 / 通知条
- 引擎级 bottom-fixed-zone pin
- 真 PTY
