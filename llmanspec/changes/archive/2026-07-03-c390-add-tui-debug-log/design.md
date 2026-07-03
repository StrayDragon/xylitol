# c390 Design — tracing subscriber 装配（文件日志 MVP）

> 方案已定稿。三个关键决策（env-only 激活 / 无条件依赖 / 同步 writer）由用户于 2026-07-03 拍板。

## 1. 问题与现状（代码事实）

- `tracing = "0.1.44"` 已是无条件依赖（`Cargo.toml:21`）。
- `infra/`+`agent/` 已有埋点（rg 验证）：`infra/event/mod.rs:79`、`infra/permission/mod.rs:155,159`、`infra/mcp/client.rs:48,78,94,108,155`、`infra/settings/{storage.rs:96,98, manager.rs:328}`、`infra/hooks/script.rs:15,74`、`infra/hooks/dispatcher.rs:5`、`agent/compaction/mod.rs:117`。
- **零 subscriber**：全仓库无 `tracing_subscriber::fmt`、无 `.init()`/`try_init()`、无 `RUST_LOG`（rg 验证）→ 上述埋点全部静默。
- `app/tui/` 下 rg `tracing::` **零命中** → TUI 层完全无声。
- TUI 用 `Viewport::Inline`（非 AlternateScreen，spec tui1 禁 alt-screen），raw mode + 每帧 DSR 光标查询（`tui/mod.rs:9-15` 文档此 hazard）→ stdout/stderr 任何输出必毁屏。
- `panic = "abort"`（`Cargo.toml:98`）→ Drop 不跑；已有 panic hook 仅恢复终端（`tui/init.rs:61-71`），不管日志。

## 2. 决策表

| 决策点 | 选项 | 选定 | 理由 |
|---|---|---|---|
| 激活机制 | flag+env+settings / env-only / config-only | **env-only** | 最小侵入：不动 `CliArgs`、不加 settings 字段。TUI 调试场景 `RUST_LOG=debug` 即用，零代码改动即可开关。 |
| 依赖门控 | 无条件 / 新 feature imply | **无条件** | 与既有 `tracing` 无条件一致；消除「emit 了却没 subscriber」的坑；`tracing-subscriber` 很轻。 |
| writer 模式 | non-blocking / 同步 / non-blocking+flush hook | **同步** | `panic="abort"` 下 non-blocking guard 来不及刷盘会丢最后几行；同步 append 在 debug 量级（既有埋点都是 warn/info 级低频）无性能问题；pi 即同步 `appendFileSync`。 |
| 落点 | `<agent_dir>/logs/xylitol.log`（复用 `DefaultResourceLoader::default_agent_dir()`，即 `~/.xylitol/`） | 同左 | 复用既有 home 解析（`cli/mod.rs:325,351` 已用），不引新依赖。 |
| 滚动 | 单文件 append / 按日滚动 | **单文件 append（MVP）** | codex TUI log 即单文件无滚动；滚动留 future。 |
| `tracing-appender` | 用 / 不用 | **不用** | 同步 writer 直接 `std::fs::OpenOptions` 即可；`tracing-appender` 的价值（`non_blocking`/`RollingFileAppender`）本 MVP 都不采用。少一个依赖。 |

## 3. 架构落点

```text
src/app/cli/mod.rs::run()        ← 唯一组合根（所有面共同入口）
   ├─ CliArgs::parse()           :81
   ├─ init_logging()             ← 【新增】装 subscriber（env 决定是否装）
   ├─ match args.command { ... } :84  (subcommands 早返回也覆盖)
   ├─ timing::reset_timings()    :99
   └─ ... → print / TUI(:456) / RPC(:382)
```

- **装在哪**：`src/app/cli/logging.rs`（新模块，`pub(crate) fn init_logging()`），由 `cli/mod.rs::run()` 在 `:81` 之后、`:84` 之前调用一次。这是所有面（print/TUI/RPC/Resources/Server）的共同前置，装此一处全覆盖。
- **为什么不在 `main.rs`**：`main.rs` 只 7 行、无 `CliArgs`、无 home 解析；组合根 `cli/mod.rs::run()` 才是第一处有完整上下文的点。
- **为什么不在 TUI crate 内**（与 codex 不同）：codex 在 `tui/src/lib.rs` 二次 init 是因它的 `main.rs` 多路分发且 config 在 TUI 内解析；xylitol 的 `cli/mod.rs::run()` 是单一共同入口，装一次即可，TUI 拿到终端时日志早已改道文件——天然安全。

## 4. 激活优先级（env-only）

```text
RUST_LOG 存在？  ─是→  EnvFilter::try_from_default_env()，装 subscriber
       │否
       ▼
XYLITOL_DEBUG=1？ ─是→  装 subscriber，默认 filter = "xylitol=debug,warn"
       │否
       ▼
不装 subscriber（off，所有 tracing:: 宏 no-op，零开销）
```

