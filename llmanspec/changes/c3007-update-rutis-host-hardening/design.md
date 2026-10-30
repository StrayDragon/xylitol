# Design：rutis host 硬化

## 目标

在不改插件 / loader / 事件模型的前提下，把 rutis 0.19 的 host 侧推荐写法落到当前有明确收益的两处：

1. **错误出口**：rutis 异步错误不再直写 stderr，改经 `log` 进入既有 file-only 观测管线。
2. **启动诊断**：自检超时用 `Loader::waits` 说明缺失服务与原因。

## 1. 错误出口

### 现状

`boot_ctx` 使用 `Ctx::root()`，rutis 安装 `host::default_sink()`，异步错误直接写 stderr。`infra-observability` 要求级别日志只写 agent 日志目录，不得写 stdout/stderr。

### 方案

```text
Ctx::root_with_sink(Handle::current(), rutis_error_sink())
  └─ rutis_error_sink() -> ErrorSink
       └─ log::error!(target: "xylitol::rutis", "{error}")
```

- `rutis_error_sink` 放在 `xylitol-kernel`，只做一件事：把 `Arc<ErrorKind>` 交给 `log`。
- `boot_ctx` 在 tokio runtime 内运行，`Handle::try_current()` 失败时映射为 `BootstrapError::KernelFailed`。
- 不使用 `default_sink()` 兜底：一旦安装自定义 sink，rutis 不再写 stderr；错误仍完整进入文件日志。
- 日志目标固定为 `xylitol::rutis`，便于 `RUST_LOG` 窄读。

### 测试

- 单测：安装一个进程内 `log::Log` 捕获器，调用 `rutis_error_sink` 传入一个 `ErrorKind`，断言捕获到 `xylitol::rutis` 记录。
- 不在 BDD 里做全局 logger 注入：logger 是进程级单例，BDD 并行下成本高；spec 规则用裸规则 + 单测守护。

## 2. 启动诊断

### 现状

`wait_active(ctx)` 轮询 `ctx.diagnostics()`；超时只输出 `name:state`。

### 方案

```text
wait_active(ctx, loader)
  ├─ 先按现有逻辑检查 Failed / Disposed
  ├─ 若仍有 Pending / Loading：
  │    ├─ loader.entries()
  │    ├─ loader.waits(entry) 非空 -> "row: service (reason), ..."
  │    └─ 无 waits -> 回落 "name:state"
  └─ 超时错误合并上述描述
```

- `Wait` 没有 `Display`；写一个 `describe_wait` 把 `service` 与 `reason` 变体转成短句。
- `WaitReason` 是 `#[non_exhaustive]`，必须保留 wildcard。
- 只读诊断，不改变启动成功路径。

### 测试

- 纯函数测试：构造 `Wait` / `WaitReason` 样例，断言格式化包含服务名与原因。
- kernel 自检测试：保留现有 boot 成功路径；超时路径由纯函数测试覆盖，避免依赖真实 loader 的时序。

## 3. 风格收口

- `ctx.plugin(p)` 返回的 `FiberView` 直接 `.await`；不再写 `(&view).await`。
- `boot_ctx` 用 `root_with_sink` 后，测试里的 `Ctx::root_with_sink` 保持原样。

## 风险

- **日志重复**：自定义 sink 后，rutis 不再写 stderr；若用户依赖 stderr 可见性，需要 `RUST_LOG` 打开 `xylitol::rutis`。文档在 proposal / commit 说明。
- **测试全局 logger**：单测捕获器必须用 `serial` 或独立测试进程；优先独立测试文件 / `#[serial]`，避免污染其他测试。
- **WaitReason 演进**：`#[non_exhaustive]`，格式化必须保留 wildcard。

## 非目标

- 不在本 change 做 `shutdown_with_timeout`；它需要 `Kernel` 所有权与 serve 注册文件收口的单独设计。
- 不引入 `PluginFactory` / `Key<T>`；当前没有对应需求。
