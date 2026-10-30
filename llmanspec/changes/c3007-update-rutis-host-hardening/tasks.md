# Tasks

- [x] 1.1 kernel 错误出口：新增 `rutis_error_sink()`（`log::error!(target: "xylitol::rutis", "{error}")`），`boot_ctx` 改用 `Ctx::root_with_sink(Handle::current(), sink)`；`Handle::try_current()` 失败映射为 `BootstrapError::KernelFailed`
- [x] 1.2 单测：进程内 `log::Log` 捕获器断言 sink 写入 `xylitol::rutis`；用 `#[serial(log_global)]` 或独立测试进程避免污染
- [x] 1.3 kernel 启动诊断：`wait_active(ctx, loader)`；超时用 `loader.entries()` + `loader.waits(&entry)` 输出缺失服务与原因，无 waits 时回落 state 列表
- [x] 1.4 纯函数测试：`describe_wait` / `describe_waits` 覆盖 `WaitReason` 代表变体与 wildcard
- [x] 1.5 风格收口：`ctx.plugin(p)` 返回的 `FiberView` 直接 `.await`，去掉 `(&view).await`
- [x] 2.1 门禁：`just qa`
- [x] 2.2 门禁：`just test-tui-e2e`（kernel / runtime 改动）
- [x] 2.3 收口校验：`llman-sdd validate --all --strict`
