# Release Profile 配置

目标：**最小体积 + 最优运行时性能**，编译时间可以接受较慢。

## 完整配置（推荐）

```toml
[profile.release]
opt-level = "z"          # 体积优先；性能优先用 3
codegen-units = 1        # 最大化跨单元优化
lto = "thin"             # 跨 crate 优化，编译时间可接受
strip = "symbols"        # 移除符号表和调试信息
panic = "abort"          # 移除 unwind 表（需确认无 catch_unwind 需求）
incremental = false      # release 不需要增量编译
```

## 各参数说明

### opt-level

| 值 | 编译速度 | 运行时性能 | 产物体积 | 适用场景 |
|----|----------|-----------|---------|---------|
| `3` | 最慢 | 最快 | 较大 | 计算密集型服务 |
| `"z"` | 慢 | 略慢于 3 | 最小 | CLI 工具、嵌入式 |
| `"s"` | 慢 | 略慢于 3 | 较小 | 平衡体积和性能 |

选择 `"z"` 还是 `3`：在不修改代码的前提下，`"z"` 是免费的体积优化。大多数 CLI 工具和 Web 服务选择 `"z"` 即可；如果是计算密集型（加密、编解码、数值计算），用 `3`。

### codegen-units = 1

整个 crate 作为单个编译单元交给后端优化。LLVM/Cranelift 能看到完整上下文，最大化内联、死代码消除、常量传播等优化。代价是编译最慢（无法并行），但 release 本来就慢，这个代价值得。

默认值是 16，改为 1 可能带来 5%-15% 的性能提升和明显的体积缩减。

### lto = "thin"

| 值 | 编译时间 | 优化效果 | 内存占用 |
|----|---------|---------|---------|
| `false` | 最快 | 最差（crate 独立优化） | 最低 |
| `"thin"` | 中等 | 好（跨 crate 优化，并行执行） | 中等 |
| `true` / `"fat"` | 最慢 | 最好（全局优化） | 最高 |

推荐 `"thin"` 作为默认选择。只有当你需要极致优化（如嵌入式、高频交易）且能接受编译时间时才用 `true`。

**注意**: LTO 会破坏增量编译——开 LTO 后修改代码需要重编译更多内容。只在 release profile 中开启。

### strip

| 值 | 移除内容 | 体积缩减 |
|----|---------|---------|
| `false` | 不移除 | - |
| `"debuginfo"` | DWARF 调试信息 | 适中 |
| `true` / `"symbols"` | 符号表 + 调试信息 | 大幅（可能 50MB → 5MB） |

推荐 `"symbols"`。release 不需要调试符号，strip 后体积大幅缩减，且避免符号泄露。

**注意**: ArchLinux 打包默认会 strip，如果你的 CI 已经 strip 了，AUR PKGBUILD 中需要显式跳过（`options=('!strip')`）。

### panic = "abort"

**前提**: 你的项目测试覆盖到位，业务逻辑不依赖 `catch_unwind`，没有需要在 panic 时执行 drop 清理的资源。

| 值 | 行为 | 产物体积 | 适用 |
|----|------|---------|------|
| `"unwind"`（默认） | 展开栈，调用 drop | 较大（需要 unwind 表） | 需要优雅处理 panic |
| `"abort"` | 直接终止进程 | 较小（无 unwind 表） | 测试到位、无 unwind 需求 |

`panic = "abort"` 的收益：
- 减小产物体积（移除 `.eh_frame` 等 unwind 段）
- 减少链接器工作量
- 编译略快

**不适合 abort 的场景**:
- 库项目（不应替下游决定 panic 策略）
- 需要 `catch_unwind` 的 FFI 边界
- 嵌入式场景需要在 panic 时安全释放硬件资源

## 极致优化配置（可选）

当你需要最大优化且不在乎编译时间时：

```toml
[profile.release]
opt-level = 3
codegen-units = 1
lto = "fat"
strip = "symbols"
panic = "abort"
```

编译可能非常慢，但产物运行最快。适合 CI 中运行、不在本地编译。
