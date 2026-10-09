# Tasks

## T1 — 应答形状迁移（session_tree → RawOk）

- [done] `wire_v3::typed_payload` 移除 `METHOD_SESSION_TREE` 强 schema 臂（走 RawOk，JSON 轨同构）+ 文档注释。
- [done] `src/main.rs` 回退 64MB worker 栈补丁 → `Runtime::new()`（默认栈）。
- [done] wire_v3 单测同步迁移（RawOk 断言 + 兼容解码面往返）。
- 校验：`cargo test -p xylitol --lib app::server::wire_v3` 绿。

## T2 — spec + BDD 守卫

- [done] `server-core` r1921（session_tree 应答透明与深度安全）+ 场景 `v3-session-tree-raw-parity`。
- [done] BDD：双轨取树对拍（v3 应答变体断言 RawOk + 两轨等价 + tree 形状）；绑定。
- 校验：`cargo test --all-features --test bdd` 并行 956/956 绿；`llman-sdd validate c2845 --strict` 绿。

## T3 — 门禁与收口

- [done] lib 1618 绿；clippy 无新增；fmt/anchors/check-scripts 绿；实机默认栈深树 /session-tree 无崩溃。
- 收口：`llman-sdd change finalize c2845-fix-deep-session-tree-recursion --into 当前分支`（不带 --no-check）+ push。
