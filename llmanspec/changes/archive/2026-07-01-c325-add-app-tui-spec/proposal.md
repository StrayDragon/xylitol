---
change_id: c325-add-app-tui-spec
title: 沉淀 TUI 应用面架构决策（渲染模式 / 连接模型 / 三面并存）为规范
status: proposed
priority: 325
depends_on: []
author: agent
---

# c325-add-app-tui-spec

## Why

xylitol 的 TUI 应用面此前两次尝试落地均失败，根因之一是**架构决策未被沉淀为规范**——每次重来都要重新论证「inline 还是 alt-screen」「TUI 是否骑 daemon」「rpc 和 server 去留」，论证过程反复消耗且结论易丢失。

本变更把本次（2026-07）经充分对标调研后确定的 TUI 架构决策，连同支撑证据，写入 `llmanspec/specs/app-tui/` 规范。它的首要目的不是写代码，而是**为后续所有 TUI 相关变更（c340 主体、c350 集成等）提供不可绕过的依据**，并内置反降级护栏：任何后续变更若把 TUI 改回 alt-screen、或让 TUI 默认骑 daemon、或砍掉 rpc/server 任一面，都将在规范层面产生冲突，从而被迫在提案阶段就显式说明理由，而非在实现时悄悄退化。

### 关键调研证据（决策的事实基础，防止「我觉得」式降级）

**证据 A —— inline 渲染在 ratatui 0.30.2 是一等公民（稳定、非 hack）**
- `Terminal::insert_before(height, draw_fn)` 是 ratatui 0.30.2 的稳定公开 API，无 feature gate。文档原文：*"If more lines are inserted than there is space on the screen, then the top lines will go directly into the terminal's scrollback buffer."* 这正是「把完成的行提交到 scrollback、只重绘底部当前轮」所需的语义。
- `Viewport::Inline(N)` + `try_init_with_options(...)` 启用 raw mode（键盘所需）但**不进** alternate screen。
- 源码事实：`~/.cargo/registry/.../ratatui-core-0.1.2/src/terminal/inline.rs:109`。
- `scrolling-regions`（减闪烁优化）是可选 feature，有稳定 fallback，MVP **不需要**。

**证据 B —— 两个对标项目的 TUI 都是 in-process，不骑 daemon**
- codex：`AppServerTarget::Embedded` 是 TUI 默认（`codex-rs/tui/src/lib.rs:814-830`）。只有显式 `--remote` 或 50ms 内探测到已运行 daemon 才走 socket。in-process 路径用 `InProcessAppServerClient`（类型化内存通道，无 IPC）。
- pi：`InteractiveMode` 构造函数直接接收 `AgentSessionRuntime`，调用 `session.prompt(...)`（`coding-agent/src/modes/interactive/interactive-mode.ts:265,391`）。从不走网络。
- 结论：**xylitol TUI 默认经 `InProcessDriver`，与 print 模式同路径**，是对标项目的共同做法。

**证据 C —— rpc 和 server 三面并存，不砍任何一个**
- codex：同时保留 in-process TUI + `app-server --listen stdio://`（驱动 VSCode 扩展，生产一等公民）+ `app-server-daemon`（unix socket，远程/移动/多会话）。ws 标记 experimental 但未移除。
- pi：同时保留 in-process TUI + `--mode rpc`（自定义 JSONL，嵌入/IDE）+ `orchestrator serve`（unix socket，监督多个 rpc 子进程）。
- 结论：rpc（嵌入/编辑器插件）与 server（多会话/远程）是**互补**而非冗余。砍任一面都丢掉一类用户。xylitol 的 `rpc.rs` 与 `server/` 都保留。

**证据 D —— pi 的 TUI 是 inline（不进 alt screen），印证 inline 选择**
- pi 全程无 `?1049h`（grep `packages/tui/src` + `modes/interactive` 仅在外部编辑器清理处出现 alt-screen 引用，自身渲染不用）。历史留在终端原生 scrollback，用户用熟悉的方式复制。codex 同理。

## What Changes

新增 capability spec `app-tui`（`llmanspec/specs/app-tui/spec.toon`），声明以下 MUST/SHALL 约束（反降级护栏）：

1. **渲染模式 = inline**：TUI MUST 用 `Viewport::Inline`，MUST NOT 进 alternate screen（除非未来以独立变更显式推翻并附理由）。
2. **连接模型 = in-process 默认**：TUI MUST 经 `app::core::driver::InProcessDriver`，MUST NOT 默认骑 daemon / RemoteDriver。
3. **三面并存**：rpc、server、tui 三种应用面 MUST 并存保留；禁止以「统一」为名砍掉 rpc 或 server。
4. **复用契约**：TUI MUST 仅 import `crate::agent`（mod 级）+ `crate::app::core`（Driver/composition）+ `crate::protocol` + `crate::domain`；MUST NOT reach into `agent::session`/`agent::runtime`/`infra`。slash 命令语义 MUST 复用 `protocol::Command`。
5. **不铺空骨架**：TUI 下每个文件 MUST 当其所属变更内即被真实入口驱动，MUST NOT 用 `#[allow(dead_code)]` 让骨架「先过编译」。

注：本变更**只写规范**，不写 TUI 代码。TUI 主体落地是 c340 的职责。

## Capabilities

- `app-tui`（新增）：TUI 应用面的架构约束与反降级护栏。

## Impact

- **受影响代码**：无直接代码改动；为 c340/c350 等后续变更提供规范依据。
- **受影响规范**：新增 `specs/app-tui/`；与 `cli-entry`、`print-output`、`layer-architecture` 并列（TUI 是 app 层的第三个应用面）。
- **风险**：低。纯规范变更，但它是防止 TUI 第四次重蹈覆辙的认知护栏。

## 反降级护栏（为本变更专属设计的自查清单）

- [ ] `app-tui` spec 包含上述 5 条 MUST/SHALL，且每条带证据来源（ratatui 版本/codex/pi 的 file 事实）。
- [ ] `llman sdd validate c325-add-app-tui-spec --strict` 通过。
- [ ] 后续任何 TUI 相关变更若违反上述 MUST，在 proposal 阶段即被识别为潜在冲突。
