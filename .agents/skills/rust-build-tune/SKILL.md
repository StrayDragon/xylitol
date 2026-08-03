---
name: rust-build-tune
description: '当需要优化 Rust 编译速度、产物体积或磁盘占用/worktree 缓存时使用：分析 Cargo.toml profile、Cranelift、codegen-units/opt-level/LTO/strip、mold/MUSL/UPX；以及 debuginfo/incremental/toolchain 清盘、禁止跨 worktree 共享 CARGO_TARGET_DIR、推荐 sccache。Use when optimizing Rust build speed, binary size, or disk/worktree caches: profiles, Cranelift, linker, and disk-health (line-tables-only, rustup prune, per-worktree target + sccache). Keywords: Rust, Cargo, build, disk, target, worktree, sccache, debuginfo, incremental, Cranelift, mold, LTO, strip'
metadata:
  origin_post: https://canmi.net/posts/development/rust-build-config-cranelift-profile-optimization
---

# rust-build-tune

Rust 项目构建配置调优：分析 Cargo.toml 和 .cargo/config.toml，按用户目标输出方案——**速度**、**产物体积**、或 **磁盘健康 + worktree 并行**。

## 概述

覆盖维度：

- **dev profile**: 快速编译（Cranelift、高 codegen-units、依赖 O3 + 业务 O0）**或**磁盘优先（`line-tables-only`、控制 incremental）
- **release profile**: LTO、strip、panic=abort、低 codegen-units
- **链接器 / 平台**: mold、可选 MUSL；UPX 仅 CLI 场景
- **磁盘与 worktree**: rustup 瘦身、`target/` 卫生、**禁止**跨 worktree 共用 `CARGO_TARGET_DIR`、registry/sccache 共享（见 [disk-and-worktree.md](references/disk-and-worktree.md)）

## 工作流程

### 0. 澄清优化目标（必做）

| 目标 | 主 reference |
|------|----------------|
| 编译速度 | [dev-profile.md](references/dev-profile.md) + [cranelift.md](references/cranelift.md) + [linker.md](references/linker.md) |
| 发布体积 | [release-profile.md](references/release-profile.md) + [binary-size.md](references/binary-size.md) |
| 磁盘 / 多 worktree | [disk-and-worktree.md](references/disk-and-worktree.md)（可叠加降 `debug` 的 dev 片段） |

冲突时显式权衡，勿把「速度默认」当成磁盘场景的推荐。

### 1. 读取项目配置

读取以下文件（如存在）：

- `Cargo.toml` — 现有 profile 配置
- `.cargo/config.toml` — 链接器、目标平台、rustflags
- `rust-toolchain` / `rust-toolchain.toml` — 工具链版本
- 环境：`CARGO_TARGET_DIR`、`RUSTC_WRAPPER`、`SCCACHE_*`；`du` 看 `target/`、`~/.cargo`、`~/.rustup`

### 2. 分析项目特征

| 特征 | 检查方式 | 影响 |
|------|----------|------|
| 是否含 unsafe/FFI/ASM | grep `unsafe` / `extern "C"` / `.s` 文件 | Cranelift 兼容性 |
| 加密库依赖 | 检查 `ring` / `aws-lc-rs` / `boring` | Cranelift 兼容性 |
| 目标平台 | `[target]` 配置、CI 文件 | 链接器、MUSL 选择 |
| 项目类型 | CLI / 服务 / 库 | UPX、strip 策略 |
| 依赖数量 | `[dependencies]` / `cargo metadata` 节点数 | 增量编译、Cranelift、**debuginfo 盘压** |
| `target/` 结构 | `du` deps/incremental/自定义子树 | 清盘与 profile 优先级 |
| worktree 用法 | `git worktree list`、是否共享 `CARGO_TARGET_DIR` | **正确性**：禁止同 crate 跨树共 target |

### 3. 按场景推荐配置

