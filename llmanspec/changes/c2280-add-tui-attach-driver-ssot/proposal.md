---
depends_on: []
---

# 双模 TUI：Driver SSOT 与本机 host 传输选型（研究草案）

> **一句话**：产品保留「默认进程内 TUI」与「显式 `xylitol serve` 后 attach」两条进线；用同一套 TUI host + `XyDriver` 符合性闸把漂移压住；本票只做固定深度调研与 SSOT 设计，不实现。
> **产品进线（已钉）**：先 `xylitol serve`（不做 Ensure-running / 第一扇窗兼 host）；习惯用窗口管理器、多开终端的用户可用会话/autostart 起 host。默认仍原生 TUI。

## Why

习惯用窗口管理器、经常多开终端的用户（例如 i3 / sway / Hyprland，或 tmux 多窗格）会开多扇 TUI：多仓各开 + 同仓多 session。痛点是 RSS、MCP stdio 被每窗 clone、冷启动重复 bootstrap。现有 Server / `XyRemoteDriver` 是缝，但 Server 仍是一把 `Mutex<XyInProcessDriver>`、REST 路径里的 `session_id` 大量未真正路由，TUI 从不 attach。拉起 serve 的习惯见 [`research/04-topology.md`](./research/04-topology.md)「窗口管理器用户怎么拉起 serve」。

同时决定 **双模式并存**（默认原生，serve 起来再 attach），而不是「产品 TUI 只 attach」。双模式的真实风险不是「多一个 CLI 旗标」，而是 host/effects/MCP/bang 走出第二套语义。本票要把 **代码 SSOT 与符合性闸** 写死，并调研 **本机 host 该不该换框架**（现成 axum+WS vs UDS 上的 Command/Event vs JSON-RPC），避免实现阶段临时选一个和未来 Web 打架的载体。

对照：`deepseek-harness` 是「一个 host、多种载体、协议与通道分离」；LSP stdio 按 `(server, canonical workspace)` 池化。xylitol 禁止抄插件市场 / 删 TUI；要抄的是进程内路由与载体分离。

## What Changes（本票交付 = 研究，不是实现）

1. 把探索结论收成可引用的 change 文档（本 proposal + `research/`）。
2. 钉死双模 **代码 SSOT**（见下）与符合性闸形态，供后续实现票 `depends_on`。
3. 固定深度调研本机 host 传输/框架：候选、否决理由、与现有 `protocol::Command`/`Event` 的关系。
4. **不**改 live specs、**不**接线 TUI attach、**不**改 Server 运行时。实现另开 change（建议 `c2290+`）。

## 已钉产品决策（探索）

| ID | 钉死 |
|---|---|
| P1 | 场景：多仓各开 **且** 同仓多 session |
| P2 | 痛点：RSS / MCP clone / 启动税；**不**把同仓写冲突当 v1 目标 |
| P3 | Host 启动：**显式 `xylitol serve`**（B）。不做第一 client Ensure-running，不做 TUI 兼 host |
| P4 | 产品 TUI **双模式**：默认进程内；serve 后显式 attach。接受漂移风险，用 SSOT+闸压 |
| P5 | 关某一扇 TUI 不得杀 host；停 host 则全体 attach 掉线 |
| P6 | 剪贴板 / TTY / `$EDITOR` **留在 TUI 进程**。**bang（`!`）在工作区执行**（server / sandbox），与旧稿「bang 永远留 TUI」不同——E 下 `!ls` 列的是工作区不是笔记本。agent 工具 bash 同样在工作区；现在 `execute_bash` 兼着 bang，拆开时会大改。 |

## 双模代码 SSOT（设计意向，供调研证伪）

目标：用户看见两条进线，仓库只养 **一套 TUI + 一套驱动合约**。

```text
                    ┌─ 面本地（两模式共用，禁止经 Driver）
                    │    clipboard / $EDITOR / TTY / 键位 / paint
xylitol TUI host ───┤
                    └─ dyn XyDriver        ← 唯一 Agent 面
                          ├─ XyInProcessDriver   默认；MCP 在本进程
                          └─ XyRemoteDriver      attach；MCP 在 host 池
                                │
                         protocol Command/Event + dispatch
                                │
                         xylitol serve（用户级一把锁 + 多 Runtime）
```

**硬规则（实现票 MUST 落地，本票只钉）：**

