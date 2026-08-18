# CS 架构对 Web 框架的要求（xylitol）

> 本篇是 c2280「传输选型不堵未来 Web」的展开：**先钉要求，再只评能满足的 UI 框架**。不是实现票，不改 live specs。
> 产品约束真源：本仓 architecture / roadmaps / `protocol` / `server-core`。框架能力真源：各框架官网（文末 URL）。上游摘录若另有 `web-framework-upstream.md` / `web-framework-peers.md`，以那些文件的引用块为准。

## 先分两层（避免评错对象）

| 层 | 已有事实 | 本篇评什么 |
|---|---|---|
| **Host / 线协议** | `Command`/`Event` + axum HTTP/WS + journal `last_seq` + 反向 RPC | **不换**；见 `research/scope.md` T2 |
| **浏览器 UI 壳** | 未开闸；roadmap 写死「Web 用 TS 生态」 | **本篇** |

Web 框架若把你推向「再写一个 Node BFF / RSC 里跑 agent」，就和「内核仍在 xylitol、Web 是控制与呈现面」打架。

```text
浏览器 SPA  ──WS/HTTP──►  xylitol serve（XyDriver + dispatch）
   渲染 Event                 禁止前端第二套 ReAct
   应答 ReverseRpc            面本地（键位/选区/剪贴板）留在浏览器
```

本机 attach 的 TUI 是同一 Driver 的另一扇窗，不是另一套协议。

## 要求（按 xylitol 加约束，不是通用 CRUD）

### MUST（不满足则否决）

| ID | 要求 | 为何是我们的，不是「网站常识」 |
|---|---|---|
| W1 | **薄客户端**：只发 `Command`、吃 `Event`；不实现对话循环 | `docs/architecture/库与多客户端.md`；Cloud-Agent：「避免再实现第二套 ReAct」 |
| W2 | **浏览器可达的双向长连接**（WebSocket 一等；SSE 不够） | 线协议是订阅 + 推事件；反向 RPC 要 client→server 应答（`ApproveTool` / `AnswerQuestion`）。SSE 是单向的。`server-ws` / `server-reverse-rpc` |
| W3 | **反向 RPC UI**：审批 / 问卷；多窗 first-wins；超时可降级 | `AskUserGateway` + WS `ReverseRpc`；不是表单 POST |
| W4 | **连接状态在客户端**：`Subscribe{last_seq}`、`ResyncRequired` 后重订 | journal 环形缓冲会丢旧事件（`w4`–`w6`） |
| W5 | **高频增量可活**：`TextDelta` / `ThinkingDelta` / `MessageUpdate` / 工具流 | 生命周期闭集；未知事件忽略不崩（`用户可见事件.md`） |
| W6 | **改道与队列同源**：Steer / FollowUp / Abort + `QueueUpdate` 进同一条流 | `插话续跑与中止.md`；远程禁止静默丢队列 |
| W7 | **TypeScript 生态** | Cloud-Agent 产品约束；可选 Cursor SDK |
| W8 | **自托管静态壳**：构建产物可被 `xylitol serve` 同源挂出；不依赖 Vercel/serverless 长连接 | Server 是 opt-in；默认 TUI 不背 Web 运行时 |
| W9 | **框架不逼出第二套服务端 action / BFF dispatch** | 组合根与 `dispatch` 已在 Rust；JS 全栈框架的 server functions 是第三条语义源 |

### SHOULD（不满足可补，不否决）

| ID | 要求 |
|---|---|
| S1 | 细粒度更新（token 级改 DOM，不必重跑整树） |
| S2 | 成熟 diff / 代码预览（Monaco 或 CodeMirror）——审阅面 M3 |
| S3 | 键盘优先；公共能力键位与 TUI SHOULD 同构 |
| S4 | 一窗多工作区控制面（会话列表 ≠ 再开 N 个 agent 内核） |
| S5 | 默认二进制 / 开箱路径 **零成本**（Web 资产走 `server` feature 或独立包） |

### 明确不要求（用它们选型会选歪）

SEO、首屏 HTML、RSC、多租户 SaaS、插件市场、移动完整编辑、自研观测台。

## 一刀切掉的形态

