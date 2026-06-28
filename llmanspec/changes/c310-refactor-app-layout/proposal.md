---
id: c310-refactor-app-layout
depends_on: []
---

# c310-refactor-app-layout

## Why

c290 把 `interactive/` 升级为 `app/` 层后，`app/` 顶层是扁平结构，但扁平里混了两类职责：

1. **seam（跨 surface 共享）**：`composition.rs`（HC-1 组合根，唯一允许同时 import agent+infra）、`driver.rs`（`Driver` 边界，surface 经它与 agent 交互）。
2. **surface（自洽入口）**：`cli/`、`rpc.rs`、`server/`、`tui/`、`gui.rs`。

seam 和 surface 平铺在一起，读 `app/mod.rs` 无法一眼分辨"谁是被多 surface 共享的特权模块、谁是独立入口"。此外还有两处**分类错位**：

- **A. `print.rs` 放错层**：它不是和 `rpc`/`server`/`tui` 同级的独立 surface，而是 **CLI 的默认渲染模式**（CLI 解析参数后 dispatch 到 `rpc` / `server` / `print` 之一）。全仓唯一调用方是 `cli/mod.rs:476`，签名 `&mut dyn Driver`（driver 由 CLI 构造后喂给它）。它和 `cli/provider_guidance.rs`（"pure CLI-surface presentation"）是同类，却滞留在顶层。
- **B. `ServerSubcommand` + `run_server` 寄生在 `cli/mod.rs`**：`ServerSubcommand` enum 及其 `run_server` 实现（含 lock-file 探测 + `kill -TERM` 的 server 生命周期逻辑）写在 CLI 里。这些是 server 域逻辑，本应与 `server/lock.rs` 同域。把它留在 cli 还会导致依赖倒流——若实现挪进 server，server 反而要反向依赖 cli 拿 enum 类型。
- **C. seam 无分组**：`composition.rs` / `driver.rs` 没有共同归属，职责（构造根 vs 运行时边界）只能靠文件名各自表达，缺一个显式的"这是 app 层共享基础设施"的视觉与结构信号。

本变更通过**纯结构重构**把三类各归其位，引入 `app/core/` 承载 seam，使 seam/surface 边界显式化。

## What Changes

目标布局：

```text
src/app/
├── cli/                   # CLI 入口（surface）
│   ├── mod.rs
│   ├── print.rs           # ← (A) 从 app/print.rs 移入：CLI 默认渲染模式
│   ├── provider_guidance.rs
│   └── resources.rs
├── rpc.rs                 # stdio RPC transport（surface）
├── server/                # always-on server（surface）
│   ├── mod.rs
│   ├── lock.rs
│   ├── port_retry.rs
│   ├── rest.rs
│   ├── runtime.rs
│   ├── subcommand.rs      # ← (B) 从 cli/mod.rs 移入：ServerSubcommand + run_subcommand
│   └── ws.rs
├── tui/                   # TUI（surface）
├── gui.rs                 # GUI 占位（surface）
├── core/                  # ← (C) 新建：跨 surface 共享 seam
│   ├── mod.rs
│   ├── composition.rs     # 从 app/composition.rs 移入：HC-1 组合根
│   └── driver.rs          # 从 app/driver.rs 移入：Driver trait + InProcess/Remote
└── mod.rs
```

### A — `print.rs` → `cli/print.rs`

- `git mv src/app/print.rs src/app/cli/print.rs`
- `cli/mod.rs` 加 `mod print;`，调用点 `crate::app::print::run_print` → `crate::app::cli::print::run_print`
- `app/mod.rs` 删 `pub mod print;`（顺带修正其原本缺少 `#[cfg(feature="cli")]` 门控的缺陷——移入 cli 后自然继承门控）

### B — `ServerSubcommand` + `run_server` → `server/subcommand.rs`

- `cli/mod.rs` 里的 `ServerSubcommand` enum、`run_server` fn 迁入 `src/app/server/subcommand.rs`（feature-gated `server`）
- `CliCommand::Server` 变体保留在 cli（CLI 解析职责），但其 dispatch 缩为一行：`Some(Server{action}) => crate::app::server::subcommand::run(action).await`
- `Ce8` scenario 行为不变（server stop 仍 SIGTERM + 清理 lock）

### C — `composition.rs` + `driver.rs` → `core/`

- 新建 `src/app/core/{mod.rs,composition.rs,driver.rs}`
- `git mv` 两个文件进 core/
- `app/mod.rs`：`pub mod composition;` `pub mod driver;` → `pub(crate) mod core;`
- 全局路径迁移：`crate::app::composition` → `crate::app::core::composition`；`crate::app::driver` → `crate::app::core::driver`（涉及 `cli/mod.rs`、`rpc.rs`、`server/runtime.rs`、`print.rs`、`driver.rs` 自身对 `server::ws` 的引用）
- 不保留 `pub use` 兼容 shim（遵循 AGENTS.md：一次性更新所有调用点）

### 配套：arch_guard 测试代码同步

`src/tests.rs::arch_guard`：
- `exempt_prefixes` 增加 `"core/"`；`exempt_files` 移除 `"driver.rs"`、`"composition.rs"`、`"print.rs"`（print 已在 `cli/` 前缀豁免内；composition/driver 在 `core/` 前缀豁免内）
- `app_driver_does_not_import_infra` 扫描路径 `src/app/driver.rs` → `src/app/core/driver.rs`
- 更新守卫注释里的文件地图

### 配套：文档

- `AGENTS.md` 项目结构描述：把 composition/driver 归入 `core/`，移除顶层 `print.rs`
- 修正 `cli-entry::ce1` 的既有漂移（spec 曾写 `src/app/print/` 且"MUST NOT be named app/"，与现状矛盾）

## Capabilities

全部为 modify（路径/结构约束的更新）；layer-architecture 额外 add 一条 codify seam/surface 分离的新 requirement：

- `layer-architecture` (modify la2, la4, la9, la11, la15；add la19)：更新 composition-root 豁免清单（去 print、composition→core/composition、driver→core/driver），新增 la19 显式 codify `app/core/` 承载 seam
- `workspace-structure` (modify r1, r4)：更新 app 层布局描述与 build_agent 路径
- `architecture` (modify ar05)：更新模块地图
- `app-client` (modify ic2, ic4, ic6, ic7)：InProcessDriver 路径、print 路径、composition 路径
- `cli-entry` (modify ce1)：修正 print 归属与既有漂移
- `server-runtime` (modify sr10)：composition 路径

## Impact

- **行为**：零运行时行为变化；纯结构重构。
- **编译**：~6 处 `use crate::app::{composition,driver,print}` 路径重写；新增 `core/` 与 `cli/print.rs`、`server/subcommand.rs` 模块声明。
- **测试**：`src/tests.rs::arch_guard` 必须同步更新（否则 `exempt_files`/扫描路径失配导致守卫误报或失效）；所有 lib tests + BDD + arch_guard 必须保持绿色。
- **文档**：`AGENTS.md` 与上述 6 个 active specs 更新；archived specs 保持冻结。
- **风险**：低-中。机械性改动为主，编译器和 arch_guard 会暴露所有遗漏；最需小心的是 arch_guard 豁免清单与扫描路径的精确同步。
