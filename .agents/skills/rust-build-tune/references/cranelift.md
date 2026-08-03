# Cranelift 代码生成后端

## 为什么编译慢：理解编译管线

Rust 编译流程分前端和后端两部分：

```
.rs 源码
  → Lexer + Parser → AST
  → 宏展开 + Desugaring
  → HIR lowering + Name resolution
  ─── 前端 / 后端分界线 ───
  → Type checking + Trait solving
  → Borrow checker + Lifetime validation
  → MIR generation (CFG, const eval, inlining)
  → LLVM IR codegen (翻译 MIR → IR)
  → LLVM optimizations (O0~O3, 100+ passes)
  → Machine code (target-specific)
  → Linker → Executable binary
```

**前端**（AST → HIR → MIR）是 Rust 编译器自己的工作，这部分相对快。

**后端**（MIR → LLVM IR → 优化 → 机器码 → 链接）是编译慢的主要来源。LLVM 是为"生成最优机器码"设计的重型基础设施，不是为"快速完成编译"设计的。即使是 debug 模式的 O0，LLVM 依然要走完 IR 生成和机器码生成的全流程。再加上 Rust 的**泛型单态化**会在编译期展开大量代码，交给 LLVM 的 IR 体量远比源码看起来的要大得多。

所以编译慢的很大一部分时间不是花在 Rust 前端，而是花在 LLVM 这个"重型后端"上。这就是 Cranelift 存在的意义——提供一个轻量替代后端。

## 什么是 Cranelift

Cranelift 最早是 Cretonne 项目（2016 年启动），由 [Bytecode Alliance](https://bytecodealliance.org/) 开发，最初是为 [Wasmtime](https://github.com/bytecodealliance/wasmtime) WebAssembly 运行时设计的代码生成后端；后被 Rust 官方收编为可选 codegen 后端。它大幅削减 LLVM 的优化 pass，只做基本寄存器分配和指令选择，用一趟线性扫描完成代码生成。

## Cranelift vs LLVM

| 维度 | LLVM | Cranelift |
|------|------|-----------|
| 优化 pass | 100+（循环展开、向量化、常量传播、DCE…） | 极少（基本寄存器分配、指令选择） |
| IR 设计 | 多层 LLVM IR（几十年通用化包袱） | 单层 CLIF IR（专为快速翻译设计） |
| 编译速度 | 慢（即使 O0 也要走完整流程） | 快 2-5x（开发阶段体感明显） |
| 运行时性能 | 最优 | 慢 10%-30% |
| 兼容性 | 完整（支持所有 Rust 特性） | 纯 Rust 代码可用，FFI/ASM 兼容性差 |
| 适用阶段 | release | dev |

## 核心原则

编译时间换运行时性能——在开发阶段，这笔账应该倒过来算。你需要的不是最优机器码，而是快速反馈：改代码 → 编译 → 验证逻辑。运行时慢 10%-30% 在开发阶段无关紧要（CPU 大部分时间在摸鱼等你输入），但编译快 2-5x 能显著提升开发体验。

## 预期收益

基于 ~32K SLoC Rust 项目（monorepo，含多个子项目）的实测参考：

| 对比项 | dev (Cranelift + O0) | release (LLVM + LTO) |
|--------|---------------------|---------------------|
| 冷编译 | ~3x 快于 release | 基准 |
| 增量编译 | 快几十倍（无 LTO，增量生效） | 慢（LTO 破坏增量） |

**项目特征影响收益**：
- 代码量大、依赖少、纯 Safe Rust 的项目（如 Skeleton 包）：Cranelift 收益最大，codegen 占比最高
- 依赖多、含加密库的项目（如 CLI 包）：需要处理 FFI/ASM 兼容性问题（见下方兼容性限制），收益受限于回退 LLVM 的 crate 数量

**平台差异**：Apple Silicon (M 系列) 单核性能和内存带宽极强，编译差距会被部分缩小；在 Linux 上差距通常更大。

> **⚠️ `-j1` 环境实测警告**：Cranelift 的速度优势依赖并行 codegen-units。当 `cargo build -j1`（或 `CARGO_BUILD_JOBS=1`）时，Cranelift **比 LLVM 慢 4 倍**。实测数据（28 依赖、129K SLoC）：
> - LLVM + `-j1`: 6m 11s
> - Cranelift + `-j1`: **26m 08s**（4.2x 慢）
>
> 原因：Cranelift 单线程 codegen 效率低于 LLVM，且 `-j1` 无法通过多 codegen-units 并行补偿。**低内存环境应使用 LLVM + `incremental = true` + `debug = "line-tables-only"` 替代方案。**

## 配置方法

### 方法一：Cargo.toml（推荐）

```toml
[profile.dev]
codegen-backend = "cranelift"

# 指定不兼容的 crate 回退 LLVM
[profile.dev.package.ring]
codegen-backend = "llvm"
```

需要 nightly 工具链：

```bash
rustup default nightly
# 或项目级
echo "nightly" > rust-toolchain
```

### 方法二：.cargo/config.toml

```toml
[build]
rustflags = ["-Z", "codegen-backend=cranelift"]

# 按目标覆盖
[target.x86_64-unknown-linux-gnu]
rustflags = ["-Z", "codegen-backend=cranelift"]
```

### 安装 Cranelift

```bash
rustup component add rustc-codegen-cranelift-preview --toolchain nightly
```

## 兼容性限制

### 不适合 Cranelift 的场景

| 场景 | 原因 | 解决方案 |
|------|------|---------|
| `unsafe` 块密集的 crate | Cranelift 不处理 unsafe 语义边界 | 用 `cfg` 条件编译回退 LLVM |
| FFI（`extern "C"` / C 绑定） | Cranelift 对 FFI 调用约定支持不完整 | 该 crate 用 LLVM |
| 内联汇编（`asm!`） | Cranelift 不支持内联汇编 | 该 crate 用 LLVM |
| `aws-lc-rs` 等加密库 | 底层用汇编 FFI 加速 | 替换为纯 Rust 的 `ring`，或该 crate 用 LLVM |
| `ring` 的部分平台 | 某些平台仍有汇编 | 检查目标平台是否纯 Rust 实现 |

### 用 cfg 绕过不兼容依赖

```toml
# Cargo.toml — 在 Cranelift 模式下用 ring（纯 Rust）替代 aws-lc-rs（汇编 FFI）
[target.'cfg(not(target_feature = "cranelift"))'.dependencies]
aws-lc-rs = "0.1"

[target.'cfg(target_feature = "cranelift")'.dependencies]
ring = "0.17"
```

或者在 profile 级别指定特定 crate 回退：

```toml
[profile.dev.package.aws-lc-rs]
codegen-backend = "llvm"
```

## 注意事项

- Cranelift 目前需要 nightly 工具链，稳定版不可用
- 偶有 edge case 导致 Cranelift 编译失败但 LLVM 正常，概率极低（≈ rustc ICE 概率）
- CI 中应保留 stable + LLVM 的 full build 作为兜底，本地 nightly + Cranelift 用于日常开发
- 库项目不要用 Cranelift，因为 codegen-backend 不影响下游编译
