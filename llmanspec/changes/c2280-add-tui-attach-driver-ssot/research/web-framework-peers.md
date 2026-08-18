# 开源 coding agent Web/GUI 控制台：框架与传输对照

> **竞品对照摘录，非选型结论。** 只引仓库 README / `package.json` / 官方文档与源码路径。`continue-dev/continue` 不存在（404）；正确仓库是 [`continuedev/continue`](https://github.com/continuedev/continue)。OpenCode：[`sst/opencode`](https://github.com/sst/opencode) 与 [`anomalyco/opencode`](https://github.com/anomalyco/opencode) 指向同一仓库（README 安装用 `anomalyco/tap`）。Goose 亦镜像于 [`aaif-goose/goose`](https://github.com/aaif-goose/goose)。

## 1. Continue — [`continuedev/continue`](https://github.com/continuedev/continue)

README 只列 CLI / VS Code / JetBrains，**无独立 Web 产品面**（[README](https://github.com/continuedev/continue/blob/main/README.md)）。

| 项 | 事实 |
|---|---|
| 前端 | IDE **webview**：React 18 + Vite（[`gui/package.json`](https://github.com/continuedev/continue/blob/main/gui/package.json)） |
| 传输 | VS Code `webview.postMessage`（[`extensions/vscode/src/ContinueGUIWebviewViewProvider.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/ContinueGUIWebviewViewProvider.ts)）；VS Code 下 Core 用进程内 `InProcessMessenger`（[`core/protocol/messenger/index.ts`](https://github.com/continuedev/continue/blob/main/core/protocol/messenger/index.ts)）。JetBrains 另起 Core 子进程（[`CoreMessenger.kt`](https://github.com/continuedev/continue/blob/main/extensions/intellij/src/main/kotlin/com/github/continuedev/continueintellijextension/continue/CoreMessenger.kt)） |
| UI vs 内核 | GUI 薄；agent 在 `core/`。VS Code：**同扩展宿主进程** |
| 对照 xylitol | 流式经 messenger 的 `llm/streamChat` 生成器回推；无浏览器 WS/SSE 产品面 |

## 2. OpenCode — [`anomalyco/opencode`](https://github.com/anomalyco/opencode)

有 Web：`opencode web`；TUI 可 `opencode attach` 到同一 server（[docs/web](https://opencode.ai/docs/web/)、[docs/server](https://opencode.ai/docs/server/)）。

| 项 | 事实 |
|---|---|
| 前端 | 产品 UI：**SolidJS + Vite**（[`packages/app/package.json`](https://github.com/anomalyco/opencode/blob/dev/packages/app/package.json)）。桌面：**Electron** 包同一 `@opencode-ai/app`（[`packages/desktop/package.json`](https://github.com/anomalyco/opencode/blob/dev/packages/desktop/package.json)）。`packages/web` 是 Astro 文档站，不是控制台 |
| 传输 | HTTP OpenAPI + **SSE** `GET /event`、`GET /global/event`（[docs/server](https://opencode.ai/docs/server/)）。审批：`POST /session/:id/permissions/:permissionID` |
| UI vs 内核 | 文档：「TUI 是连 server 的 client」；web/desktop 同为 client。`opencode` 启动时 **TUI 与 server 同进程起**，但 UI 不内嵌第二套 agent |
| 对照 xylitol | **多客户端 attach 同一 HTTP+SSE server**（流式、权限、多 project `GET /project`） |

## 3. Goose — [`block/goose`](https://github.com/block/goose)

桌面 GUI，不是浏览器 SaaS。README：desktop + CLI + API（[README](https://github.com/block/goose/blob/main/README.md)）。

| 项 | 事实 |
|---|---|
| 前端 | **Electron + React**（[`ui/desktop/README.md`](https://github.com/block/goose/blob/main/ui/desktop/README.md)、[`ui/desktop/package.json`](https://github.com/block/goose/blob/main/ui/desktop/package.json)） |
| 传输 | 桌面 **spawn 独立 `goose` CLI**，连其 ACP：`GOOSE_EXTERNAL_BACKEND_URL=http://127.0.0.1:3000`（同上 README）。ACP：`POST /acp` streamable HTTP（SSE MIME）+ `GET /acp` **WebSocket**（[`crates/goose-acp/src/transport.rs`](https://github.com/block/goose/blob/main/crates/goose-acp/src/transport.rs)）。另有 session SSE（[`crates/goose-server/src/routes/session_events.rs`](https://github.com/block/goose/blob/main/crates/goose-server/src/routes/session_events.rs)） |
| UI vs 内核 | UI 薄；agent 在 **另一进程** Rust CLI |
| 对照 xylitol | 审批走 ACP JSON-RPC；多 chat 由 desktop 对同一 ACP server |

## 4. Open WebUI — [`open-webui/open-webui`](https://github.com/open-webui/open-webui)

自托管聊天平台，非 coding harness。`open-webui serve` → `localhost:8080`（[README](https://github.com/open-webui/open-webui/blob/main/README.md)）。

| 项 | 事实 |
|---|---|
| 前端 | **Svelte 5 + SvelteKit + Vite**（[`package.json`](https://github.com/open-webui/open-webui/blob/main/package.json)） |
| 传输 | **Socket.IO WebSocket**（`socket.io-client`；服务端 [`backend/open_webui/socket/main.py`](https://github.com/open-webui/open-webui/blob/main/backend/open_webui/socket/main.py)）。另有 SSE 解析（[`src/lib/apis/streaming/index.ts`](https://github.com/open-webui/open-webui/blob/main/src/lib/apis/streaming/index.ts)）。README：多 worker 用 Redis + WebSocket |
| UI vs 内核 | 浏览器薄；agent/工具在 **Python 后端**。浏览器 ≠ 后端进程；`serve` 同进程出静态+API |
| 对照 xylitol | 流式走 WS emit；插件可做 approval flow（README Features） |

## 5. LibreChat — [`danny-avila/LibreChat`](https://github.com/danny-avila/LibreChat)

| 项 | 事实 |
|---|---|
| 前端 | **React 18 + Vite**（[`client/package.json`](https://github.com/danny-avila/LibreChat/blob/main/client/package.json)）；根 workspace 分 `api` / `client`（[根 package.json](https://github.com/danny-avila/LibreChat/blob/main/package.json)） |
| 传输 | **SSE**（`sse.js`；[`client/src/hooks/SSE/useSSE.ts`](https://github.com/danny-avila/LibreChat/blob/main/client/src/hooks/SSE/useSSE.ts)）。可恢复流：POST 开 job + GET `/api/agents/chat/stream/:streamId`（[docs](https://www.librechat.ai/docs/features/resumable_streams)、[`useResumableSSE.ts`](https://github.com/danny-avila/LibreChat/blob/main/client/src/hooks/SSE/useResumableSSE.ts)） |
| UI vs 内核 | client 薄；agent 在 **Node `api/`**（独立进程） |
| 对照 xylitol | SSE 事件 `ApprovalEvents.ON_PENDING_ACTION`（HITL）；多 tab 同会话同步 |

## 6. Cline — [`cline/cline`](https://github.com/cline/cline) + [`cline/kanban`](https://github.com/cline/kanban)

README：VS Code / JetBrains / CLI / SDK；**Kanban 是单独 web 任务板**（[cline README](https://github.com/cline/cline/blob/main/README.md)）。

| 项 | 事实 |
|---|---|
| 前端 | VS Code webview：**React 18 + Vite**（[`apps/vscode/webview-ui/package.json`](https://github.com/cline/cline/blob/main/apps/vscode/webview-ui/package.json)）。Kanban：**React + Vite**（[`web-ui/package.json`](https://github.com/cline/kanban/blob/main/web-ui/package.json)） |
| 传输 | IDE：gRPC 语义 overlay **`postMessage`**（[`webview-ui/src/services/grpc-client-base.ts`](https://github.com/cline/cline/blob/main/webview-ui/src/services/grpc-client-base.ts)、[`src/shared/WebviewMessage.ts`](https://github.com/cline/cline/blob/main/src/shared/WebviewMessage.ts)）。Kanban：tRPC + **WebSocket** PTY（[`src/terminal/ws-server.ts`](https://github.com/cline/kanban/blob/main/src/terminal/ws-server.ts)；根 [`package.json`](https://github.com/cline/kanban/blob/main/package.json) 含 `ws`/`@trpc/*`） |
| UI vs 内核 | webview 薄；agent 在扩展宿主 / `@cline/sdk`。Kanban **不内嵌 agent**，spawn CLI 进 worktree |
| 对照 xylitol | IDE 逐步审批（README Plan/Act）；Kanban **每卡独立 worktree** |

## 7. Aider — [`Aider-AI/aider`](https://github.com/Aider-AI/aider)

**无 Web 产品面。** README：「AI Pair Programming in Your Terminal」；「Copy/paste to web chat」是向第三方网页粘贴，不是本仓库 GUI（[README](https://github.com/Aider-AI/aider/blob/main/README.md)）。

## 8. Codex — [`openai/codex`](https://github.com/openai/codex)

README 三分：本地 CLI、`codex app` 桌面、云端 [chatgpt.com/codex](https://chatgpt.com/codex)（[README](https://github.com/openai/codex/blob/main/README.md)）。**云 Web 源码不在本 OSS 仓。** 桌面 Electron 主仓无公开 `packages/desktop`；社区 issue 从打包 asar 观察到 Electron+Vite（[issue #11023](https://github.com/openai/codex/issues/11023)）。可核验的 OSS 面：

| 项 | 事实 |
|---|---|
| 前端（OSS） | 公开的是 Rust TUI + **`codex app-server`** 给 VS Code 等富 UI（[`codex-rs/app-server/README.md`](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)） |
| 传输 | JSON-RPC：默认 **stdio JSONL**；`--listen ws://` **实验/不受支持**；unix socket 上 WS upgrade。审批章节 **Approvals** |
| UI vs 内核 | 扩展/桌面 renderer 连 **独立 app-server 进程**（issue 中 `codex.exe` vs Electron renderer） |
| 对照 xylitol | 流式 turn notifications；审批走 JSON-RPC，非浏览器 SSE |

## 9. Claude Code — [`anthropics/claude-code`](https://github.com/anthropics/claude-code)

公开仓是 CLI/plugins（[README](https://github.com/anthropics/claude-code/blob/main/README.md)）。官方文档另有 **Desktop** 与 **Web** [`claude.ai/code`](https://claude.ai/code)（[overview](https://code.claude.com/docs/en/overview)）。Web 任务跑在 Anthropic 云环境，关浏览器仍继续；`--teleport` / `--cloud` 在 CLI 与云之间搬 session（[claude-code-on-the-web](https://code.claude.com/docs/en/claude-code-on-the-web)）。**Web/Desktop 前端框架与 WS/SSE 实现未开源**，无法从本仓 `package.json` 核实。

| 项 | 事实 |
|---|---|
| 公开仓 | 无 Web 产品面源码 |
| 对照 xylitol | 文档层：多 surface 共引擎；Web ≠ 本机同进程。Remote Control 把**本地 CLI**暴露给网页监控，与云 session 是两条路径 |

## 10. Charmbracelet Crush — [`charmbracelet/crush`](https://github.com/charmbracelet/crush)

**无 Web 产品面。** README：「available in your favourite terminal」（[README](https://github.com/charmbracelet/crush/blob/main/README.md)）。MCP 可走 `http`/`stdio`/`sse`，那是工具通道，不是 GUI。未再发现其它 charm 官方 coding-agent Web 控制台。

## 可对照 xylitol 的横切事实（非推荐）

| 模式 | 谁 |
|---|---|
| 本机 HTTP+SSE，TUI/Web 同 server | OpenCode |
| 本机 spawn CLI，ACP HTTP/WS | Goose |
| 浏览器 SSE，agent 在 Node API | LibreChat |
| 浏览器 Socket.IO，agent 在 Python | Open WebUI |
| IDE postMessage，内核同扩展进程 | Continue（VS Code）、Cline webview |
| 多 worktree Web 板 + PTY WS | Cline Kanban |
| JSON-RPC stdio（WS 实验） | Codex app-server |
| 无本机 Web 面 | Aider、Crush；Claude Web 为云托管 |
