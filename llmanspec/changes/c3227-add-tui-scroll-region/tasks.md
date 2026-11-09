# Tasks — TUI 有界滚动区域与 MCP 详情接线

- [ ] 1. 引擎 `ScrollRegion` 渲染核心：`set_content` / `set_height` / 按宽换行 / 定高裁剪 / 不足补空 / overflow gutter / width 0 安全；组件单测 + 包快照。seam：`crates/xylitol-tui/src/components/scroll_region.rs` 单测、`tests/snapshot_test.rs`。
- [ ] 2. 引擎 `ScrollRegion` 输入与指针：PageUp/PageDown（`app.slot.pageUp` / `app.slot.pageDown`）、滚轮、gutter 顶部/底部点击、`offer_pointer` 矩形命中、clamp；按键→状态测试。seam：`crates/xylitol-tui/tests/harness_test.rs`。 [blocked-by: 1]
- [ ] 3. app-tui `McpSlot` 详情视口：摘要 + 列表 + `ScrollRegion` 详情 + footer；`detail_rows` 固定上限；`panel_text` 保留全量；长详情 80/100 列 harness。seam：`crates/xylitol-app-tui/src/harness.rs`。 [blocked-by: 1]
- [ ] 4. app-tui footprint 与交互接线：`apply_slot_footprint` 计入详情视口高度；列表 `max_visible` 取剩余预算；`app.slot.pageUp/pageDown` 注册；`UiRoot` 指针路由到详情视口；滚轮/gutter harness。 [blocked-by: 2, 3]
- [ ] 5. 回归与规格对齐：短终端 status/editor 可见；窄宽详情可滚动；无 render overflow；包/根 specs 与实现一致；`llman-sdd validate c3227-add-tui-scroll-region` + 包 `llman-sdd validate --check --directory crates/xylitol-tui`。seam：app harness、`tests/tui_e2e/pty.rs`（可选）。 [blocked-by: 3, 4]
- [ ] 6. 门禁：`just fmt`、`just lint`、`just test-tui`、`just qa`；涉及真终端再加 `just qa-e2e`。 [blocked-by: 5]
