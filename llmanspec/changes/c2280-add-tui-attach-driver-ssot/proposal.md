---
depends_on: []
---

# 双模 TUI：Driver SSOT 与本机 host 传输选型（研究草案）

> **一句话**：产品保留「默认进程内 TUI」与「显式 `xylitol serve` 后 attach」两条进线；用同一套 TUI host + `XyDriver` 符合性闸把漂移压住；本票只做固定深度调研与 SSOT 设计，不实现。
> **产品进线（已钉）**：先 `xylitol serve`（不做 Ensure-running / 第一扇窗兼 host）；习惯用窗口管理器、多开终端的用户可用会话/autostart 起 host。默认仍原生 TUI。

## Why

习惯用窗口管理器、经常多开终端的用户（例如 i3 / sway / Hyprland，或 tmux 多窗格）会开多扇 TUI：多仓各开 + 同仓多 session。痛点是 RSS、MCP stdio 被每窗 clone、冷启动重复 bootstrap。现有 Server / `XyRemoteDriver` 是缝，但 Server 仍是一把 `Mutex<XyInProcessDriver>`、REST 路径里的 `session_id` 大量未真正路由，TUI 从不 attach。拉起 serve 的习惯见 [`research/serve-autostart.md`](./research/serve-autostart.md)。

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

## Further Notes

- 探索对话结论摘要：多仓 RSS 下限仍是 N 套工具世界；attach 主要打 **同仓第二扇窗** 的 MCP/启动；DSH 的 host≠TUI 父进程。
- 调研范围与一手资料清单：[`research/scope.md`](./research/scope.md)（固定深度；未完成文献前不选框架）。
- **架构 E**：操作器在 server/sandbox，TUI 只画界面和采集输入。tokio 留下。长期不做 Web。
- 用词（wire protocol / Command / Event / tagged JSON …）：[`research/glossary.md`](./research/glossary.md)。
- wire protocol 继续 **tagged JSON**（不换 JSON-RPC envelope）。HTTP 栈钉 **axum**，listener 按拓扑换：[`research/http-stack-pick.md`](./research/http-stack-pick.md)。
- Docker 连法：默认 **published port**；UDS 仅同内核 Linux；vsock 不进 v1：[`research/docker-connect.md`](./research/docker-connect.md)。
- **server 进 Docker = 粗粒度沙盒**；N TUI ↔ 1 server 是本票；多 server 编排以后再做：[`research/topology-sandbox.md`](./research/topology-sandbox.md)。
- poem/salvo 的 UDS 双听确实更好，但对 Docker 主 attach（TCP）不对齐，不换栈：[`research/http-stack-revisit.md`](./research/http-stack-revisit.md)。
- bang 直播是 attach MUST（Remote 现在 REST 丢掉 `chunk_tx`）。实现票草稿（不改 live specs）：[`research/impl-bashdelta.md`](./research/impl-bashdelta.md)。Event 名 **`BashDelta`**。
- **默认**本机 serve；Docker 是可选粗粒度沙盒。多 server 编排只留指针，不画协议：[`research/topology-sandbox.md`](./research/topology-sandbox.md)。
- 窗口管理器 / 多开终端用户如何拉起 serve（不实现）：[`research/serve-autostart.md`](./research/serve-autostart.md)。

- 哪段代码必须留 TUI：[`research/ssot-seams.md`](./research/ssot-seams.md)（剪贴板、TTY；**bang 在工作区**；export/import 按路径切开）。

- nightly：[`nightly-simd.md`](./nightly-simd.md)。默认 1.97.1 stable。postcard / rkyv / ntex-uring 都不需要 nightly。`portable_simd` 和 Cranelift 单独 spike。
- 现有代码锚点：`src/app/core/driver/{proto,in_process,remote}.rs`、`src/app/server/{runtime,rest,lock}.rs`、`llmanspec/specs/server-core/spec.toon`。
- 一手摘录（不是选型决议；决议在上列 pick / revisit）：
  - TS 壳：[`research/web-framework-upstream.md`](./research/web-framework-upstream.md)（React+Vite 可；Next App Router 官方否定 Route Handler WebSocket）
  - 竞品 attach：[`research/web-framework-peers.md`](./research/web-framework-peers.md)（最接近：OpenCode 本机 HTTP+SSE，`web` 与 `attach` 共 server）
  - Rust HTTP 栈：[`research/rust-server-framework-upstream.md`](./research/rust-server-framework-upstream.md)（axum 0.8 覆盖 UDS+WS+JSON；jsonrpsee/tonic 换 envelope）
  - ≥4000 star HTTP JSON：[`research/http-json-frameworks-upstream.md`](./research/http-json-frameworks-upstream.md)（无过闸的 JSON-RPC 2.0 服务端；jsonrpsee 851）