- **编译速度优先** (dev): [dev-profile.md](references/dev-profile.md) + [cranelift.md](references/cranelift.md)
- **产物优化优先** (release): [release-profile.md](references/release-profile.md) + [binary-size.md](references/binary-size.md)
- **链接器调优**: [linker.md](references/linker.md)
- **磁盘 / worktree**: [disk-and-worktree.md](references/disk-and-worktree.md)

### 4. 应用并验证

```bash
# 验证 dev 编译
cargo check
cargo build

# 验证 release 编译
cargo build --release

# 检查产物体积与磁盘
ls -lh target/release/<binary>
du -sh target target/debug 2>/dev/null

# worktree：确认各树 CARGO_TARGET_DIR 不同；勿共用
# sccache（若启用）
sccache --show-stats
```

## 决策指南

### Cranelift：用还是不用？

| 条件 | 推荐 |
|------|------|
| 纯 Safe Rust，无 FFI/ASM | 用 Cranelift（dev profile） |
| 含 `ring` 等加密库 | 用 `cfg` 条件编译绕过，或回退 LLVM |
| 含 unsafe/FFI/ASM 的 crate | 不用 Cranelift，这些 crate 回退 LLVM |
| 库项目（发布给外部） | 不用 Cranelift，不影响下游 |
| **`-j1` / 低内存环境** | **不用 Cranelift**（见下方警告） |

> **⚠️ `-j1` 环境下 Cranelift 会更慢**：Cranelift 的优势是轻量并行 codegen-units，但 `-j1` 只有 1 个编译 job，无法利用并行。实测 28 依赖项目，Cranelift clean build 比 LLVM **慢 4 倍**（26m vs 6m）。原因：Cranelift 单线程 codegen 比 LLVM 慢，且无法通过多 codegen-units 补偿。

### 编译并行度：`-j1` vs `-j4+`

| 场景 | `-j1`（低内存） | `-j4+`（充足内存） |
|------|----------------|-------------------|
| Cranelift | ❌ 更慢（无并行优势） | ✅ 显著提速 |
| 依赖 `opt-level = 3` | ❌ 串行 O3 慢 3.8x | ✅ 并行无感 |
| mold linker | ≈ 持平（链接不是瓶颈） | ✅ 链接阶段提速 |
| `incremental = true` | ✅ **关键优化**（5x+） | ✅ 有益 |

**低内存规则**：优先保证 `incremental = true` + `debug = "line-tables-only"`，不要开 Cranelift 或依赖 O3。

### 存储优化：debug 等级选择

| `debug` 值 | `.debug_info` | `.debug_line` | 典型 deps 大小 | 适用场景 |
|------------|--------------|---------------|---------------|---------|
| `2` (full) | ✅ 大 | ✅ | ~57GB+ | **仅**需要 gdb/lldb 变量级调试 |
| `1` | ✅ 中 | ✅ | ~30GB | 旧默认；磁盘仍重 |
| `"line-tables-only"` | ❌ | ✅ | **~4GB** | **日常 / 磁盘优先（默认推荐）** |
| `0` | ❌ | ❌ | ~2GB | 极致省空间，无 backtrace |

> **实测数据**（28 依赖、129K SLoC 项目）：`debug=1` → `debug="line-tables-only"` 将 `target/` 从 **64GB 降到 7GB（9x 缩减）**。backtrace 仍显示函数名+行号，仅丢失变量类型信息。
> 本机大二进制抽样：`debug=2` 下可执行文件内 `.debug_*` 常占 **~80%**。多 worktree / 自定义 `target/bdd-*` 会把上述体积再乘份数。

磁盘与 worktree 的完整决策（含 **禁止共享 `CARGO_TARGET_DIR`**、sccache、清盘清单）见 [disk-and-worktree.md](references/disk-and-worktree.md)。

### Worktree 与 `CARGO_TARGET_DIR`（硬规则）

