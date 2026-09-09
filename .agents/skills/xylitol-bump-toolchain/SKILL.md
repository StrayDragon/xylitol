---
name: xylitol-bump-toolchain
description: >-
  Bump xylitol's dated Rust nightly pin, rust-version, and crates.io
  dependencies. Use whenever the user mentions rust-toolchain.toml, rustc
  nightly, rustup update, cargo update, Cargo.lock drift, clippy after a
  toolchain bump, MSRV/rust-version vs nightly, or upgrading workspace crates.
  Not for Cranelift/profile/disk tuning (use rust-build-tune) and not for
  architecture/dead-code audits.
---

# xylitol 工具链与依赖 bump

本仓是 **应用 + 未发布库**（Pre-0.0.1）：编译器钉 dated nightly，不追滚动 `channel = "nightly"`，不为下游保留旧 rustc / 旧 crate 别名。

## SSOT

| 文件 | 钉什么 |
|---|---|
| `rust-toolchain.toml` | **实际编译器**：`channel = "nightly-YYYY-MM-DD"` + `rustfmt`/`clippy` |
| 各 `Cargo.toml` 的 `rust-version` | Cargo MSRV **声明**：必须是 **已发布 stable**，且 **小于** 该 nightly 的 `x.y.z` |
| `Cargo.lock` | 解析出的 crate 图 |
| `references/lessons.md` | 历次 bump 踩坑（先读最近一条再动手） |

`rust-build-tune` 管 profile / Cranelift / worktree target；本 skill 不管那些，只交接「先 `eval "$(just cargo-wt-env)"`」。

## 工作流

1. **读本机与 pin**
   `rustup check`；`rustc --version --verbose`（项目目录会走 override）。
   滚动 toolchain `nightly` 的 commit **不必**等于 dated `nightly-YYYY-MM-DD`：日期 channel 是「那天发布的 nightly」，commit-date 常是前一天；同日稍后 `rustup update nightly` 可能更新。

2. **钉日期**
   改 `rust-toolchain.toml` 的 `channel` 为 `nightly-YYYY-MM-DD`（与要锁定的发布日一致）。不要写成裸 `nightly`。

3. **安装完整 sysroot**
   ```bash
   rustup toolchain install nightly-YYYY-MM-DD --profile minimal --component rustfmt,clippy
   ```
   装完立刻检查：
   - `$(rustc --print sysroot)/lib/librustc_driver*.so` 存在
   - `ls "$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/lib"` 非空
   中断的 rustup 会留下能 `--version` 但不能链接 `core` 的残缺 rustc。此时 **uninstall + 删 toolchain 目录 + 重装**，不要在半装上 `cargo check`。

4. **对齐 `rust-version`**
   设成当前 **stable**（例如本机 `rustup check` 的 stable）。
   **禁止**写成该 nightly 的 `x.y.z`（`1.100.0-nightly` 是 prerelease，Cargo MSRV 检查可能直接拒绝）。代码一旦启用 `#![feature]`，在 lessons 里记一笔：声明与 pin 已经分叉。

5. **编译器闸**
   `cargo fmt --all`
   `cargo clippy --all-features --all-targets -- -D warnings`
   `cargo clippy -p xylitol-tui --all-targets -- -D warnings`
   新 nightly 的 clippy 默认允许变 deny：就地改代码（Pre-0.0.1，不留兼容写法）。

6. **依赖（两层）**
   - `cargo update`：lockfile 升到 Cargo.toml 已允许的最高兼容版。
   - 根 `Cargo.toml` 里 **写了精确 x.y.z 的直接依赖**：把数字同步到 lock 里实际解析的版本（提高下次 bump 地板）。
   - workspace 成员里 `"1"` / `"0.7"` 这种宽松 pin **不要**无故改成精确版。
   - **大版本**（`cargo update` 提示 `available: vN`）另开一轮，不要和 rustc pin 绑死。先记 lessons。

7. **Cargo 新 lint**
   nightly Cargo 可能对 manifest 报警：`cargo::manual_readme`（可删显式 `readme = "README.md"`）、`cargo::unused_dependencies`（真没用就删，Pre-0.0.1 不留）。

8. **记 lessons**
   在 `references/lessons.md` 追加一条：日期、channel、rustc 完整 `--verbose` 一行、clippy/Cargo 新闸、推迟的 major、future-incompat。

9. **验证**
   至少 clippy 绿。合并前 `just qa`。Cranelift 不在本流程默认打开。

## 硬规则

- 不滚动钉 `channel = "nightly"`（CI 会隔夜被 clippy/rustc 打红）。
- 不为旧 rustc / 旧 crate 留兼容别名。
- 不在 skill / 仓库文档里写代理或镜像；下载失败按操作者环境重试。
- `async_trait` + `dyn` port **不是**「升 nightly 就可以删」的项（见 `src/AGENTS.md`）。
- 同 crate 多 worktree 仍禁止共用 `CARGO_TARGET_DIR`。

## 报告给用户

写清：新 `channel`、`rustc --version` 一行、`rust-version`、lockfile 是否 `cargo update`、跳过的 major、clippy 是否绿、`just qa` 是否已跑。