| 形态 | 原因 | 一手出处 |
|---|---|---|
| **Next.js 当产品壳（App Router Route Handlers 承载 WS）** | 官方 BFF：lambda 超时后连接关闭，**WebSockets won’t work** | [Next.js BFF · Deployment environment](https://nextjs.org/docs/app/guides/backend-for-frontend) |
| **TanStack Start / Nuxt 当第二后端** | 官方定位 fullstack SSR + server functions；我们已有 Rust host | [Start overview](https://tanstack.com/start/latest/docs/framework/react/overview) |
| **Leptos / Dioxus 默认 Web** | 产品已钉 TS 生态（Cursor SDK / 审阅 UI） | `docs/roadmaps/Cloud-Agent与Web控制台.md` |
| **纯 HTMX 控制台** | 反向 RPC、流式 markdown、diff 预览不是「点击换 HTML」 | 能力缺口，非官网否决 |
| **gRPC-only（T2 的 C5 若作为唯一传输）** | 浏览器要 grpc-web；与现有 JSON 帧双栈 | `research/scope.md` C5 |

`output: 'export'` 的 Next 能出静态文件，但此时 Next 的 SSR/RSC 全部用不上，只剩 React——应直接 Vite。

## 能满足 MUST 的候选对照

WS 本身是浏览器 API，**UI 库都不「自带 WS」**。差异在：是否把你推向 SSR/BFF、更新模型是否扛得住 token 流、TS/审阅生态。

| 候选 | 官方定位 | W2 长连接 | W8 静态自托管 | S1 流式 DOM | S2 审阅生态 | 相对 xylitol |
|---|---|---|---|---|---|---|
| **React 19 + Vite SPA** | 客户端渲染；状态更新触发 **整组件函数再执行**，再 diff DOM | 自接 `WebSocket` | `vite build` → `dist`，任意静态托管 | 默认不细粒度；token 须隔离到叶子 + `memo` | Monaco / diff 组件几乎全是 React | **默认采用** |
| **Solid + Vite** | 组件函数 **只跑一次**；signal 改哪补哪 | 自接 | 同 Vite 静态 | **最贴 W5** | 要包 React 组件或另找 | **流式备选** |
| **Svelte 5 + Kit `adapter-static`** | 无 `+server.js` 时可 SPA；`fallback: '200.html'` | 自接 | [官方 SPA](https://svelte.dev/docs/kit/single-page-apps) | 编译期细粒度 | 弱于 React | **可采用** |
| **Vue 3 + Vite** | 官方把「深会话、非平凡前端状态」标成 SPA 用法 | 自接 | 同 Vite | Proxy 细粒度 | 中等 | **可采用** |
| **TanStack Router（不要 Start）** | 官方：若确定不需要 SSR/server functions，**只用 Router 做 SPA** | 自接 | Vite | 取决于 React/Solid | 随 UI 库 | **路由可选，不作壳** |
| **Next.js App Router** | 全栈 / RSC / Route Handler 流 | 官方 serverless **不能**把 WS 放在 Handler 里；客户端仍可连 **外部** axum | `export` 能静态，但框架税浪费 | React 同左 | 同 React | **否决作壳**；勿让 xylitol 依赖 Next 进程 |

React 官方：后续 render 会再次调用发生 state 更新的组件函数（[Render and Commit](https://react.dev/learn/render-and-commit)）。Solid 官方：组件只在初次执行，之后只更新依赖该 signal 的 DOM（[Component basics](https://docs.solidjs.com/concepts/components/basics)）。对 `TextDelta` 这是真实差异，**不是**「React 做不到」——Continue 类产品用 React 流式聊天是常态，代价是纪律（delta 不要抬到 layout 根 state）。

## 优劣（只谈对我们痛的点）

**React + Vite**
- 利：W7/S2/S4 最省；Cursor SDK 样例是 TS/React；贡献者池最大；axum 只挂静态 + 已有 `/api/v1/.../ws`。
- 弊：W5 要自己做更新边界；容易把「Next 惯例」带进来（server actions）—— **规范里禁止**。

**Solid**
- 利：W5/S1 与 token 流同构。
- 弊：Monaco/SDK/招聘；和 TUI 同源动作层仍要自写，框架帮不上。

**SvelteKit static**
- 利：官方承认「无 server 逻辑就 adapter-static」；产物仍是静态，host 仍是 xylitol。
- 弊：Runes 心智与 React 生态隔离；审阅组件少。

**Vue 3**
- 利：SPA 官方推荐场景正好是「深会话 + 前端状态」。
- 弊：coding-agent 控制台先例少于 React；Cursor SDK 集成要自己包。

## 给 c2280 / 实现票的可证伪结论

1. **Web 框架默认 = React + Vite SPA**（静态 `dist`，同源连现有 axum WS）。不选 Next/Start 当壳。
2. **传输 MUST 保持浏览器能升级 WS**（T2：C1 或 C2）。C3/C4 若作本机唯一载体，Web 必须再开 HTTP/WS——那是第二套，违反「不堵 Web」。
3. **协议不换**：浏览器只是 `XyRemoteDriver` 的另一种实现语言（TS），不是新词汇。
4. 流式性能不够再评 Solid，不作为第一刀。

## 来源

- 本仓：`docs/architecture/{库与多客户端,远程体验与线协议,用户可见事件,插话续跑与中止}.md`；`docs/roadmaps/{Cloud-Agent与Web控制台,Web与TUI同源}.md`；`src/protocol/wire/{command,event}.rs`；`src/app/server/ws.rs`；`llmanspec/specs/server-core/server-{ws,reverse-rpc}.feature`；`llmanspec/specs/protocol-app/spec.toon`
- 上游： [React render](https://react.dev/learn/render-and-commit) · [Vite static dist](https://vite.dev/guide/static-deploy.html) · [Next BFF WS](https://nextjs.org/docs/app/guides/backend-for-frontend) · [Solid lifecycle](https://docs.solidjs.com/concepts/components/basics) · [SvelteKit SPA](https://svelte.dev/docs/kit/single-page-apps) · [Vue SPA](https://vuejs.org/guide/extras/ways-of-using-vue.html) · [TanStack Start vs Router](https://tanstack.com/start/latest/docs/framework/react/overview)
