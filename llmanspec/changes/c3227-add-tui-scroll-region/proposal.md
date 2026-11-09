---
depends_on: []
title: "TUI 有界滚动区域与 MCP 详情接线"
needs_specs_change: true
---

# TUI 有界滚动区域与 MCP 详情接线

## Why

固定区 EditorSlot 只能按 `max_visible` 滚列表项。非列表内容（MCP 诊断、resume 状态、tree 详情、capabilities 说明）没有有界滚动区域；长内容只能截断或撑高槽。`/mcp` 详情在普通终端宽度下被 `…` 截断，用户无法完整查看每 server 的 transport / scope / credential / counts / timeout / last error。

引擎已有 `ScrollView`（纯模型）与 `ScrollGutter`（transcript 专用），没有可复用的 `ScrollRegion` 组件。`ExpandableOutput` 只有 max preview，`SelectList` / `TreeSelector` 只滚列表项，`OverlayOptions` 缺有界区域约束。c3226 草案规划 `PanelRegion` / `PanelBounds` / `PanelPlacement` 做侧栏有界面板，但未落地，且不是槽内详情视口。

## What Changes

### 引擎（crates/xylitol-tui）

- 新增 `ScrollRegion` 组件：定高视口，内容按宽度换行后裁剪；内容不足按实际行收口，内容超出显示 gutter。
- 组件包装既有 `ScrollView`，复用 `scroll_gutter` helper；不新增第二套滚动状态机。
- 交互：键盘 PageUp/PageDown、鼠标滚轮、gutter 顶部/底部点击；滚动位置按内容行数 clamp。
- 指针：`offer_pointer` 只认领视口矩形内的滚轮与 gutter 命中；不穿透到 transcript 选区。
- `Component::render` 输出恰好视口高度行；width 为 0 时安全返回。

### 应用端（crates/xylitol-app-tui）

- `McpSlot` 详情区改用 `ScrollRegion`：摘要 + 列表 + 有界详情视口 + footer。
- Fixed-Zone Footprint：详情视口高度计入 `SlotFixedZoneRows` trailer；列表 `max_visible` 取剩余 body 预算；短终端仍保证 status/editor 可见。
- 键位：新增 `app.slot.pageUp` / `app.slot.pageDown`（默认 PageUp/PageDown），只在槽内详情视口响应；不改变 `tui.select.*` 列表选择语义。
- 鼠标：详情视口内滚轮滚动；gutter 顶部/底部可点击；槽外不穿透。
- `/mcp` 详情完整显示：窄宽换行 + 滚动，不再以 `…` 截断作为唯一手段。

### Specs

- 新增包 capability `package-tui-scroll-region`。
- 更新 `app-tui-fixed-zone`：槽内有界详情视口高度/滚动与 footprint。
- 更新 `app-tui-commands`：`/mcp` 诊断详情可滚动完整查看。
- 更新 `app-tui-mouse-actions`：详情视口滚轮/gutter 指针入口。
- 更新 `app-tui-host`：`/mcp` SelectList + 有界详情视口。

## Capabilities

- `package-tui-scroll-region`（新）
- `app-tui-fixed-zone`
- `app-tui-commands`
- `app-tui-mouse-actions`
- `app-tui-host`

## Impact

- 引擎：新增组件、导出、测试；不改既有 `ScrollView` / `ScrollGutter` 语义。
- app-tui：`McpSlot` 布局与 footprint；新增键位；指针命中。
- 其它槽：本 change 不接线；后续可复用。
- c3226：不合并；`ScrollRegion` 是组件层，`PanelRegion` 是放置层，共用 `ScrollView`。

## Scope / Non-goals

- 本 change 只接 `/mcp` 详情；resume / tree / capabilities 等槽留后续。
- 不做 c3226 侧栏有界面板。
- 不改 transcript content-end 视口语义；不引入引擎级 bottom-fixed-zone pin。
- 不做双栏 `SelectList` 详情面板。
- 不改变 `/mcp` 列表选择与 OAuth 授权路径。

## Decisions

- 范围：引擎 `ScrollRegion` + 先接 `/mcp`。
- 交互：键盘 PageUp/PageDown + 滚轮 + gutter 点击。
- 载体：独立 c3227，不并入 c3226。
- 测试 seam：引擎组件单测/快照；app-tui harness（80/100 列长详情、PageDown、滚轮、gutter、footprint）；包实例 validate；根实例 validate；可选 PTY smoke。

## Risks

- footprint 误算导致短终端挤压 status/editor。
- `tui.select.pageUp/pageDown` 默认空和弦；新增 app 键位需文档化。
- 指针命中与 transcript 选区 / 侧栏 gutter 冲突。
- 与 c3226 的 `ScrollView` 共用边界不清，出现两套滚动抽象。
- 滚动重绘可能触发 transcript 重排；需只重绘槽矩形。

## Further Notes

- 本 change 为 propose 阶段；不写应用代码。
- c3226 为独立草案；本 change 不依赖它。
- 测试 seam 复用既有 `TuiTestHarness` / app-tui harness；不新造平行 harness。
