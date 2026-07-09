# c420 Tasks — autocomplete debounce 补齐

> 补齐 async+CancellationToken+fd递归+debounce，~350 行新代码 + ~15 测试。c405 第 1+3+4 层验证。

## 阶段 1：autocomplete_fd.rs — fd 子进程递归搜索（1.5h）

- [x] 1.1 新建 `packages/xylitol-tui/src/autocomplete_fd.rs`
- [x] 1.2 实现 `build_fd_path_query(query: &str) -> String`：对齐 pi 的 `buildFdPathQuery`（规范化路径、构建 `--full-path` 正则）
- [x] 1.3 实现 `walk_directory_with_fd(...)`：spawn fd 子进程，参数对齐 pi，CancellationToken 取消
- [x] 1.4 `cargo check -p xylitol-tui` 通过

## 阶段 2：autocomplete.rs — async 方法 + DebouncedAutocomplete wrapper（2h）

- [x] 2.1 `CombinedAutocompleteProvider` 增加 `fd_path: Option<String>` 字段和 `new_with_fd()` 构造器
- [x] 2.2 增加 `async get_suggestions_async(...)` 方法：接受 `CancellationToken`，逻辑对齐 `get_suggestions` 但 async
- [x] 2.3 `get_fuzzy_file_suggestions_async`：有 fd_path 时调用 `walk_directory_with_fd`，否则 fallback 到现有 read_dir
- [x] 2.4 新增 `DebouncedAutocomplete` struct：`new(provider, delay)` / `get_suggestions(...)`（async，内部用 `tokio::time::sleep` + `CancellationToken`）
- [x] 2.5 `DebouncedAutocomplete::get_suggestions`：新调用到达时 cancel 上一个 token，创建新 token，sleep delay 后调用 `provider.get_suggestions_async`
- [x] 2.6 `lib.rs` 导出 `walk_directory_with_fd`、`build_fd_path_query`、`DebouncedAutocomplete`
- [x] 2.7 `cargo check -p xylitol-tui` 通过

## 阶段 3：测试（第 1+3+4 层，2h）

- [x] 3.1 新建 `packages/xylitol-tui/tests/autocomplete_test.rs`
- [x] 3.2 ac02 测试：`fd_recursive_search_finds_nested_files`（tempdir + 嵌套文件，验证 fd 输出解析）
- [x] 3.3 ac02 测试：`fd_cancellation_kills_subprocess`
- [x] 3.4 ac02 测试：`fd_path_none_fallback`（fd_path=None 时回退 read_dir）
- [x] 3.5 ac02 测试：`build_fd_path_query_handles_path_separator`（含 `/` 的查询 → `--full-path`）
- [x] 3.6 ac03 测试：`debounce_drops_intermediate_calls`（`#[tokio::test(start_paused)]`，3 次调用只触发最后一次）
- [x] 3.7 ac03 测试：`debounce_cancellation_stops_in_flight`
- [x] 3.8 ac03 测试：`debounce_in_test`（advance 250ms 后 query 触发）
- [x] 3.9 ac03 测试：`debounce_no_delay_under_window`（第一次调用后 249ms 新调用，旧被取消）
- [x] 3.10 ac01 测试：`async_get_suggestions_respects_cancellation`
- [x] 3.11 ac01 测试：`get_suggestions_sync_still_works`（回归现有同步路径）
- [x] 3.12 第 4 层（proptest）：`fd_results_subset_of_read_dir`（fd 返回的结果是 read_dir 递归超级的子集）

## 阶段 4：验证（0.5h）

- [x] 4.1 `cargo test -p xylitol-tui` 全量通过（包括现有 207 测试）
- [x] 4.2 `cargo clippy -p xylitol-tui --all-targets -- -D warnings` clean
- [x] 4.3 `llman sdd validate c420-port-autocomplete-debounce --strict` 通过
- [x] 4.4 更新 `_HANDOFF.md` 已知差距表（autocomplete 标完成或更新缺口百分比）

## 校验命令

```bash
cargo test -p xylitol-tui
cargo clippy -p xylitol-tui --all-targets -- -D warnings
llman sdd validate c420-port-autocomplete-debounce --strict
```
