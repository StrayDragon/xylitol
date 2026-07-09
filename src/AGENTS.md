# src/ 分层架构（代码架构 SSOT）

本文件是 `src/` **代码架构**的单一真值源：分层不变量、各层职责、应用面状态、seam。全局工作方式与「如何写 AGENTS」见根 `AGENTS.md`。子目录 `AGENTS.md` 与 skills 只引用本文件，不重复长文。

`xylitol` 主 crate：薄编排（`agent/`）+ 运行时域（`infra/`）+ 可插拔应用面（`app/`）+ 线协议（`protocol/`）+ 领域词（`domain/`）+ ports（`runtime_protocol/`）。通用 TUI 库在 workspace 包 `packages/xylitol-tui`（不在本文件展开）。

## 分层不变量（normative）

由 `src/tests.rs::arch_guard` 强制（代码是真值）：

- **组合根集中装配**：仅 `app/core/composition.rs` 与次级组合根 `app/cli/mod.rs`、`app/server/subcommand.rs` 可同时 import `agent` 与 `infra`。
- **agent 不依赖 infra**；**infra 不依赖 agent**。
- **domain** 零 crate 内依赖；**runtime_protocol** 只依赖 `domain`。
- **应用面走 seam、不 reach 内部**：禁止 `agent::session::*` / `agent::runtime::*` / `infra::*`；只从 `crate::agent`（mod 级）与 `crate::app::core` import。共享 seam：`composition::build_agent` → `Driver::run(prompt)` → `XyEvent` 流 → 该面渲染；不够就扩 seam，不绕过。方法论：`write-surface` skill。

```text
app → agent → runtime_protocol → domain
  ↓     ↑
  └──── infra ───────────────────┘
protocol ───────────────────────→ domain
```

## 各层职责（摘要）

- `domain/` — 纯领域词汇与 `XyEvent` 等；零内部依赖。
- `runtime_protocol/` — agent↔infra ports（`XyModel`/`XyTool`/`XySessionStore`/…）。
- `infra/` — ports 的实现（provider、tools、session、config、…）。
- `agent/` — ReAct / session / model / tools 编排；公共入口为 mod 级 re-export。
- `protocol/` — `Command` / `Event` 线协议，传输无关。
- `app/` — 应用面 + `core/` seam。状态：`cli/print` ✅；`server/` 🟡；`tui/` 🟡（占位，基于 `xylitol-tui` 重做，见 `src/app/tui/AGENTS.md`）；`gui` 🔴。跨面：`core/{bootstrap,dispatch,composition,driver}`。落地顺序 print → server → TUI，禁止并行铺骨架。

模块级文件地图以目录与代码为准；本文件不维护易变文件清单。

## 跨层测试与守卫

- `arch_guard`；BDD（`tests/features` + `tests/bdd.rs`）；回归（`tests/regression/`）。
- 设计史：`llmanspec/changes/archive/<变更>/design.md`。
