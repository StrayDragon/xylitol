# Tasks

测试缝：`UiRoot` 可见视口（`src/app/tui/layout/root`，经 AO 裁切或等价钩子，不单测未裁切 `render()[0]`）；滚轮再投影（`packages/xylitol-tui` `reproject_frame` / 产品叠层钩子）；主机泵 BDD（`tests/bdd/steps_app_tui_host.rs` / `bindings_app_tui_fixed_zone_c2826.rs`）。不新开 harness。不测真 TTY。

## T1 — 通知栈改叠已裁切视口

- [ ] `overlay_client_notices` 不再写入 ScrollView 内容缓冲第 0 行。每一帧（含 `project_frame` 与只投影的 `reproject_frame`）把未过期通知叠在可见 transcript 顶行右上。
- [ ] 仍不计入 `last_dock_rows` / Fixed-Zone Footprint；仍非 `Error: ` 通知条。
- [ ] 校验：内容高于视口且 `scroll_top > 0` 时，可见第一行含 `✓ Copied`；内容第 0 行不再是唯一/错误落点。

## T2 — [blocked-by: T1] 滚轮再投影仍钉住

- [ ] 模拟：先 ingest 一帧（臂装 Copied），再只 `reproject_frame` 且 `scroll_top > 0`，不重绘 `UiRoot::render`。
- [ ] 校验：可见顶行仍有 Copied。

## T3 — [blocked-by: T1] 门禁与 BDD

- [ ] 改写 `client_notice_stack_right_aligns_copied_not_error_bar`（及同类只看未裁切 `render()[0]` 的断言）到可见视口。
- [ ] 绑定 r1930 新场景 `notice-stack-copied-pins-scrolled-viewport`；主机泵 `render_frame` 若仍直调 `UiRoot::render`，改为已裁切视口。
- [ ] 校验：`cargo test --all-features --test bdd` 在本 change 分支相对 merge-base 绿（本票相关场景）。
