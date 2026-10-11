# Tasks

测试缝：`Tabs::handle_input` / `offer_pointer` + `render` / 两类区间；`TUI::dispatch_event` 的指针投递。crate 单测在 `packages/xylitol-tui`。BDD 复用 `tests/bdd` 的 `bindings_package_tui_*`。不新开 harness。不测产品 host。

## T1 — Tabs 标题条与键

- [x] `packages/xylitol-tui/src/components/tabs.rs`：`new` / `set_titles` / `set_selected` / `with_theme` / `with_on_change`。无 `with_pages`。
- [x] `create_default_definitions` 加 `tui.tabs.prev` / `tui.tabs.next`，默认键列表为空。
- [x] `lib.rs` 再导出 `Tabs` / `TabsTheme`。
- [x] `set_screen_origin` / `offer_pointer`：屏上坐标减原点；行不在条上返回 false。
- [x] `handle_input`：已绑定的上一 / 下一标题键切下标；指针命中则改下标；空表与未命中忽略。
- [x] `Component` 默认空操作的 `set_screen_origin` / `offer_pointer`；`Tabs` 覆盖。`input_wants_rerender` 在改了下标的指针上为 true。
- [x] `render`：只输出一行标题条。
- [x] `title_col_ranges` / `overflow_col_ranges` 在 `render(width)` 后可查。
- [x] 校验：crate 单测覆盖键循环、未绑定忽略、点标题、点标记、条上滚轮、条外指针不改下标、超宽窗口与两类区间。

## T1b — [blocked-by: T1] 引擎投递

- [x] `TUI::dispatch_event` 在 listener 之后、焦点之前对根 child / 可见 overlay 调 `offer_pointer`；true 则不再送给焦点组件。
- [x] `Container::offer_pointer` 按上次 render 行偏移转给 child。`handle_input` 仍不转发键盘。
- [x] `packages/xylitol-tui/AGENTS.md` host checklist 补：布局后 `set_screen_origin`；走 `dispatch_event` 即可。底层区间与 `set_selected` 仍可用。
- [x] 校验：crate 单测「根 Tabs 认领指针时焦点 editor 收不到」。

## T2 — [blocked-by: T1, T1b] BDD

- [x] `package-tui-tabs` 场景绑到 `tests/bdd/bindings_package_tui_tabs.rs` + `tests/bdd/steps_package_tui_tabs.rs`，接入 `tests/bdd/suite.rs`。
- [x] 校验：`cargo test --all-features --test bdd` 在本 change 分支相对 merge-base 绿（仅本 capability）。

## T3 — [blocked-by: T1] 包门禁与文档一行

- [x] `just test-tui`（或 `cargo test -p xylitol-tui`）绿。
- [x] 公开 API 变化时：`packages/xylitol-tui/AGENTS.md` 组件列表补 `Tabs` 一行（不写产品 Down / 子 agent）。

不在本任务列 finalize。