1. **一个 TUI host**（`src/app/tui` 的 host/effects/bridge/layout）。禁止 `tui_remote/` 第二套循环。
2. **Agent 能力只经 `XyDriver` + `dispatch`。** 新 slash/steer/session 先加 trait 方法与 dispatch 臂，再两实现；禁止 InProcess 走捷径、Remote 返回未实现。
3. **面本地能力不进 Driver。** InProcess 里若还夹着 clipboard / TTY，attach 路径会缺一块——实现前拆到 TUI 侧协作器。**bang（`!`）走 Driver，在工作区执行**（attach 时经 wire protocol 到 server）。
4. **符合性闸（SSOT 的牙齿）**：同一张 Command 表 + 同一组可观察 Event，对 InProcess（harness）与「测试 host + RemoteDriver」各跑一遍。Remote 对已承诺命令返回未实现 = 闸红。现有 `sr-remote1` 是合约锚，缺的是 **双实现同跑**。
5. **MCP**：池化只发生在 serve 进程，key 意向 `(mcp_name, canonical cwd)`（对齐 DSH LSP）。原生模式仍每进程一份——这是双模式付的税，文档写明，不要用「自动探测 serve 就切换」偷偷改默认。
6. **进线显式**：`xylitol tui --attach`（或等价）连 host；**禁止**「探测到 socket 就静默切 Remote」（第三模式，bang/cwd/失败语义会鬼畜）。serve 未起 → attach 失败并提示 `xylitol serve`。
7. **Print / embed** 继续 InProcess，不经假 attach。

无法用 SSOT 消灭的差异（必须当产品差异写进以后的 spec，而不是代码分叉）：

| 差异 | 为什么合法 |
|---|---|
| 原生：无 host 也能开 | P4 |
| attach：第二窗不付 MCP 启动税 | 这是 attach 的收益 |
| attach：host 崩则所有 attach 窗断 | P5 |
| 原生：杀该进程即杀该 agent | 今天行为 |

## 非目标（本票）

- 实现 attach / 多 runtime / MCP 池 / UDS
- Ensure-running、systemd socket 激活、TUI 兼 host
- 同仓多 agent 写锁 / worktree 隔离（`c1770` 方向）
- Web 产品面（只要求传输选型 **不堵** 未来 Web 载体）
- 换 ReAct、拆 crate、上插件平台
- vsock / microVM 沙盒连法
- 多 server 编排（1 管理面 → N server）
- 换编码 / 换流模型（postcard / rkyv / tonic / ntex / 自写帧；本票只定 tagged JSON + axum）
- 默认切 nightly

## Capabilities（实现票才 landing；本票只引用）

- `server-core`（sr-remote1、单实例锁、journal、反向 RPC）
- `protocol-app`（Command/Event/dispatch）
- `layer-architecture`（la6/la-server-driver、server opt-in）
- `app-tui-host` / `agent-runtime` / `infra-mcp` / `agent-session-store`（s7 单写者）

## Impact

- 本票：仅 `llmanspec/changes/c2280-add-tui-attach-driver-ssot/`
- 后续实现票：TUI 进线、Server 多 runtime、锁从 `/tmp/xylitol-server.lock` 改为用户 runtime 目录、MCP 池 key

## Test seams（给实现票，本票不写测）

- Driver 符合性：InProcess harness ∪ 测试 host+Remote，同一场景表
- 面本地：剪贴板 / `$EDITOR` 在 `--attach` 下仍走 TUI 进程
- bang：`--attach` 下 `!ls` 在 host 工作区执行，输出经 Event（意向 `BashDelta`）回 TUI（不是 TUI 本机 cwd）
- 锁：第二 `xylitol serve` 失败；停 host 后 attach 窗可观察断开

## BashDelta 实现票种子（供后续实现 change；不是本票 apply）

产品行为（landing spec 时作 MUST 种子）：

- attach 下 `!` / `!!` 在 **server 工作区**执行。
- stdout/stderr 增量作为 Event 推同一条 WebSocket 订阅；TUI 画法与进程内 bang 区一致。
- 结束仍是 `Event::BashResult`；Esc → `Command::Abort` 杀这棵 bang 进程树（不是 agent Aborted 文案）。
- 模型 bash 工具继续走 `ToolStart` / `ToolExecutionUpdate` / `ToolEnd` —— **两条 API**（用户 `!` 与模型工具，不共用同一 Driver 方法语义）。

