# c390 — Tasks

> 方案已定稿（design.md）。任务按 ≤2h 切片。

## 0. 规划工件

- [x] proposal.md（ready）
- [x] design.md（ready）
- [x] specs/debug-log/spec.toon（ready）

## 1. 依赖与 init 模块

- [x] `Cargo.toml`：新增无条件依赖 `tracing-subscriber`（带 `env-filter` feature）
- [x] 新增 `src/app/cli/logging.rs`：`init_logging(agent_dir)` 装同步 file-only subscriber（`OpenOptions::append` + unix `mode(0o600)` + `with_ansi(false)` + `try_init`）；env 决定是否装：`RUST_LOG`（非空）优先，`XYLITOL_DEBUG=1` 次之，否则 no-op
- [x] `src/app/cli/mod.rs`：`mod logging;` 并在 `run()` 内 `CliArgs::parse()`（:81）之后、子命令分发（:84）之前调 `logging::init_logging(&default_agent_dir())`

## 2. 最小 seam 埋点（可选，低优先）

- [x] `app/cli/logging.rs::init_logging`：init 段加 `tracing::info!("logging enabled path=...")`
- [x] `app/tui/mod.rs:194`（driver.run 提交）+ `:208`（XyEvent bridge）：加 `tracing::debug!`/`trace!`，仅调宏、不跨 seam

## 3. 测试与验证

- [x] 单测：`open_append`（建文件/目录、append 不截断、只读父目录 no-op）+ `is_truthy`；env 驱动分支因动全局 subscriber/env 改走手测（见下）
- [x] 回归：`just qa` 全绿（arch_guard 4/4、BDD 87/87、fmt/clippy/docs clean；prek 未装属环境）
- [x] 手测：`RUST_LOG=debug` / `XYLITOL_DEBUG=1` → 建文件 + 内容；无 env / 空 `RUST_LOG=` → 不建文件；权限 0600；stdout/stderr 零输出
