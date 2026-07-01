# c325 Design — TUI 架构决策记录（ADR）

本变更不写代码，只把 2026-07 的 TUI 架构决策及其**支撑证据**沉淀为规范。目的是防止第四次重蹈覆辙——前两次失败部分源于决策未被记录，每次重来都要重新论证且结论易丢。

## 决策 1：渲染模式 = inline（非 alt-screen）

**决策**：TUI 用 ratatui `Viewport::Inline(N)` + `Terminal::insert_before`，不进 alternate screen。

**理由**：
- 历史留在终端原生 scrollback，用户用熟悉的选区/鼠标/滚轮复制，零学习成本。alt-screen 退出后历史消失，且要自建内部 pager + 选区→剪贴板才能选到 buffer 外内容，工作量大。
- 两个对标项目都选 inline：pi 全程无 `?1049h`（仅外部编辑器清理处出现）；codex 同理。

**可行性证据**（这是「第三次能成」的关键）：
- ratatui 0.30.2 的 `Terminal::insert_before(height, draw_fn)` 是**稳定、公开、无 feature gate** 的 API。文档原文：*"If more lines are inserted than there is space on the screen, then the top lines will go directly into the terminal's scrollback buffer."*
- 源码：`~/.cargo/registry/.../ratatui-core-0.1.2/src/terminal/inline.rs:109`。
- `Viewport::Inline(N)` + `try_init_with_options(...)` 启用 raw mode（键盘所需）但**不进** alt screen。
- `scrolling-regions`（减闪烁优化）是可选 feature，有稳定 fallback，MVP 不需要。
- ratatui issue #1426（status TUI + 结果向上滚动）正是 `insert_before` 的设计场景。

**前两次失败的根因消解**：inline 之前意味着「自写差分渲染器」（pi 那套 JS 才需要），现在 ratatui 原生解决了 stable-region + mutable-tail，无需手搓 `\x1b[2K` 差分。

## 决策 2：连接模型 = in-process 默认（非 daemon 客户端）

**决策**：TUI 经 `app::core::driver::InProcessDriver`，与 print 模式同路径。不默认骑 RemoteDriver/auto-spawn daemon。

**理由**：
- 两个对标项目的 TUI 都是 in-process：
  - codex：`AppServerTarget::Embedded` 是 TUI 默认（`codex-rs/tui/src/lib.rs:814-830`）。仅显式 `--remote` 或 50ms 内探测到已运行 daemon 才走 socket。in-process 用 `InProcessAppServerClient`（类型化内存通道，无 IPC）。
  - pi：`InteractiveMode` 构造函数直接接收 `AgentSessionRuntime`，调 `session.prompt(...)`（`coding-agent/src/modes/interactive/interactive-mode.ts:265,391`），从不走网络。
- in-process 把 TUI 风险压到与 print 同级（print 已验证在用 `InProcessDriver`）。让 TUI 骑未验证的 RemoteDriver+WS（当前零集成测试）会重蹈「并行铺骨架」反模式。

**被否决的方案**：daemon 统一架构（TUI 默认骑 auto-spawn daemon + 砍 rpc）。与 codex/pi 都相悖，且把 TUI 绑在未验证设施上。

## 决策 3：rpc / server / tui 三面并存（不砍任何一个）

**决策**：rpc（stdio JSONL）、server（REST+WS）、tui（inline）三种应用面并存保留。

**理由**：codex 和 pi **都同时保留**三种面，无一砍掉：
- codex：in-process TUI + `app-server --listen stdio://`（驱动 VSCode 扩展，生产一等公民）+ `app-server-daemon`（unix socket，远程/移动/多会话）。ws 标 experimental 但未移除。
- pi：in-process TUI + `--mode rpc`（自定义 JSONL，嵌入/IDE）+ `orchestrator serve`（unix socket，监督多 rpc 子进程）。

rpc（嵌入/编辑器插件）与 server（多会话/远程）是**互补**而非冗余。砍任一面都丢掉一类用户。

**真正的统一工作是协议而非面**：让 `server/ws.rs` 靠拢 `protocol::Command/Event` SSOT（c345），而非砍面。

## 决策 4：复用契约（TUI 专属硬约束）

TUI 是 `app/` 的一个面，受 write-surface 复用契约约束：
- 输入采集 / 渲染：TUI 内部重写（crossterm + ratatui）。
- 驱动 agent：**只能**经 `Driver`（`InProcessDriver`）。
- 构造 agent：**只能**经 `composition::build_agent`。
- slash 命令：复用 `protocol::Command` 语义，不另造一套。
- 禁止 import `agent::session`/`agent::runtime`/`infra`。

## 决策 5：不铺空骨架

每个 `src/app/tui/` 下文件必须当其所属变更内即被 `run()` 真实驱动，禁止 `#[allow(dead_code)]` 让骨架先过编译。这是前两次失败的直接根因之一（write-surface 铁律）。

## 风险

- **纯规范变更，风险极低**。无代码、无回归。
- 唯一风险是「规范写得不够具体，后续被钻空子降级」——故每条 MUST 都带证据来源（file 事实 / 版本事实），反降级护栏具体化。
