# M0 spike：rutis 0.13.0 + loader

日期：2026-10-20
探针：`scripts/lab_rutis_kernel_spike`（非 workspace 成员；`cargo run --manifest-path scripts/lab_rutis_kernel_spike/Cargo.toml`）
上游：crates.io `rutis =0.13.0` / `rutis-loader =0.13.0`
工具链：仓库 rust-toolchain.toml（edition 2024 nightly + mold/sccache）

## 结论

主线 **GO**。会话实例必须用 `TypeKey::instance`，不能对同一类型做两次全局 `provide`。外进程瀑布未跑（不阻塞）。

## 分项

| 项 | 结果 | 数字 / 备注 |
|---|---|---|
| edition 2024 编译 + `Ctx::root` | GO | diagnostics.plugins=1 |
| 动态 session instanced group | GO | A/B 均为 Active 且 instance id 不同。**条件**：`register_with` + `TypeKey::instance::<T>(build.instance("session"))`。第一次用全局 `provide` 时第二份实例 `Active=false`（`ServiceExists`）。 |
| `watch` + 首轮超时 | GO | SlowMcp 80ms apply；15ms `wait_for(Active\|Failed)` 超时，elapsed≈16ms，门可放行。 |
| `tool/pre-execute` waterfall | GO | 1 监听 2000 次，**2.61 µs/op**（预算 < 5 µs）。 |
| `shutdown_with_timeout` | GO | 200ms AsyncDisposer；40ms 等待得 `DisposeWaitError::TimedOut`；随后 `shutdown()` join 剩余 ≈160ms。合作取消的 apply（`cancelled().await`）会立刻成功，测不到超时——abort 映射要靠不让出的 disposer/任务。 |
| `Chain` + admit 白名单 | GO | 核心名 `Resolve { blocked by admit }`；官方 Builtins 命中；第二环解析 `third-party`。 |
| 外进程瀑布 | 记录 | 未执行。不启用 `rutis-loader` 的 python/node（会拉 `rutis-bridge`）。契约是事件 `NAME` + serde；不阻塞主线。 |

## 对施工的约束

1. 会话私有服务一律 instance 键；全局名空间只放真全局。
2. MCP 首轮门用 `FiberView::watch` + `tokio::time::timeout`，不要 `injects` 硬等。
3. 热路径瀑布监听器 ≤1–3；实测 1 监听已占 ~2.6 µs，余量不大。
4. abort：对不合作的清理走 `shutdown_with_timeout`；合作路径听 `ctx.cancelled()`。
5. admit 必须对核心名返回非 `NotFound`（否则 Chain 会落到下一环）。
