# Design — TUI 有界滚动区域与 MCP 详情接线

## 1. 引擎组件形状

- 新增 `ScrollRegion`（`crates/xylitol-tui/src/components/scroll_region.rs`），包一个 `ScrollView`。
- 公开构造：`ScrollRegion::new(height, KeybindingIds { page_up, page_down })`；`KeybindingIds` 由应用端传入，引擎不硬编码产品动作 id。
- 内容：`set_content(Vec<String>)`；`set_height(usize)`；`height()`；`content_rows()`（按上次渲染宽度换行后的行数）；`scroll_by` / `scroll_to_top` / `scroll_to_bottom`。
- `Component` 实现：`render(width)` 输出恰好 `height` 行；`handle_input` 只处理 PageUp/PageDown 键；`offer_pointer` 处理滚轮与 gutter 点击；`set_screen_origin` 记录屏幕原点。
- 不新增第二套滚动状态；`ScrollView` 仍是滚动位置/视口高度的唯一模型。

## 2. 渲染与 gutter

- 内容行按 `wrap_text_with_ansi(line, content_width)` 换行；`content_width = width`，overflow 且宽度 ≥ `GUTTER_MIN_WIDTH` 时预留最右 1 列给 gutter。
- 视口显示 `scroll_top..scroll_top+height` 的换行后行；不足 `height` 用空行补齐；超出 `height` 由 gutter 表达。
- gutter 复用 `scroll_gutter` 的 `stamp_gutter` 比例算法；顶部/底部按钮行仍为 `^` / `v`。
- 不为槽 gutter 复用 transcript 的全局 `GUTTER_TOP_HIT_ID` / `GUTTER_BOTTOM_HIT_ID`；`ScrollRegion` 用自身矩形做直接命中，避免与 transcript gutter 冲突。
- width 0 返回空 `Vec`；width 1 安全；不得 panic。

## 3. 输入与指针

- 键盘：`ScrollRegion` 匹配应用传入的 `app.slot.pageUp` / `app.slot.pageDown`，按 `height` 步长滚动；clamp 到 `0..=max_scroll`。
- 滚轮：`offer_pointer` 收到 `ScrollUp` / `ScrollDown` 且指针在视口矩形内时滚动 `ScrollView::wheel_notch()`，返回 `true`；矩形外返回 `false`。
- gutter：`offer_pointer` 收到 `Left Down` 且列 = 视口最右列、行 = 顶/底行时，等价 `scroll_to_top` / `scroll_to_bottom`，返回 `true`；轨道行不产生动作。
- 不启动 transcript 拖选；不认领视口外事件。
- 应用端在 `UiRoot::handle_pointer_input` / `handle_overlay_pointer` 中把鼠标事件转给当前槽的详情视口；命中按屏上实际矩形，不按冻结坐标。

## 4. McpSlot 布局与 footprint

- `McpSlot` 增加 `detail: Option<ScrollRegion>` 与 `detail_rows: usize`。
- `detail_rows = min(详情逻辑行数, MAX_DETAIL_ROWS=4)`；无详情时为 0。
- `apply_slot_footprint`：`mcp_slot.trailer = MCP_SLOT_BASE.trailer + detail_rows`；`mcp.set_max_visible(slot_body_budget(...))`。
- `set_max_visible` 只设置列表 `max_visible`；详情视口高度由 `detail_rows` 固定。
- `render`：摘要 1 行 + 列表 `max_visible` 行 + 详情视口 `detail_rows` 行 + footer 1 行；总数 = `term_rows - reserved_lower`。
- 短终端：`slot_body_budget` 已扣 reserved；列表与详情视口仍至少保留 1 行；status/editor 不被挤出。
- `panel_text` 保留全部详情行供 harness 断言；渲染只显示视口窗口。

## 5. 键位

- `app.slot.pageUp` / `app.slot.pageDown` 加入 app keybinding catalog，默认 `pageUp` / `pageDown`。
- 仅当当前槽有详情视口时响应；无详情时不吞键，列表仍走既有 `tui.select.*`。
- 不改 `tui.select.pageUp/pageDown` 的引擎默认空和弦；不改列表选择语义。

## 6. Specs 与测试 seam

- 包 spec：`crates/xylitol-tui/llmanspec/specs/package-tui-scroll-region/package-tui-scroll-region.feature`。
- 根 specs：`app-tui-fixed-zone`、`app-tui-commands`、`app-tui-mouse-actions`、`app-tui-host`。
- 引擎 seam：`crates/xylitol-tui/tests/harness_test.rs`（按键→状态）、`tests/snapshot_test.rs`（渲染）、组件内单测。
- 应用 seam：`crates/xylitol-app-tui/src/harness.rs`（80/100 列长详情、PageDown、滚轮、gutter、footprint）、`src/tests.rs`。
- 真终端：`tests/tui_e2e/pty.rs` 产品 smoke（可选，按 `just qa-e2e`）。
- 门禁：根 `llman-sdd validate c3227-add-tui-scroll-region`；包 `llman-sdd validate --check --directory crates/xylitol-tui`。

## 7. 边界与性能

- 不引入引擎级 bottom-fixed-zone pin；Fixed-Zone Footprint 仍由 app-tui 持有。
- 不改 transcript content-end 视口、selection、gutter 语义。
- 不与 c3226 合并；`ScrollRegion` 是组件层，`PanelRegion` 是放置层，二者共用 `ScrollView`。
- 滚动重绘只影响槽矩形；不得触发 transcript 重排或整段 stale。
- 本 change 只接 `/mcp`；其它槽后续复用同一组件。
