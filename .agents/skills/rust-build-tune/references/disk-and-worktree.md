# 磁盘健康与 worktree 编译缓存

目标：在「不怕更慢编译」的前提下减少 `target/` / toolchain 占用，并让 **git worktree 并行开发**安全复用缓存。

> 完整调研与本机证据：xylitol 仓 `docs/research/rust-disk-worktree-cache-2026.md`（若在该仓工作）。

## 调优前先选目标

| 目标 | 优先杠杆 | 不要默认做的事 |
|------|----------|----------------|
| 编译速度 | Cranelift（条件允许）、高 codegen-units、deps O3、mold、incremental | 为速度开 `debug=2` 而不提磁盘成本 |
| 发布产物体积 | release LTO / strip / opt-level z / panic=abort | 用 UPX 掩盖未 strip |
| **磁盘 + 多 worktree** | `debug=line-tables-only`、少 toolchain、每树独立 target、sccache | **跨 worktree 共用 `CARGO_TARGET_DIR`** |

一次会话只主攻一个目标；三者冲突时显式权衡。

## 占盘放大器（按影响）

| 放大器 | 磁盘 | 说明 |
|--------|------|------|
| `debug = 2` | ★★★★★ | DWARF 嵌进几乎每个依赖产物；二进制内 `.debug_*` 常占 70–80% |
| 多份 `target/`（worktree / 自定义子目录） | ★★★★★ | 同依赖图线性拷贝（例：`target/bdd-*`） |
| 多 rustup toolchain | ★★★★★（全局） | 每套常 0.7–1.7G |
| `incremental = true` | ★★★★ | 单仓可数 GB |
| 高 `codegen-units` | ★★ | 碎文件 / inode |
| `dev.package."*".opt-level = 3` | ★★ | 主要吃时间与依赖体积 |
| `~/.cargo/registry/src` | ★★ | 可清后重解压 |

实测参考（大依赖图）：`debug=1` → `line-tables-only` 可将 `target/` 从数十 GB 降到个位数 GB；`debug=2` 更肥。

## 磁盘优先 dev profile

```toml
[profile.dev]
opt-level = 0
codegen-units = 16              # 8–64；256 更快但更碎、略肥
debug = "line-tables-only"      # 行号 backtrace；无完整变量/类型 DWARF
incremental = false             # 盘紧或多 profile 时关；要增量可 true + 定期 clean

[profile.dev.package."*"]
opt-level = 3                   # 可选：不怕冷编译时间可留
```

折中：`line-tables-only` + `incremental = true` + `codegen-units = 64`。

**不要**在「磁盘优先」叙事下默认推荐 `debug = 2`。完整 DWARF 仅在用户明确需要 gdb/lldb 变量级调试时开启。

## Worktree：共享什么 / 禁止什么

### 禁止（正确性）

**禁止**多个同 crate、不同路径的 worktree 共用一个 `CARGO_TARGET_DIR`。

Cargo（至少至 1.97.x）对同名 package 的 fingerprint **不区分绝对路径**：另一棵树可被标成 `Fresh`，却运行到**别的树**编出的二进制（静默串味）。已用最小 bin crate 复现。

同 target 上并行 `cargo` 还会抢全局锁，并行体验差。

### 应该共享

| 层 | 路径 | 原因 |
|----|------|------|
| registry / git | `~/.cargo/registry`, `~/.cargo/git` | 按 crate 内容寻址，跨项目安全 |
| sccache | `~/.cache/sccache` + `RUSTC_WRAPPER=sccache` | 按编译输入哈希，跨 worktree 安全 |
| toolchain | 尽量少的 rustup 套件 | 全局税 |

### 必须私有

| 层 | 建议 |
|----|------|
| `CARGO_TARGET_DIR` / `target/` | 每 worktree 一份 |

```bash
# xylitol 仓内助手（推荐）：
source scripts/cargo_worktree_env.sh
# 或：eval "$(scripts/cargo_worktree_env.sh --print)"
# just： just cargo-wt-env

# 等价手写：
repo=$(basename "$(git rev-parse --show-toplevel)")
root=$(git rev-parse --show-toplevel)
wt_id=$(printf '%s' "$root" | sha1sum | cut -c1-12)
export CARGO_TARGET_DIR="$HOME/.cache/cargo-targets/${repo}/${wt_id}"
mkdir -p "$CARGO_TARGET_DIR"
# sccache 跨树安全（需已安装）：
# export RUSTC_WRAPPER=sccache
# export SCCACHE_CACHE_SIZE=20G   # 防止缓存反噬磁盘
```

### 何时「共享同一个 target」仍然 OK

- **不同 package name** 的多个项目指向同一 `CARGO_TARGET_DIR`：第三方依赖可 Fresh 复用（有收益）。
- **同一 worktree、同一源树** 的多次构建：默认行为。

## 清盘检查清单（agent 输出命令；**默认不执行**，除非用户明确授权）

1. `du -sh target ~/.cargo ~/.rustup/toolchains`；大仓再 `du -sh target/debug/{deps,incremental}`
2. `rustup toolchain list` → 卸载未 pin 的旧版本（常可回收十余 GB）
3. 闲置项目 `rm -rf …/target`；自定义子树（如 `target/bdd-*`）优先砍
4. `cargo cache -i` / 清理 `registry/src`
5. 若用 sccache：设 `SCCACHE_CACHE_SIZE`，定期 `sccache --show-stats`

xylitol：`source scripts/cargo_worktree_env.sh` 或 `eval "$(just cargo-wt-env)"`。

## 与速度调优的冲突提示

| 速度向默认 | 磁盘向替代 |
|------------|------------|
| `debug = 2` | `line-tables-only` |
| `codegen-units = 256` | `16`–`64` |
| `incremental = true` | 盘紧则 `false` 或定期 clean |
| 每 worktree 一份完整 target 且无 sccache | 私有 target + sccache |
| 留十几套 toolchain「备用」 | 只留 stable + 项目 pin |

## 验证

```bash
# 磁盘
du -sh target target/debug 2>/dev/null

# worktree 隔离冒烟：两树同 crate 改不同 println，分别 build 后运行各自 target 下二进制，输出必须不同
# sccache
sccache --show-stats
```

读 `readelf -SW target/debug/<bin> | rg debug_`：`line-tables-only` 后 `.debug_info` 应显著缩小或消失，`.debug_line` 仍可在。
