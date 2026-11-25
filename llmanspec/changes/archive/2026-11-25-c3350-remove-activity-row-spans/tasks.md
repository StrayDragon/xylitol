# Tasks: c3350-remove-activity-row-spans

## 1. 删掉只写不读的行区间

- [x] 1.1 删除 `SegmentRowSpans`、`ActivityFoldState.row_spans`、`live.rs` 里的 `insert`，以及 `scrollback/mod.rs` 里的 `clear`。折叠级别、命中表和展开状态保留。

## 2. 现有折叠与滚动区测试

seam：`xylitol-app-tui` 里已有的折叠与滚动区测试。不新增 BDD。

- [x] 2.1 `cargo test -p xylitol-app-tui -- activity_fold scrollback` 保持通过。

## 3. 临时评判（deslop-please）

不进 `just qa`。基线是 `.tmp/deslop-please.allow`。测试路径诊断见 `.tmp/deslop-please-deferred-tests.md`，本 change 不改。全仓基线只由不带 crate 参数的 `deslop-please --update-allow .tmp/deslop-please.allow` 重写。只检查一个 crate 时，其它 crate 的条目不在这次输出里，它们仍留在基线中。

- [x] 3.1 在本 change 分支上跑 `deslop-please xylitol-app-tui`。`ActivityFoldState::row_spans` 从报告里消失。`ActivityFoldState::settings` 的泄漏留在基线里。
- [x] 3.2 `deslop-please xylitol-app-tui --allow .tmp/deslop-please.allow` 没有基线以外的新诊断。