代码切口（实现时对着改）：`src/protocol/wire/event.rs` 加 `BashDelta`（wire 先定 UTF-8 文本，非法字节用 `infra/tools/bash.rs` 的替换策略）；`XyEvent`（lifecycle）加对应变体与 `to_wire_event` / `try_from` 往返；journal 在 bang 进行中 `append` 每条 `BashDelta`（是否只 replay 未完成 bang 由实现票定）；server `BangExecHandler::execute` 的 `chunk_tx` 接「写 journal + 广播 WebSocket」而非只走 REST 返回值；`XyRemoteDriver` 的 attach 路径弃用 REST bash，改订阅 Event 本地 `append_bash_chunk`（REST 留给 Print / 调试）；`effects/bang.rs` Remote 时 `chunk_rx` 从 Event 流来；harness 把现有 bang 测参数化 InProcess | 测试 host+Remote。

非目标：不换 HTTP 栈 / 不换 JSON-RPC；不设计多 server 编排；不以 Docker 为 `BashDelta` 前置（本机 serve 也要直播）。

## 研究结论（裁决与边界；research 只供中立事实，采纳/不采纳以本节为准）

**采纳：**

- **协议**：wire protocol 保持 **tagged JSON**（`#[serde(tag="type")]`），不换 JSON-RPC 信封。能力考察：`01`「四候选架构」。
- **HTTP 栈**：选 **axum 0.8**，listener 按拓扑换 TCP / UDS。poem/salvo 的 UDS 双听/权限 API 更完整，但对主拓扑连法无帮助，**不换**。能力考察：`01`「候选对照」。
- **主载体**：attach = **TCP loopback + Docker 发布端口 + WebSocket + tagged JSON**；UDS 仅「本机 Linux 无 Docker」的可选项。能力考察：`04`「三种连法」。
- **沙盒**：server 整进程进 Docker = **粗粒度沙盒**（隔离进程树 / 文件系统 / 网络命名空间；不是每工具 seccomp）。能力考察：`04`「粗粒度沙盒」。
- **bang 直播**：attach 下 `!` / `!!` 在 server 工作区执行、增量推送、Esc → `Abort` 杀 bang 进程树；Event 名 **`BashDelta`**（专给 bang，不复用 `ToolExecutionUpdate`）。产品行为种子见「BashDelta 实现票种子」；缺口事实见 `03`。
- **双模代码 SSOT**：一个 TUI host；Agent 能力只经 `XyDriver` + `dispatch`；面本地不进 Driver；符合性闸同表双跑；MCP 池只发生在 serve 进程（key 意向 `(mcp_name, canonical cwd)`）；进线显式。见上文「双模代码 SSOT」。
- **Web 壳**：默认 **React + Vite SPA**（静态 `dist`，同源连现有 axum WS）；传输保持浏览器可升级 WS。能力考察：`06`「候选对照」。
- **工具链**：默认 **1.97.1 stable**；postcard / rkyv / ntex-uring / sonic-rs 均不需 nightly；`portable_simd` 与 Cranelift 单独 spike。能力考察：`05`「nightly 议题」。

**不采纳 / 边界：**

- vsock（`AF_VSOCK`）不进 v1；UDS 仅同内核 Linux（Desktop / VM 不行）。
- 换编码 / 换流模型（postcard / rkyv / tonic / ntex / 自写帧）不作为本票载体。
- 多 server 编排（1 管理面 → N server）不是本票；本票只 N TUI ↔ 1 server。
- nightly 不作为主线默认。
- 浏览器面不开闸（只保证传输不堵未来 Web）。

**探索结论残留事实：** 多仓 RSS 下限仍是 N 套工具世界；attach 主要打**同仓第二扇窗**的 MCP / 启动；DSH 的 host ≠ TUI 父进程。

## 调研文档（中立事实，非裁决）

- 入口 / 术语表 / 调研地图：[`research/00-overview.md`](./research/00-overview.md)
- 框架与流模型能力对照：[`research/01-framework.md`](./research/01-framework.md)
- TUI 与 server 的能力归属切分：[`research/02-split.md`](./research/02-split.md)
- 消息路径与现状缺口：[`research/03-paths.md`](./research/03-paths.md)
- 拓扑与 Docker 连法能力对照：[`research/04-topology.md`](./research/04-topology.md)
- 吞吐分层与工具链 / nightly 能力面：[`research/05-throughput.md`](./research/05-throughput.md)
- 浏览器 UI 壳候选与竞品横切：[`research/06-web.md`](./research/06-web.md)
- 现有代码锚点：`src/app/core/driver/{proto,in_process,remote}.rs`、`src/app/server/{runtime,rest,lock}.rs`、`llmanspec/specs/server-core/spec.toon`
