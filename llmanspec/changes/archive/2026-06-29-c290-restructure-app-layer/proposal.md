---
id: c290-restructure-app-layer
depends_on:
  - c285-refactor-domain-runtime-protocol-boundary
---

# c290-restructure-app-layer

## Why

当前单 crate 的 `src/` 布局在 c285 之后又出现了新的结构张力：

1. `src/interactive/` 名不副实——它同时承载 CLI composition root、stdio RPC transport、打印模式、TUI diff review，未来还要放 GUI。
2. `src/server/` 与 `src/interactive/cli/` 本质上是平级的“应用入口”，却被放在顶层，和 `agent`/`infra` 并列，模糊了“runtime core”与“app surface”的边界。
3. `src/protocol.rs` 是单文件，REST/WS/Envelope 等 future transport 扩展没有干净的子模块位置。
4. `interactive/cli`、`interactive/rpc`、`server` 各自重复写 `Agent::with_ports(...)` 的 wiring 样板。
5. TUI/Server/GUI 未来会引入重依赖（ratatui/crossterm、axum、tauri），单 crate 下无法按需裁剪编译。

本变更在**不引入 Cargo workspace**（避免 build artifact 膨胀）的前提下，把 `interactive` 升级为 `app` 层，把 `server` 收进 `app/server`，把 `protocol.rs` 拆成 `protocol/` 目录，并提取统一的 `app::composition` 来构造 `Agent`。

## What Changes

### P0 — 创建 `app/` 层，合并 `interactive/` 与 `server/`

- `git mv src/interactive src/app`
- `git mv src/server src/app/server`
- 更新 `src/lib.rs`：
  - `pub mod interactive;` → `pub mod app;`
  - `pub mod server;` 移除（由 `app::server` 提供）
- 全局替换 `crate::interactive::` → `crate::app::`
- 全局替换 `crate::server::` → `crate::app::server::`

新布局：

```text
src/app/
├── mod.rs
├── driver.rs              # Driver trait + InProcessDriver
├── composition.rs         # 统一 build_agent / build_runtime
├── print.rs               # print mode
├── rpc.rs                 # stdio RPC transport
├── cli/                   # CLI composition root
│   ├── mod.rs
│   ├── args.rs
│   ├── provider_guidance.rs
│   └── run.rs
├── server/                # always-on server app
│   ├── mod.rs
│   ├── runtime.rs
│   ├── rest.rs
│   ├── ws.rs
│   └── lock.rs
├── tui/                   # future TUI
│   └── diff_review/
└── gui.rs                 # future GUI placeholder
```

### P1 — `protocol.rs` → `protocol/`

- 创建 `src/protocol/mod.rs`，导出公共符号。
- 创建 `src/protocol/command.rs`，移入 `Command` 枚举。
- 创建 `src/protocol/event.rs`，移入 `Event` 枚举。
- 创建 `src/protocol/transport.rs`，放置 JSONL/WS/REST 通用辅助（目前可留空或只放 `Envelope`/`Frame` 等未来共享结构）。
- 删除 `src/protocol.rs`。
- 更新所有 `crate::protocol::Command` / `crate::protocol::Event` 引用（路径不变）。

### P2 — 提取 `app::composition`

- 新建 `src/app/composition.rs`。
- 提供 `build_agent(options: BuildAgentOptions) -> Agent`，统一完成：
  - 加载 config
  - 构造 `ModelRegistry`
  - 注入 `InfraBashExecutor`、`EventBus`、`SessionManager`、`StdExportIo`
  - 调用 `Agent::with_ports(...)`
- CLI、RPC、Server、TUI 全部改为调用 `app::composition::build_agent(...)`，删除各自重复的 wiring 代码。
- `BuildAgentOptions` 放在 `src/app/composition.rs` 或 `src/app/types.rs`。

### P3 — Feature-gate 应用入口

更新根 `Cargo.toml`：

```toml
[features]
default = ["cli"]
cli = []
rpc = []          # stdio RPC mode，可与 cli 合并或独立
server = ["dep:axum", "dep:tokio-tungstenite", "dep:tower-http"]
tui = ["dep:ratatui", "dep:crossterm"]
gui = []
```

- `src/app/mod.rs` 按 feature 暴露子模块：
  - `pub mod cli;` → `#[cfg(feature = "cli")] pub mod cli;`
  - `pub mod server;` → `#[cfg(feature = "server")] pub mod server;`
  - `pub mod tui;` → `#[cfg(feature = "tui")] pub mod tui;`
  - `pub mod gui;` → `#[cfg(feature = "gui")] pub mod gui;`
- 把原 `ui-review` feature 改名为 `tui`。
- `src/app/tui/diff_review/` 只在 `tui` feature 下编译。
- `src/app/server/` 只在 `server` feature 下编译。

### P4 — 更新架构守卫与文档

- `src/tests.rs::arch_guard`：
  - 扫描路径 `src/app/` 替代 `src/interactive/`。
  - 允许 import `crate::agent` / `crate::infra` 的 composition root 列表更新为：
    - `app/cli/`
    - `app/server/`
    - `app/rpc.rs`
    - `app/composition.rs`
    - `app/print.rs`（只读流匹配，可保留例外）
  - 禁止 `app/driver.rs` 直接 import `crate::infra`。
- `AGENTS.md` 项目结构描述更新为 `app/` 层说明。
- 更新 active specs 中涉及 `interactive/`、`server/` 路径的 requirement/scenario。

## Capabilities

- `architecture` (modify): 更新模块地图，说明 `app/` 层与 `protocol/` 目录。
- `layer-architecture` (modify): 更新 la2/la4/la6/la9/la11，把 `interactive/` 改为 `app/`，把 `server/` 改为 `app::server`。
- `app-client` (modify): 重命名/更新为 `app-client` 语义，说明 CLI/RPC/print 都位于 `app/`。
- `app-protocol` (modify): 更新路径引用，`interactive/rpc.rs` 改为 `app/rpc.rs`。
- `server-runtime` (modify): 更新 server 模块位置为 `app/server/`，composition root 必须注入 `StdExportIo` 并复用 `app::composition`。
- `workspace-structure` (modify): 更新源码树布局说明，强调单 crate + feature gate。

## Impact

- **行为**：零运行时行为变化；纯结构重构。
- **编译**：大规模 import 路径重写；feature gate 改变默认构建产物。
- **测试**：所有 lib tests、BDD scenarios、`arch_guard` 必须保持绿色；需要在多种 feature 组合下验证（`--features cli`、`--features server`、`--features tui`）。
- **文档**：`AGENTS.md` 与 active specs 更新；archived specs 保持冻结。
- **风险**：中。机械性改动多，但编译器和 arch_guard 会暴露所有遗漏。
