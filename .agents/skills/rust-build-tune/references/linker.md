# 链接器与目标平台配置

## 链接器选择

链接器是编译流程最后一环，负责把 `.o` 目标文件拼成可执行文件。默认链接器可能是最慢的那个（Linux 上的 GNU ld/bfd），替换后链接阶段提速明显。

### Linux: mold

[mold](https://github.com/rui314/mold) 是 Rui Ueyama 写的高性能链接器，多线程处理符号解析和重定位，比 GNU ld 快数倍。

安装：

```bash
# ArchLinux
pacman -S mold

# Ubuntu/Debian
apt install mold

# 从源码
cargo install mold
```

配置 `.cargo/config.toml`：

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

需要同时安装 `clang`（mold 通过 clang 的 `-fuse-ld` 参数调用）。

### macOS: Apple ld64

macOS 上无需额外配置。Apple 自带的 ld64 在 Xcode 15+ 后速度提升非常明显。lld 也快但 Apple 生态下 ld64 更稳定。

唯一注意：macOS CI 中可能遇到 Xcode 许可协议未接受导致链接器失败，需在 CI 中加 `sudo xcodebuild -license accept`。

## MUSL 静态链接

### 什么是 MUSL

MUSL 是一个轻量 C 标准库，能生成完全静态链接的二进制——不依赖系统动态库，拷到任何 Linux 机器即可运行。适合容器、嵌入式、CLI 工具分发。

### Dev vs Release 目标选择

| 阶段 | 推荐 target | 原因 |
|------|------------|------|
| dev | `x86_64-unknown-linux-gnu` | 动态链接编译更快 |
| dev (macOS) | `aarch64-apple-darwin` | 唯一选择，macOS 不支持静态链接 |
| release | `x86_64-unknown-linux-musl` | 静态链接，可移植 |

MUSL 静态链接比 gnu 动态链接慢在链接阶段（所有东西都打包进去），但 release 本来就慢一次，产物可移植性更重要。

### MUSL 配置

安装 target：

```bash
rustup target add x86_64-unknown-linux-musl
```

`.cargo/config.toml`：

```toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
ar = "x86_64-linux-musl-ar"
```

需要安装 musl 工具链：

```bash
# ArchLinux
pacman -S musl

# Ubuntu/Debian
apt install musl-tools
```

编译：

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

## 交叉编译

### Linux host → Linux target

推荐 [cross](https://github.com/cross-rs/cross)（Docker 驱动的交叉编译工具）：

```bash
cargo install cross
cross build --release --target aarch64-unknown-linux-gnu
```

环境全齐，glibc 版本可控。如果需要 glibc 兼容性，建议用 debian 大版本 -2 的 glibc 编译（不是用新 feature，是 glibc 的版本检查会拒绝运行）。

### macOS host → 任意 target

直接用 rustup target + cargo：

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

macOS 上的交叉编译工具链通常比 Linux 更顺畅。

### 不要用 zig build

`zig cc` 作为交叉编译 C 工具链在 nightly Rust 上问题很大，不稳定。不推荐。
