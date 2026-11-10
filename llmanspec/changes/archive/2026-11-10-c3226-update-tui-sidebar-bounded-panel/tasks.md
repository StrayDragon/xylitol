# Tasks — 会话侧栏有界尺寸与固定区避让

- [x] 1. 引擎 `PanelRegion` 核心：`PanelBounds` / `PanelPlacement` / 宽高 clamp / `set_available` / `rect` / `ScrollView` 溢出；组件单测 + 包快照。seam：`crates/xylitol-tui/src/components/panel_region.rs`、`tests/snapshot_test.rs`。
- [x] 2. 引擎 overlay 约束：`OverlayOptions` 增 `max_width` / `min_height` / `clip`；`composite_overlays` 求交 / clamp / `last_screen_rect`；单元与 harness 测试。seam：`crates/xylitol-tui/src/tui/overlay.rs`、`tests/harness_test.rs`。
- [x] 3. app-tui DockRight 布局：transcript 让宽、dock 全宽、面板拼接；移除整高 `overlay_viewport` 侧栏带；`PanelRegion` 接线；80/100 列 harness snapshot。seam：`crates/xylitol-app-tui/src/layout/root/{mod.rs,render.rs}`、`src/harness.rs`。 [blocked-by: 1]
- [x] 4. app-tui 宽度 / 高度状态与持久化：`open_intent` / placement / width / height cap；`sidebar-layout` 全局 key；separator 拖拽 clamp；24/32/40 快捷档；harness。seam：`layout/root`、`storage.rs`。 [blocked-by: 3]
- [x] 5. app-tui FloatRight / Hidden 降级：阈值、overlay clip、`◨` 状态、resize 迁移、`open_intent` 自动回到 DockRight；harness snapshot。seam：`layout/root`、`host/session_ops.rs`。 [blocked-by: 2, 3]
- [x] 6. app-tui 命中 / 滚轮 / 选区：面板矩形命中、面板带内滚轮、transcript 选区边界、固定区不受影响；harness。seam：`layout/root`、`host`、pointer。 [blocked-by: 3, 5]
- [x] 7. Specs 与 BDD：更新 `app-tui-fixed-zone` r2904、`package-tui-engine`、`package-tui-pointer`；补 / 改 BDD 场景与 binding；根 + 包 `llman-sdd validate`。 [blocked-by: 5, 6]
- [x] 8. 回归与门禁：`just fmt`、`just lint`、`just test-tui`、`just qa`；涉及真终端加 `just qa-e2e`。 [blocked-by: 7]

## 风险与 fallback

- 若 FloatRight / Hidden 复杂度超预期，按 proposal fallback 退回“压缩保持驻列”（仅 DockRight）；在 tasks 3/5 范围调整并记录。
- 面板正文点击是否关闭、FloatRight 是否 `non_capturing`、持久化 schema 版本在实现时定稿；不得静默改产品语义。