- `RUST_LOG` 走 `EnvFilter::try_from_default_env()`（仿 codex `tui/src/lib.rs:1206`），用户完全控制粒度。
- `XYLITOL_DEBUG=1` 给「不想记 RUST_LOG 语法」的用户一个一键开关，默认 filter 覆盖 `xylitol` 全部 + 所有 `warn`。
- **未装时零开销**：tracing 的宏在无 subscriber 时编译期即 no-op，不影响 print/server 生产路径性能。

## 5. 文件 writer 细节

```rust
// src/app/cli/logging.rs（示意，非最终代码）
let dir = agent_dir.join("logs");          // agent_dir = default_agent_dir()
std::fs::create_dir_all(&dir)?;            // ~/.xylitol/logs/
let mut opts = std::fs::OpenOptions::new();
opts.create(true).append(true);
#[cfg(unix)] { opts.mode(0o600); }          // 仿 codex：文件权限收紧
let file = opts.open(dir.join("xylitol.log"))?;
let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("xylitol=debug,warn"));
tracing_subscriber::fmt()
    .with_writer(file)                      // 同步、file-only，绝不 stdout/stderr
    .with_ansi(false)                       // 文件无 ANSI（仿 codex）
    .with_env_filter(filter)
    .try_init()                             // try_init 不 panic（测试/二次 init 安全）
```

- **无 `Box::leak`、无 guard**：同步 writer 无 background worker，无 flush 语义，Drop 即关闭 fd——`panic="abort"` 下 OS 自动回收 fd，已写入字节不丢。
- **try_init**：返回 `Result`，失败（如已装）静默忽略，不 panic。

## 6. 埋点策略

**MVP 只激活既有埋点 + 最小 seam 埋点，不大规模铺点。**

| 位置 | 级别 | 内容 | 来源 |
|---|---|---|---|
| `infra/*`（既有 ~12 处） | warn/info | 自动激活，零改动 | 已存在 |
| `agent/compaction`（既有） | warn | 自动激活 | 已存在 |
| `app/tui/mod.rs:194`（`driver.run` 提交） | debug | `turn start, prompt len=N` | 新增（可选） |
| `app/tui/mod.rs:250`（XyEvent bridge） | debug | `event={kind}` | 新增（可选） |
| `app/cli/mod.rs::run` init 段 | info | `logging enabled, filter=.., path=..` | 新增 |

- TUI 埋点**仅调 `tracing::debug!` 宏**，不 import infra、不跨 seam（守 `src/app/tui/AGENTS.md:36`）。
- render seam（pi 的 `PI_DEBUG_REDRAW` 风格 redraw-reason log）与 mpsc 流控埋点 → **future**，不在 MVP。

## 7. 不变性（design 约束）

1. **file-only**：subscriber 的 writer 永远是文件句柄，**永不** `std::io::stdout()`/`stderr`（守 TUI 不毁屏）。
2. **不碰 CliArgs、不加 settings 字段**：激活纯靠 env（决策 D1）。
3. **不在 `agent/` 层 init subscriber**：init 只在组合根 `app::cli`；`agent/` 只 emit（守 `arch_guard::agent_does_not_import_infra`，tracing 是 leaf crate 不违反分层，但 init 带订阅器属 app 职责）。
4. **同步写**：不引 `non_blocking`（决策 D3）。
5. **未装即 no-op**：生产默认 off，零性能影响。

## 8. 显式不做（→ future.md）

- ❌ CLI `--debug` flag、`--log-level`/`--log-path`（决策 D1，env-only）
- ❌ `settings.json` 的 `logging.{level,path}` 字段
- ❌ 按日滚动 / 大小滚动（`tracing-appender::RollingFileAppender`）
- ❌ non-blocking writer（`tracing-appender::non_blocking`）
- ❌ TUI 内 debug 日志面板 / `:messages` 查看器（参考 codex `log_db` SQLite Layer 模式）
- ❌ render seam / mpsc 流控 / stream 状态机的细粒度埋点
- ❌ 即时快照命令（pi 的 `/debug`）

## 9. 验证

- `just qa` 全绿（fmt/clippy/test/docs/arch_guard）。
- 手测：`RUST_LOG=debug cargo run -- tui` → 不毁屏，`~/.xylitol/logs/xylitol.log` 出现内容。
- 手测：`cargo run -- tui`（无 env）→ 行为与现状完全一致，log 文件不创建或为空。
- 回归：现有 print/server 行为不变（subscriber off 时 tracing 宏 no-op）。

## 10. 参考

- codex `tui/src/lib.rs:1138-1258`（file layer + `with_ansi(false)` + `RUST_LOG` + `try_init`）、`core/src/config/mod.rs:3633-3637`（log_dir 默认）
- pi `tui/src/terminal.ts:111-124,454-462`（file-only tee）、`tui/src/tui.ts:1327-1333`（redraw log）
- ratatui 官方 recipe https://ratatui.rs/recipes/apps/log-with-tracing/
- 社区共识 https://forum.ratatui.rs/t/how-do-you-println-debug-your-tui-programs/66
