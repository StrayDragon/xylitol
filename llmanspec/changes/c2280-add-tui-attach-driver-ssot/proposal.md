---
depends_on: []
---

# 双模 TUI：Driver SSOT 与本机 host 传输选型（研究草案）

> **一句话**：产品保留「默认进程内 TUI」与「显式 `xylitol serve` 后 attach」两条启动与连接方式；用同一套 TUI host + `XyDriver` 符合性闸把漂移压住；本票只做固定深度调研与 SSOT 设计，不实现。
> **产品启动与连接方式（已钉）**：先 `xylitol serve`（不做 Ensure-running / 第一扇窗兼 host）；习惯用窗口管理器、多开终端的用户可用会话/autostart 起 host。默认仍原生 TUI。

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

目标：用户看见两条启动与连接方式，仓库只养 **一套 TUI + 一套驱动合约**。

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
6. **启动与连接显式**：`xylitol tui --attach`（或等价）连 host；**禁止**「探测到 socket 就静默切 Remote」（第三模式，bang/cwd/失败语义会鬼畜）。serve 未起 → attach 失败并提示 `xylitol serve`。
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
- 后续实现票：TUI 启动与连接方式、Server 多 runtime、锁从 `/tmp/xylitol-server.lock` 改为用户 runtime 目录、MCP 池 key

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
- **双模代码 SSOT**：一个 TUI host；Agent 能力只经 `XyDriver` + `dispatch`；面本地不进 Driver；符合性闸同表双跑；MCP 池只发生在 serve 进程（key 意向 `(mcp_name, canonical cwd)`）；启动与连接显式。见上文「双模代码 SSOT」。
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
- ACP 外部侧表面事实（未来迭代 / provider 适配器）：[`research/acp-interop.md`](./research/acp-interop.md)
- 现有代码锚点：`src/app/core/driver/{proto,in_process,remote}.rs`、`src/app/server/{runtime,rest,lock}.rs`、`llmanspec/specs/server-core/spec.toon`

## 拆分草案（Open Questions —— DRAFT，未定死）

> 本节是「从研究收口到实现立项」的拆分草稿。**标注 DRAFT：不构成最终约束**。research（00–06）尚未核对完，术语/结论敲定并回填 01–06 之后再转正为正式实现 change（id 暂占 c230…，收敛后可重排）。此处只记意图与冲突，不锁约束。

### 已记录的意图（用户，DRAFT）

- **本轮边界**：直达 **多 TUI ↔ 多 session**——server 多 runtime 纳入本轮（对应产品 P1：多仓各开 + 同仓多 session）。
- **形态方向**：**统一 CS**——TUI 恒为客户端，只发 Command / 只收 Event；「进程内默认」只是 serve 的 **embed 形态**（默认单命令 = 同进程 embedded host + 协议自连）。用户可见的默认路径可保持不变，但**代码路径统一为一条**，不再有"进程内 vs 远程"两套语义。
- **REST**：现有 `src/app/server/rest.rs` 是临时设计，**不作为基准**；整体废弃、按新协议重做新 transport 面。
- **拆分偏好**：尽量粗（可单 change 分多个 task 目标），但**每个 change 必须可验证**；先 draft、不锁约束。

### 与既有内容冲突（待 research 核对后裁决）

1. 统一 CS vs 现「已钉产品决策」P4「双模：默认进程内 + 显式 attach」→ 需修 P4（embed 与 attach 是同一协议族的两条启动与连接方式）。
2. REST 废弃重建 vs research「保留极薄 HTTP 旁路」→ 按用户意图走废弃，research 相应修订（`01` / `04` 里 REST 相关内容）。
3. 本轮含 server 多 runtime（原「非目标/实现票」范围）→ 边界扩大，从「研究」变「实现立项」。

### 粗拆草案（非最终；收敛后填 id / spec / tasks / 验收）

> 产品位 draft **全部落地**（先建后改、依赖图以各 proposal `depends_on` 为准）：
> `c2300 归属+连接方式` → `c2301 稳定线协议` → `c2302 host 多会话+传输面`（框架已锁 salvo/axum 回退） → `c2303 统一启动与连接（embed+attach）` → `c2304 符合性闸双跑` → `c2305 ACP 外层适配器（后置迭代）`。
> 与下表映射：A→c2301，B→c2302，C→c2303，D→c2304（bang 直播语义在 c2301 协议闭集 + c2303 端到端验证）。

| 块 | 做什么（草案） | 可验证点（草案） |
|---|---|---|
| A | 稳定 wire 闭集：Command/Event + session 语义 + bash + 反向 RPC + journal/replay 定型（0.0.1 前禁双语义） | wire round-trip；TUI 方法集对协议 gap=0 |
| B | Server 组合根多 runtime（按 session 单飞）+ 全新 transport（废弃现 REST） | 多 session 隔离 BDD；健康/调试端点 |
| C | 统一启动与连接：embed 默认（同进程 host + 协议自连）+ `--attach` 连外部 serve；面本地在 TUI 侧 | 默认路径端到端 + attach 路径端到端 |
| D | 符合性闸双跑（embed host 内实现 ∪ Remote）+ bang 直播（`BashDelta` / 两条 bash API） | 同一场景表双双全绿；多 chunk 直播 + Abort |

### 待核对 / 待定（未锁）

- research 00–06 的术语与结论是否保持（逐篇核对中）。
- 最终 change 数量与 id（很粗 → 数个大 change；或 1 change 多 tasks）。
- ~~**连接方式的载体形态（Open）**~~ **已决（c2300）**：开销已实测（`c2300/research/overhead-eval.md`）——载体开销可忽略 → **默认 A（统一 CS）**；B（单进程自持）由「进程税 / 启动体感」产品判定，不再阻塞 c2301。
- **HTTP/server 框架（已决：salvo）**：旧「钉 axum」premise 失效（REST 废弃重建 + 统一 CS(A) 后本机 UDS 是一级拓扑）；salvo 覆盖需求矩阵见 `c2302/research/framework-pick.md`（WS 一等、UDS+TCP 双听、sock 权限、优雅停机带 force、广播背压文档化、静态/CORS/ACP 可后加）；**axum 为保守回退**。按用户要求不跑可行性验证，仅文档对比锁选。
- **ACP 外部接入（调研已完成 → 后置决策）**：结论见 `research/acp-interop.md`。定位 **provider 角色 + WebSocket-only 外层适配器**，消费 native 契约、不入核心闭集；会话/消息/权限/取消近 1:1，三处必桥：① `last_seq` 精确续传（ACP v1 不重放、v2 才有）② steer/队列（v1 缺、v2「beyond the turn」才对上）③ 单写者 + 只读第二 attach（ACP 无此概念，适配层自强制写者锁）。**排在 native wire 稳定 + serve 多会话之后**，独立 change（`depends_on` 核心 wire）；v1 够基本会话/审批、要 steer/强续传则等 v2 稳定再评。**本轮不作核心交付。**
- 多窗写入规则已定（c2300）：一 session 一写者；附加窗口 = 只读静态恢复；host 追踪每 session 连接数。
- 多 session 的 session 归属与 MCP 池 key 对齐（`(mcp_name, canonical cwd)`）。
