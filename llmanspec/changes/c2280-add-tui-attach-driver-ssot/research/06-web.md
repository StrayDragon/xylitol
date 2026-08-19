# 06 浏览器 UI 壳

> 未开闸。浏览器是同一 host 的另一个客户端；契约不堵 WebSocket（c2301）。内核在 Rust，roadmap：Web 用 TS 生态。

## 两层边界（事实）

| 层 | 已有事实 | 与浏览器的关系 |
|---|---|---|
| Host / 线协议 | `Command` / `Event` + WS + journal `last_seq` + 反向 RPC | 浏览器连同一 host |
| 浏览器 UI 壳 | 未开闸；roadmap 钉「TS 生态」 | 本篇 |

```text
浏览器 SPA  ──WS/HTTP──►  xylitol serve（XyDriver + dispatch）
    渲染 Event                 禁止前端第二套 ReAct
    应答 ReverseRpc            面本地留给浏览器
```

浏览器等价于 `XyRemoteDriver` 的另一种实现语言（TS），不是新词汇。

## 浏览器面要成立的约束维度（据现有产品约束推导）

| ID | 维度 | 依据 / 出处 |
|---|---|---|
| W1 | 薄客户端：只发 Command、吃 Event；不实现对话循环 | `docs/architecture/库与多客户端`；Cloud-Agent「避免再实现第二套 ReAct」 |
| W2 | 双向长连接（WebSocket 一等；SSE 不够） | 线协议是订阅 + 推事件；反向 RPC 要 client→server 应答（`ApproveTool` / `AnswerQuestion`）→ SSE 单向不够 |
| W3 | 反向 RPC UI：审批 / 问卷；多窗 first-wins；超时可降级 | `AskUserGateway` + WS `ReverseRpc` |
| W4 | 连接状态在客户端：`Subscribe{last_seq}`、`ResyncRequired` 后重订 | journal 环形缓冲会丢旧事件 |
| W5 | 高频增量可活：`TextDelta` / `ThinkingDelta` / `MessageUpdate` / 工具流 | 生命周期闭集；未知事件忽略不崩 |
| W6 | 改道与队列同源：Steer / FollowUp / Abort + `QueueUpdate` 进同一条流 | `插话续跑与中止`；远程禁止静默丢队列 |
| W7 | TypeScript 生态 | Cloud-Agent 产品约束；可选 Cursor SDK |
| W8 | 自托管静态壳：构建产物可被 `xylitol serve` 同源挂出；不依赖 Vercel/serverless 长连接 | Server 是 opt-in；默认 TUI 不背 Web 运行时 |
| W9 | 不逼出第二套服务端 action / BFF dispatch | 组合根与 `dispatch` 已在 Rust；JS 全栈框架的 server functions 是第三条语义源 |

次要维度（不满足可补、不阻塞）：**S1** 细粒度更新（token 级改 DOM）；**S2** 成熟 diff / 代码预览（Monaco / CodeMirror）；**S3** 键盘优先、与 TUI 同构；**S4** 一窗多工作区控制面；**S5** 默认二进制零成本（Web 资产走 `server` feature 或独立包）。

不适用维度（用它们选型会选歪）：SEO、首屏 HTML、RSC、多租户 SaaS、插件市场、移动完整编辑、自研观测台。

## 形态冲突的事实（哪些全栈形态与上述维度的官方/能力冲突）

