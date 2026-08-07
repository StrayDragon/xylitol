---
name: "write-tui"
description: >-
  Write or redesign xylitol product TUI under src/app/tui/ on packages/xylitol-tui
  (XyDriver + XyEvent seam, host-driven sync engine, no agent/infra reach-in). Use
  when changing the app TUI surface, slash/theme/host loop, or wiring xylitol-tui
  into the CLI — not for generic component work inside packages/xylitol-tui alone
  (use test-tui-harness / package AGENTS for that).
---

# 编写 TUI（src/app/tui/）

写/改前读：`src/app/tui/AGENTS.md`（**先读 File Layout / Module Responsibilities**）→ `packages/xylitol-tui/AGENTS.md` → `src/AGENTS.md`。方法论总纲：`write-surface`。包内测试：`test-tui-harness`。

旧实现已删除；本面基于 `xylitol-tui` **从零重做**。

**开闸后**：可按 `c465` 等 Track B change 扩展 `src/app/tui`。**仍 STOP**：在 **c491 stub** 上加活树 / filter / 真 XyDriver travel；缺通用能力先改 `agent_demo` / `packages/xylitol-tui`。

## 1. 复用契约

- **渲染/组件/键协议**：只用 `xylitol_tui`。产品面 host 驱动（`dispatch_input` / `request_render` / `try_render` / `idle_tick`）；勿在产品路径调 `TUI::start()`。
- **事件合流**：本面异步 host（如 `tokio::select!`）；不把 tokio 绑进 `xylitol-tui`。
- **Agent**：只经 `app/core/driver::XyDriver` 与 `composition::build_agent`；禁止 `agent::capabilities` / `runtime` / `infra`。
- **slash**：本面解析；执行经 `app/core/dispatch` + `protocol::Command`。

## 2. 分工

| `packages/xylitol-tui` | `src/app/tui/` |
|---|---|
| 引擎、键协议、通用组件 | App Shell、UX 状态机、slash |
| 闭包 theme 接口 | 语义 token → 闭包 |
| 五层测试 1–4（见 `test-tui-harness`） | XyDriver / `XyEvent`→UI |

## 3. 落点

- 新 `XyEvent` 呈现 → 本面状态更新 → 驱动组件。
- 通用 widget → 优先 `packages/xylitol-tui`（先查是否已有）。
- 新 agent 行为 → 不进 TUI。

文件布局随重做落地；以当时 `src/app/tui/AGENTS.md` 为准。

## 4. 验证

- 通用组件/引擎：`test-tui-harness`。
- 本面 seam/slash：就近 `#[cfg(test)]`，优先扩既有文件。
- 提交前：`just qa`；无违规 reach-in；无「暂用」`#[allow(dead_code)]`（见 `audit-dead-code`）。

## 5. Debug

禁止 `println!`/`eprintln!`/`dbg!`。`XYLITOL_DEBUG=1` 或 `RUST_LOG=…` → `~/.xylitol/logs/xylitol.log`（`app/cli/logging.rs` 装配）。埋点只用 `tracing::`，`target: "xylitol::tui"`。
