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
| 各 `Cargo.toml` 的 `rust-version` | Cargo MSRV **声明**：已发布 stable，且 **小于** 该 nightly 的 `x.y.z` |
| `Cargo.lock` | 解析出的 crate 图 |

`rust-build-tune` 管 profile / Cranelift / worktree target；动手前 `eval "$(just cargo-wt-env)"`。

不另写 bump 日志。新踩坑若六个月后仍真，收进本节硬规则；一次性现象写进 commit message。

## 工作流

1. **读本机与 pin**
   `rustup check`；项目目录 `rustc --version --verbose`。
   滚动 `nightly` 的 commit **不必**等于 dated `nightly-YYYY-MM-DD`（日期 channel = 那天发布的包；commit-date 常早一天）。

2. **钉日期**
   `channel = "nightly-YYYY-MM-DD"`。不要裸 `nightly`。

3. **安装完整 sysroot**
   ```bash
   rustup toolchain install nightly-YYYY-MM-DD --profile minimal --component rustfmt,clippy
   ```
   确认 `$(rustc --print sysroot)/lib/librustc_driver*.so` 和 `lib/rustlib/<host>/lib` 都在。中断安装会留下能 `--version` 却找不到 `core` 的残缺 rustc → uninstall、删目录、重装。

4. **对齐 `rust-version`**
   写成当前 **stable**。禁止写成该 nightly 的 `x.y.z`（`1.100.0-nightly` 是 prerelease，Cargo MSRV 可能拒编）。启用 `#![feature]` 时在 commit 里写明声明已与 pin 分叉。

5. **编译器闸**
   `cargo fmt --all`
   `cargo clippy --all-features --all-targets -- -D warnings`
   `cargo clippy -p xylitol-tui --all-targets -- -D warnings`
   新 deny 就地改代码，不留兼容写法。

6. **依赖**
   先改完 `Cargo.toml` 再 `cargo update`（或 `-p <crate> --precise`），然后核对 lock 里的版本。
   根 crate 写了精确 x.y.z 的直接依赖：同步到 lock 实际解析值。成员里 `"1"` / `"0.7"` 不要无故改成精确版。
   **大版本**另开一轮，不要和 rustc pin 捆在一起。

7. **Cargo manifest lint**
   `cargo::manual_readme` → 可删显式 `readme = "README.md"`。
   `cargo::unused_dependencies` → 真没用就删。

8. **验证**
   至少 clippy 绿；合并前 `just qa`。Cranelift 不在本流程默认打开。

## 硬规则

- 不滚动钉 `channel = "nightly"`。
- 不为旧 rustc / 旧 crate 留兼容别名。
- 不在 skill / 仓库文档里写代理或镜像。
- `async_trait` + `dyn` port 不因升 nightly 而删（`src/AGENTS.md`）。
- 同 crate 多 worktree 禁止共用 `CARGO_TARGET_DIR`。

## 报告给用户

新 `channel`、`rustc --version` 一行、`rust-version`、lock 是否已 update、跳过的 major、clippy / `just qa` 是否绿。
