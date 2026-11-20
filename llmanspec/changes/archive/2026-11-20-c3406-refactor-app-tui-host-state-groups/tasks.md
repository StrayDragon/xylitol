# Tasks

单 task 上限约 2 小时；逐组一提交（proposal What Changes）。全部无行为变更；顺序按引用量升序（stage-name → stream-paint → footer-token）。

## T1 stage-name 组类型化（引用最少 14 处，最小风险先行）

- [x] host/mod.rs 新增私有 `StageNameState { policy, attempted, request_pending }` + 组内方法；HostSession :90-92 三字段收为一个组值；:153 / :160 构造处同步初始化。
- [x] 全部 14 处引用迁移为组方法 / 组字段访问；方法归属与调用时序不变。
- 判据：`rg -n 'stage_name_policy|stage_name_attempted|stage_name_request_pending' crates/xylitol-app-tui/src` 仅剩组类型定义 / impl 命中；`cargo nextest run -p xylitol-app-tui` 全绿；单独 commit（`refactor(app-tui): ...`）。

## T2 stream-paint 组类型化

- [x] `StreamPaintState { paint_dirty, stream_sync_pending, suppress_xy_until_stream_end }`（:95 / :99-100，32 处引用）照 T1 同型收编。
- 判据：三字段名平铺 0 命中（组内除外；深挖补注：`tests.rs:3780` 断言消息字符串含 `paint_dirty`，属预期残留、不改测试文本）；`cargo nextest run -p xylitol-app-tui` 全绿（含 snapshot）；单独 commit。

## T3 footer-token 组类型化

- [x] `FooterTokenState` 收 :107-112 六字段（32 处引用）；channel 元组 `(u64, Option<String>, bool)` 收 type alias；tx / rx 所有权与主循环消费位置不变。
- 判据：六字段名平铺 0 命中；`cargo nextest run -p xylitol-app-tui` 全绿；单独 commit。

## T4 全量门禁

- [x] `just fmt` + `just lint`。
- [x] `cargo nextest run -p xylitol-app-tui` 全绿，记录输出。
- [x] `just qa` 全绿，记录输出。
- [x] `llman-sdd validate c3406-refactor-app-tui-host-state-groups` failed 0。
- [x] 复核平铺字段 41 → 32：`sed -n '81,125p' crates/xylitol-app-tui/src/host/mod.rs` 数组段。
