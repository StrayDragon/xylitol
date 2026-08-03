# 产物体积优化

## strip：移除调试符号和符号表

Rust release 产物包含大量调试信息（DWARF）和符号表（`.symtab`），strip 可以移除它们。

### strip 级别

```
Full binary
├── .text              (代码段)
├── .rodata/.data      (只读/可变数据)
├── .symtab            (符号表)
├── .debug_* (DWARF)   (调试信息)
└── .eh_frame (unwind) (栈展开表)

strip="debuginfo" → 移除 .debug_*
strip="symbols"   → 移除 .symtab + .debug_*
panic="abort"     → 额外移除 .eh_frame
```

### Cargo.toml 配置

```toml
[profile.release]
strip = "symbols"    # 或 true，等价于 "symbols"
```

体积效果：可能从 50MB → 5MB，差异巨大。同时避免符号泄露（类似前端 source map 泄露源码的问题）。

### dev 下不要 strip

dev profile 需要完整调试信息：`gdb` / `lldb` 设断点、查看变量、backtrace 显示函数名都依赖这些信息。

### ArchLinux 打包注意

ArchLinux 打包流程默认会 strip。如果你的 `Cargo.toml` 已经设了 `strip = "symbols"`，AUR PKGBUILD 需显式跳过：

```bash
# PKGBUILD
options=('!strip')
```

否则双重 strip 可能报错。

## panic = "abort"：移除 unwind 表

详见 [release-profile.md](release-profile.md#panic--abort)。

panic = "abort" 让 Rust 在 panic 时直接终止进程，不展开调用栈。这省去了编译器生成 `.eh_frame` unwind 表的工作，减小产物体积、减少链接器工作量。

**前提**: 测试覆盖到位，业务不依赖 `catch_unwind`。

## UPX：二进制压缩

[UPX](https://upx.github.io) 是可执行文件压缩工具，将 BIN 压缩后包上一个解压 stub，运行时先在内存解压再执行。

### 适用场景

| 场景 | 适合 | 原因 |
|------|------|------|
| CLI 工具（MUSL 静态） | 适合 | 静态链接内部稳定，UPX 干净压缩 |
| 低频工具 / 长任务 | 适合 | 冷启动解压几乎无感 |
| Docker 服务 | 不适合 | 内存换磁盘空间是血亏；容器 Layer 压缩和 UPX 效果重复 |
| 高频短任务 | 不适合 | 每次启动都要内存解压，高频路径上延迟明显 |

### UPX + MUSL 组合（推荐）

MUSL 全静态链接 + UPX 是最佳组合：内部结构稳定，UPX 压缩/解压干净，无动态库干扰。

CI 中加一步：

```yaml
# GitHub Actions 示例
- name: Compress binary with UPX
  run: |
    upx --best target/x86_64-unknown-linux-musl/release/<binary>
```

体积效果：通常压缩到原来的 30%-50%。

### UPX + GNU glibc 动态链接（不要用）

glibc 动态链接的二进制有特殊 section 和动态加载机制，UPX 压缩后可能破坏这些结构，运行时 `segment fault`。

### UPX 限制

- 只适合 ELF 二进制（Linux），macOS 的 Mach-O 支持有限
- 压缩后的二进制无法再被 strip 或 `objdump` 分析
- 某些安全审计工具会对 UPX 打包的二进制报毒（误报，因为压缩 stub 的行为类似壳）
