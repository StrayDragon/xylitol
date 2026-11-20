# Tasks

单 task 上限约 2 小时；全部为纯删除与签名收敛，无行为变更。

## T1 删 Resp 列

- [ ] `crates/xylitol-protocol/src/registry.rs`：删 `enum Resp`（:18-22）、`MethodEntry.resp`（:36）、`m()` 的 `resp` 实参（:45）、`mx()` 的 `resp` 实参（:62）。
- [ ] `REGISTRY` 50 行逐行去掉 `Resp::*` 实参（:130-489）。
- [ ] `crates/xylitol-protocol/src/lib.rs:21`：从 `pub use registry::{...}` 去掉 `Resp`。
- 验证：`cargo check -p xylitol-protocol` 无错；`rg 'Resp' crates/ tests/ --glob '!target'` 仅剩无关命中。

## T2 删 Idem 列

- [ ] `registry.rs`：删 `enum Idem`（:12-15）、`MethodEntry.idem`（:35）、`m()` / `mx()` 的 `idem` 实参。
- [ ] `REGISTRY` 50 行逐行去掉 `Idem::*` 实参。
- [ ] `lib.rs:21`：去掉 `Idem` 导出。
- 验证：`cargo check -p xylitol-protocol` 无错；`rg 'Idem' crates/ tests/ --glob '!target'` 无命中。

## T3 收敛构造器签名并删 names()

- [ ] `m(name, auth, command_backed)` 与 `mx(name, auth, command_backed, exec)` 定稿；`MethodEntry` 字段顺序同步为四列。
- [ ] 删 `names()`（:495-497）与 `lib.rs:21` 的 `names` 导出。
- 验证：`cargo check -p xylitol-protocol` 无错；`rg 'registry::names|::names\(' crates/ tests/ --glob '!target'` 无命中。

## T4 清理守卫测试

- [ ] `registry.rs` 的 `writer_set_and_bypass_set_match_declaration`：删 `bypass` 断言段（:663-677），保留 `writers` 断言段。
- [ ] 若测试名不再贴切，改为反映「Writer 集合锁定」的语义名（中文注释同步）。
- 验证：`cargo nextest run -p xylitol-protocol` 全绿。

## T5 全树回归

- [ ] `cargo nextest run -p xylitol-protocol -p xylitol-serve -p xylitol-app-tui -p xylitol-kernel`。
- [ ] 若 `tests/snapshots/*agent_session_api_baseline*` 因公开面变化而 diff，核对差异只含被删的 `Resp` / `Idem` / `names`，然后更新 snapshot。
- [ ] `cargo clippy -p xylitol-protocol --all-targets -- -D warnings`。
- [ ] `cargo fmt --check`。
- 验证：以上命令全绿；`llman-sdd validate --all` failed 0。
