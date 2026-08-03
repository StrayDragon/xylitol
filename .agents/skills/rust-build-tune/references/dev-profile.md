# Dev Profile 配置

目标：**最快编译速度**，牺牲运行时性能换取开发反馈循环速度。

## 完整配置

```toml
[profile.dev]
opt-level = 0          # 不优化，最快编译
codegen-units = 256    # 最大并行度，最快编译
debug = 2              # 完整调试信息（gdb/lldb 可用）
incremental = true     # 增量编译（默认开启，显式声明）

# 依赖用 O3 编译——只影响首次冷编译，后续增量无感
[profile.dev.package."*"]
opt-level = 3
```

## 各参数说明

### opt-level = 0

完全不优化。编译器跳过所有优化 pass，从 MIR 直接到代码生成。这是 dev 模式的默认值，显式声明以确保意图清晰。

依赖用 `[profile.dev.package."*"]` 设为 `opt-level = 3` 的原因是：第三方库源码不会频繁变动，首次 O3 编译后缓存住，后续增量编译不会重编译它们。而 O3 编译的依赖在运行时也更快（比如 serde 序列化、regex 匹配等）。

### codegen-units = 256

将 crate 拆成 256 个编译单元并行处理。数字越大并行度越高、编译越快，但优化效果越差（跨单元内联和优化被切断）。dev 模式下编译速度优先，256 是 Rust 的 dev 默认值。

### debug = 2

保留完整调试信息（DWARF），`gdb` / `lldb` 可正常设断点、查看变量、backtrace 显示函数名。

**磁盘成本极高**（依赖 `.rlib` 与二进制都膨胀；多 worktree 线性放大）。日常开发默认应优先 `"line-tables-only"`（见下方「存储优化配置」与 [disk-and-worktree.md](disk-and-worktree.md)）。仅在用户明确需要变量级调试时再用 `2`。`debug = 0` 会丢掉行号 backtrace，一般不推荐。

### incremental = true

增量编译默认开启，显式声明。只重编译修改过的 crate 及其依赖。注意：开启 LTO 或 `codegen-units = 1` 会影响增量编译效果。

## 存储优化配置（日常 / 磁盘优先时的默认推荐）

用 `line-tables-only` 替代完整 debug info；多 worktree 时再叠加「每树独立 target + sccache」（[disk-and-worktree.md](disk-and-worktree.md)）：

```toml
# 日常折中（仍要增量）
[profile.dev]
debug = "line-tables-only"  # 保留 backtrace 函数名+行号，去掉类型/变量信息
codegen-units = 64
incremental = true

# 磁盘优先（可接受更慢增量；xylitol 草案 A）
# codegen-units = 16
# incremental = false
```

效果（28 依赖、129K SLoC 项目实测）：

| 配置 | `target/debug/deps/` | `target/` 总计 |
|------|---------------------|---------------|
| `debug = 1` | 57GB | 64GB |
| `debug = "line-tables-only"` | **3.7GB** | **7.0GB** |
| `debug = 0` | ~2GB | ~5GB |

`line-tables-only` 是最佳折中：backtrace 仍然可用（显示函数名+行号），但 `.debug_info`（类型、变量、作用域）被移除——这是 DWARF 中最大的部分。

## 低内存 / `-j1` 配置

当 `CARGO_BUILD_JOBS=1` 或机器内存 ≤16GB 时：

```toml
[profile.dev]
debug = "line-tables-only"
codegen-units = 256
incremental = true

# 不要加 codegen-backend = "cranelift"
# 不要加 [profile.dev.package."*"] opt-level = 3
```

**警告**：
- Cranelift 在 `-j1` 下比 LLVM **慢 4 倍**（无法利用并行 codegen-units）
- 依赖 `opt-level = 3` 在 `-j1` 下**慢 3.8 倍**（28 个依赖串行 O3 编译）
- 这两个优化只在 `-j4+` 并行编译时有效

## Cranelift 配置（可选，需要 `-j4+`）

如果项目适合用 Cranelift（详见 [cranelift.md](cranelift.md)），且编译并行度 ≥4，在 dev profile 中启用：

```toml
[profile.dev]
codegen-backend = "cranelift"
```

需要 nightly 工具链，且项目不含 FFI/ASM 等 Cranelift 不支持的代码。

**⚠️ 不要在 `-j1` 环境使用 Cranelift**，会更慢。

## 不应该在 dev 中做的事

- **不要** 开启 `lto` — 破坏增量编译，编译慢几十倍
- **不要** 设 `strip = true` — 丢失调试信息
- **不要** 设 `panic = "abort"` — dev 阶段需要 unwind 信息定位 panic 位置
- **不要** 设 `codegen-units = 1` — 这会把编译拖到极慢
- **`-j1` 下不要** 用 Cranelift 或依赖 `opt-level = 3` — 串行编译会更慢
