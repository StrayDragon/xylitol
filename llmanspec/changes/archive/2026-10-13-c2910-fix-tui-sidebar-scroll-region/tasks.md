# Tasks

## T1 包层边界口径

- [x] `ApplicationOwnedRuntime.extra_reserved_cols` + setter；rect / 滚轮 / gutter stamp·hit·drag 统一 `content_right`
- [x] 单测 `extra_reserved_owns_sidebar_band`（带内不消费、内容照常、边界公式）

## T2 宿主对账

- [x] 根 `SESSION_SIDEBAR_W` / `session_sidebar_reserved_cols`；drain 每轮 `sync_session_sidebar_reserve`；`Tui` 门面 + begin 重放

## T3 合约与验证

- [x] r2904 规则补滚动分区与 gutter 贴靠两句
- [x] lib / bdd / package 全绿；clippy 0；`llman-sdd validate --strict` 绿
