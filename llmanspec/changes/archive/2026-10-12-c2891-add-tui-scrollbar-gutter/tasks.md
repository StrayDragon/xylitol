# Tasks — c2891 右缘滚动槽

前置：design.md 已锁 D1–D5 拍板；本票含产品接线（同票末刀，产品默认开、仅 ApplicationOwned）。

- [x] T1 包层 gutter 配置与 stamp：`ApplicationOwnedRuntime` 增 `set_scroll_gutter(Option<GutterConfig>)`（默认 None）；`paint_visible` 尾部 stamp 右列 `^ │ v`（顶/底行按钮、中轨 capturing 语义字符）；`content_len ≤ viewport_h` 或宽 `< 20` 不 stamp。单测：几何、阈值隐藏、双路（project/reproject）一致。
- [x] T2 carve 与 reproject 一致：`do_render` AO+gutter 时 `render_w = cols−1`；`can_reproject_application_owned` 宽比较统一用 carve 后值；selection transcript rect 同 carve。单测：resize 进出快路径不脏帧、`render_w` 下 wrap 正常。
- [x] T3 包层命中封装：gutter 两个 `Hit::at_cell(..).capturing()`（顶/底行、末列）随帧可注册；提供 host 查询当前应注册的 hit 集（含隐藏时为空）。单测：`hit_at` 于 scroll 前后均命中同 id（Viewport 语义）。
- [x] T4 产品接线：`src/app/tui/widgets/scroll_gutter.rs`（disjoint 常量 id 表）→ `sync_pointer_hits` concat → `take_pointer_click` 分支先 gutter 后 fold → 动作 `scroll_top=0` / `scroll_to_end` + wheel 同族 repaint。host 构造 opt-in 常量默认开。interaction_scene 测试：点 ^ 后 `scroll_top==0`、点 v 后 `at_bottom`、拖选路过 gutter 不产生选区。
- [x] T5 BDD：新 capability feature + `bindings_package_tui_scrollbar_gutter.rs`（照 pointer 模式）；`app-tui-transcript` 增补接线规则场景并接 bindings。含 interaction_scene/host 层点击断言（T4 挪入：click→scroll_top==0、拖选过 gutter 无选区）。
- [x] T6 收口：`just fmt && just lint && just test && just test-tui`（或 `just qa`）；insta 快照若动则 `cargo insta` 流程接受。