| 做法 | 结论 |
|------|------|
| 每 worktree **独立** `CARGO_TARGET_DIR` + 共享 `~/.cargo` + **sccache** | ✅ 推荐 |
| 不同 **package name** 的项目共用一个 target 目录 | ✅ 依赖可复用 |
| 同 crate 多个 worktree **共用**一个 `CARGO_TARGET_DIR` | ❌ **禁止**（Cargo 可错误 `Fresh`，二进制串味；1.97.x 已复现） |

### `incremental` 开还是不开？

| 条件 | 推荐 |
|------|------|
| 单 crate 项目、磁盘充裕 | ✅ 开（增量构建 53s → 10s） |
| 多 crate workspace（有 stale artifact 问题） | 关，或用独立 `dev-fast` profile |
| CI 环境 | 关（每次 clean build，增量缓存无用） |
| 磁盘紧张、可接受更慢增量 | **关**，或开但定期 `cargo clean` / 删旧 worktree 的 target |
| 磁盘紧张但仍要快速迭代 | 开 + `line-tables-only`（先砍 DWARF，再谈 incremental GB） |

### opt-level 选择

| 场景 | 推荐值 | 说明 |
|------|--------|------|
| dev 业务代码 | `0` | 最快编译 |
| dev 第三方依赖 | `3` | 首次慢，后续增量无感 |
| release 体积优先 | `"z"` | 最小体积 |
| release 性能优先 | `3` | 最快运行 |

### LTO 选择

| 场景 | 推荐值 | 说明 |
|------|--------|------|
| release（推荐） | `"thin"` | 编译时间可接受，优化效果好 |
| release（极致优化） | `true`（= "fat"） | 编译最慢，产物最优 |
| dev | `false`（默认） | 不开，编译最快 |

### panic 策略

| 场景 | 推荐 |
|------|------|
| 测试覆盖完善、无 unwind 需求 | `panic = "abort"`（减小体积、加速链接） |
| 需要 catch_unwind / 双重 panic 处理 | `panic = "unwind"`（默认） |
| 库项目 | 保持默认 `unwind`，不替下游决定 |

## 参考索引

| 文件 | 内容 |
|------|------|
| [dev-profile.md](references/dev-profile.md) | dev profile 完整配置（opt-level、codegen-units、Cranelift） |
| [release-profile.md](references/release-profile.md) | release profile 完整配置（LTO、strip、codegen-units=1） |
| [cranelift.md](references/cranelift.md) | Cranelift 后端详解、配置方法、兼容性限制 |
| [linker.md](references/linker.md) | 链接器选择（mold）、MUSL 静态链接、交叉编译 |
| [binary-size.md](references/binary-size.md) | strip、panic=abort、UPX 压缩 |
| [disk-and-worktree.md](references/disk-and-worktree.md) | 磁盘健康、清盘、worktree/`CARGO_TARGET_DIR`、sccache |

## 完成检查清单

- [ ] 已确认本次目标：速度 / 体积 / **磁盘·worktree**
- [ ] `Cargo.toml` 中 `[profile.dev]` 和 `[profile.release]` 已按目标配置
- [ ] 磁盘场景下 dev 使用 `debug = "line-tables-only"`（除非用户要完整 DWARF）
- [ ] `[profile.dev.package."*"]` 已按并行度/内存评估是否保留 O3
- [ ] 链接器已按平台配置（Linux: mold, macOS: 默认）
- [ ] 已评估 Cranelift 适用性（无 FFI/ASM 冲突；`-j1` 不用）
- [ ] worktree：每树独立 `CARGO_TARGET_DIR`；未建议跨树共 target
- [ ] 若并行多树：已提示 sccache + 可选 `SCCACHE_CACHE_SIZE`
- [ ] `cargo check` 通过；需要时 `cargo build --release`
- [ ] `du -sh target`（及 toolchain）符合预期
- [ ] CI 中保留 stable + LLVM 的 full build 作为兜底
