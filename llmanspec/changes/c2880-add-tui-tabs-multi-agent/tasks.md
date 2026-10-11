# Tasks

测试缝：`Tabs::handle_input` + `Tabs::render` / `title_col_ranges`。crate 单测在 `packages/xylitol-tui`。BDD 复用 `tests/bdd` 的 `bindings_package_tui_*` 模式。不新开 harness。不测产品 host。

## T1 — Tabs 组件与键

- [ ] `packages/xylitol-tui/src/components/tabs.rs`：`new` / `set_titles` / `set_selected` / `with_pages` / `with_theme` / `with_on_change`。
- [ ] `create_default_definitions` 加 `tui.tabs.prev`（left）、`tui.tabs.next`（right）。
- [ ] `lib.rs` 再导出 `Tabs` / `TabsTheme`。
- [ ] `handle_input`：左右循环、`1`–`9`、空表与越界 digit 忽略。
- [ ] `render`：一行标题条 + 当前 page；未选 page 不 `render`。
- [ ] `title_col_ranges` 在 `render(width)` 后可查。
- [ ] 校验：crate 单测覆盖循环、digit、空表、未选 page 不出现、ranges 不重叠且落在 width 内。

## T2 — [blocked-by: T1] BDD

- [ ] 新建 `packages/xylitol-tui/llmanspec/specs/package-tui-tabs/package-tui-tabs.feature` 的场景绑到 `tests/bdd/bindings_package_tui_tabs.rs` + `tests/bdd/steps_package_tui_tabs.rs`，接入 `tests/bdd/suite.rs`。
- [ ] 校验：`cargo test --all-features --test bdd` 在本 change 分支相对 merge-base 绿（仅本 capability）。

## T3 — [blocked-by: T1] 包门禁与文档一行

- [ ] `just test-tui`（或 `cargo test -p xylitol-tui`）绿。
- [ ] 公开 API 变化时：`packages/xylitol-tui/AGENTS.md` 组件列表补 `Tabs` 一行（不写产品 Down / 子 agent）。

不在本任务列 finalize。