| 形态 | 冲突的事实 | 出处 |
|---|---|---|
| Next.js App Router 当壳 | 官方 BFF 页：lambda 超时/响应后连接关闭，**「WebSockets won’t work」**；Route Handler 的 HTTP 动词无 Upgrade | [Next BFF · Deployment environment](https://nextjs.org/docs/app/guides/backend-for-frontend) |
| TanStack Start / Nuxt 当第二后端 | 官方定位 fullstack SSR + server functions；Start 的 server routes = HTTP handlers，无 List of WS upgrade | [Start overview](https://tanstack.com/start/latest/docs/framework/react/overview) |
| SolidStart / SvelteKit / Nuxt 的一等实时 | 一等 WS/stream 在**自家 JS 服务器**（Nitro WS、`+server.js`、route），落在浏览器与 Rust host 之间叠 BFF | [Nitro WS](https://nitro.build/docs/websocket) · [SvelteKit remote functions](https://svelte.dev/docs/kit/remote-functions) |
| Leptos / Dioxus 默认 Web | 产品已钉 TS 生态（Cursor SDK / 审阅 UI） | `docs/roadmaps/Cloud-Agent与Web控制台.md` |
| grpc-web 作为唯一双向 | tonic 官方无 WS、无 client/bidi streaming | [tonic-web](https://docs.rs/tonic-web/latest/tonic_web/) |

注：Next `output:'export'` 能出静态文件，但 SSR/RSC 全部用不上，只剩 React——等价于直接 Vite。

## 候选 UI 库能力对照

WS 本身是浏览器 API，UI 库都不「自带 WS」；差异在：是否把你推向 SSR/BFF、更新模型扛不扛得住 token 流、TS/审阅生态。

| 候选 | 官方更新模型 | 静态自托管 | 细粒度更新 | 审阅生态 | 备注 |
|---|---|---|---|---|---|
| **React 19 + Vite SPA** | 状态更新触发**整组件函数再执行**、再 diff DOM | `vite build` → `dist` | 默认不细粒度；token 须隔离到叶子 + `memo` | Monaco / diff 组件几乎全是 React | 贡献者池最大 |
| **Solid + Vite** | 组件函数**只跑一次**；signal 改哪补哪 | 同 Vite 静态 | 最细粒度 | 要包 React 组件或另找 | 与 token 流最同构 |
| **Svelte 5 + Kit `adapter-static`** | `$state` 深代理，属性级更新 | [官方 SPA](https://svelte.dev/docs/kit/single-page-apps) | 编译期细粒度 | 弱于 React | 与 React 生态隔离 |
| **Vue 3 + Vite** | 编译器知情的 Virtual DOM（patch flags） | 同 Vite | Proxy 细粒度 | 中等 | SPA 官方推荐场景恰是「深会话 + 前端状态」 |
| **TanStack Router**（不要 Start） | 同底层 React/Solid | Vite | 同底层 | 随底层 | 纯 SPA 路由时官方建议只用 Router |

React/Solid 对 `TextDelta` 的更新粒度是**真实差异**；但「React 做不到」是假的——Continue 类产品用 React 流式聊天是常态，代价是纪律：delta 不要抬到 layout 根 state。

## 竞品横切事实（一手摘录）

| 模式 | 谁 | 可对照的点 |
|---|---|---|
| 本机 HTTP+SSE，TUI/Web 同 server | **OpenCode**（SolidJS+Vite UI；`GET /event`、`GET /global/event` SSE；审批 `POST /session/:id/permissions/:permissionID`；TUI 可 `attach` 同一 server） | 多客户端 attach 同一 server |
| 本机 spawn CLI，ACP HTTP/WS | **Goose**（Electron+React；spawn 独立 `goose` CLI，`GOOSE_EXTERNAL_BACKEND_URL`；ACP streamable HTTP(SSE MIME) + `GET /acp` WebSocket） | 审批走 ACP JSON-RPC；多 chat 对同一 ACP server |
| 浏览器 SSE，agent 在 Node API | **LibreChat**（React 18+Vite client；`useSSE` / 可恢复流 `POST` 开 job + `GET /stream/:streamId`；`ApprovalEvents` HITL） | SSE 事件驱动审批 |
| 浏览器 Socket.IO，agent 在 Python | **Open WebUI**（Svelte 5+Kit；Socket.IO WS + SSE；`serve` 同进程出静态+API） | 流式走 WS emit |
| IDE postMessage，内核同扩展进程 | **Continue**（React 18+Vite webview；VS Code 用进程内 `InProcessMessenger`；JetBrains 另起 Core 子进程） / **Cline**（React 18+Vite webview；gRPC 语义 overlay `postMessage`） | 无浏览器 WS/SSE 产品面 |
| 多 worktree Web 板 + PTY WS | **Cline Kanban**（React+Vite；tRPC + WebSocket PTY；每卡独立 worktree） | 多 worktree 控制面 |
| JSON-RPC stdio（WS 实验） | **Codex app-server**（OSS 仓 Rust TUI + `codex app-server`；默认 stdio JSONL、`--listen ws://` 实验/不支持；审批章节 Approvals；桌面 Electron 源码不在 OSS 仓） | 流式 turn notifications |
| 无本机 Web 面 | **Aider**、**Crush**；**Claude Code** 公开仓只有 CLI，Web/Desktop 未开源，云 Web 任务跑在 Anthropic 云（`--teleport` / `--cloud` 搬 session），Web ≠ 本机同进程 | — |

## 一手来源

- 本仓约束：`docs/architecture/{库与多客户端,远程体验与线协议,用户可见事件,插话续跑与中止}.md`；`docs/roadmaps/{Cloud-Agent与Web控制台,Web与TUI同源}.md`；`src/protocol/wire/{command,event}.rs`；`src/app/server/ws.rs`；`llmanspec/specs/server-core/server-{ws,reverse-rpc}.feature`
- React：[render-and-commit](https://react.dev/learn/render-and-commit) · [creating-a-react-app](https://react.dev/learn/creating-a-react-app) · [streaming（SSR+Suspense）](https://react.dev/reference/react-dom/server) · [use-client（RSC 不能持 socket）](https://react.dev/reference/rsc/use-client)
- Vite：[static-deploy](https://vite.dev/guide/static-deploy.html)
- Next：[BFF（WebSockets won’t work）](https://nextjs.org/docs/app/guides/backend-for-frontend) · [route-handlers（无 Upgrade）](https://nextjs.org/docs/app/getting-started/route-handlers) · [static-exports](https://nextjs.org/docs/app/guides/static-exports)
- Solid：[component basics（只跑一次）](https://docs.solidjs.com/concepts/components/basics) · [intro-to-reactivity](https://docs.solidjs.com/concepts/intro-to-reactivity)
- Svelte：[SPA / adapter-static](https://svelte.dev/docs/kit/single-page-apps) · [project-types（Rust 后端+SPA）](https://svelte.dev/docs/kit/project-types) · [remote functions](https://svelte.dev/docs/kit/remote-functions)
- Vue / Nuxt：[ways-of-using-vue](https://vuejs.org/guide/extras/ways-of-using-vue.html) · [rendering-mechanism](https://vuejs.org/guide/extras/rendering-mechanism.html) · [Nitro WS](https://nitro.build/docs/websocket)
- TanStack：[Start vs Router overview](https://tanstack.com/start/latest/docs/framework/react/overview) · [server functions](https://tanstack.com/start/latest/docs/framework/react/guide/server-functions)
- 竞品：OpenCode [web/server docs](https://opencode.ai/docs/server/) · Goose [desktop README / ACP transport](https://github.com/block/goose/blob/main/crates/goose-acp/src/transport.rs) · LibreChat [useSSE](https://github.com/danny-avila/LibreChat/blob/main/client/src/hooks/SSE/useSSE.ts) · Codex [app-server README](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md) · Claude [claude-code-on-the-web](https://code.claude.com/docs/en/claude-code-on-the-web)
