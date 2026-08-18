> 本文件是 **上游摘录**（官方文档 / 框架站 / 规范），抓取日 2026-08-18。
> **不是** xylitol 产品结论；不改 live specs、不改应用代码。
> 对照轴仅作阅读框：localhost / self-host CS 控制台、内核已在 Rust、JSON Command/Event + 长连接 WebSocket 双向帧 + 反向 RPC、高频 TextDelta、禁止前端第二套 ReAct。
> 每条论断带 URL。不引博客。

---

## 1. React 19 + Vite SPA

- **官方定位（SPA/SSR/全栈）**：React 团队推荐全栈框架（Next / React Router / Expo）；若约束不被框架覆盖，可从 Vite 等构建工具从零搭应用。[react.dev/learn/creating-a-react-app](https://react.dev/learn/creating-a-react-app)。Vite 是 dev server + 生产静态资源打包器，`vite build` 输出可部署的静态资源。[vitejs.dev/guide](https://vitejs.dev/guide/)。
- **高频流式文本：细粒度 vs VDOM；官方 streaming UI**：状态更新触发组件函数重跑，再对 DOM 做最小提交（协调/VDOM 模型）。[react.dev/learn/render-and-commit](https://react.dev/learn/render-and-commit)。官方 streaming 是 **SSR HTML 流 + Suspense**（`renderToReadableStream` / `renderToPipeableStream`），不是「token 追加到已挂载文本节点」的指引。[react.dev/reference/react-dom/server](https://react.dev/reference/react-dom/server)、[react.dev/reference/react/Suspense](https://react.dev/reference/react/Suspense)。
- **WebSocket / SSE**：框架无一等 WS/SSE。客户端用浏览器 `WebSocket` / `EventSource` 自接。
- **双向 RPC / 长 socket vs SSR/RSC**：纯 SPA 不启用 RSC。RSC 默认在服务端渲染，**不能**用事件处理 / 多数 Hooks / 持久状态；交互与浏览器 API 必须 `'use client'`。[react.dev/reference/rsc/use-client](https://react.dev/reference/rsc/use-client)。Server Components 输出不在内存中持久化，无法持有 socket。
- **TS 类型**：官方文档要求加 `@types/react` / `@types/react-dom`。[react.dev/learn/typescript](https://react.dev/learn/typescript)。Vite 内建 TS 转译。[vitejs.dev/guide/features](https://vitejs.dev/guide/features.html)。
- **代码编辑/diff**：官网未提 Monaco/CodeMirror → **生态、非官方**。
- **本地 self-host 静态资源**：`vite build` → `dist`，可部署到任意静态主机。[vitejs.dev/guide/static-deploy](https://vitejs.dev/guide/static-deploy.html)。
- **前端不跑 agent 内核**：Vite SPA 不提供 Server Actions；不推向 BFF。React 官方仍把全栈框架当默认起点。[react.dev/learn/creating-a-react-app](https://react.dev/learn/creating-a-react-app)。
- **一句话：采用。** 出处：Vite 静态产物 + React「从零 Vite」路径；RSC 限制只在启用全栈时才打架。

---

## 2. Next.js App Router（当前稳定文档，站点标 16.3.1）

- **官方定位**：用 App Router **创建全栈 Web 应用**。[nextjs.org/docs/app/getting-started](https://nextjs.org/docs/app/getting-started)。默认 layout/page 是 Server Components。[nextjs.org/docs/app/getting-started/server-and-client-components](https://nextjs.org/docs/app/getting-started/server-and-client-components)。
- **高频流式文本**：底层仍是 React 协调。官方 streaming = **Suspense 页面流** + Route Handler 的 Web Streams；并写明常与 LLM 流式内容一起用。[nextjs.org/docs/app/guides/streaming](https://nextjs.org/docs/app/guides/streaming)、[nextjs.org/docs/app/api-reference/file-conventions/route](https://nextjs.org/docs/app/api-reference/file-conventions/route)。**没有**「客户端 token 细粒度补丁」指引。
- **WebSocket / SSE**：SSE/原始 `ReadableStream` 是 Route Handler 一等模式。[nextjs.org/docs/app/guides/streaming](https://nextjs.org/docs/app/guides/streaming)。BFF 指南明确：lambda 上长任务会被超时杀掉，**「WebSockets won’t work because the connection closes on timeout, or after the response is generated.」** [nextjs.org/docs/app/guides/backend-for-frontend](https://nextjs.org/docs/app/guides/backend-for-frontend)。HTTP 动词仅 GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS（无 Upgrade）。[nextjs.org/docs/app/getting-started/route-handlers](https://nextjs.org/docs/app/getting-started/route-handlers)。
- **双向 RPC / 长 socket vs SSR/RSC**：浏览器 API / `useEffect` / 状态只能在 Client Components。[nextjs.org/docs/app/getting-started/server-and-client-components](https://nextjs.org/docs/app/getting-started/server-and-client-components)。Server Functions/Actions 是 **异步 POST、单次往返**；客户端「一次一个」派发。[nextjs.org/docs/app/getting-started/mutating-data](https://nextjs.org/docs/app/getting-started/mutating-data)。这与「host 经同一长 socket 反向问 client 审批」不是同一模型。RSC 不能持有 socket（见 React `'use client'` 限制）。
- **TS**：一等（`create-next-app` / RouteContext 类型生成）。[nextjs.org/docs/app/getting-started/route-handlers](https://nextjs.org/docs/app/getting-started/route-handlers)。
- **代码编辑/diff**：**生态、非官方**。
- **纯静态 SPA**：`output: 'export'` 生成 `out`，任意静态服务器可托管。**不支持** Server Actions、依赖 Request 的 Route Handlers 等。[nextjs.org/docs/app/guides/static-exports](https://nextjs.org/docs/app/guides/static-exports)。
- **推向 BFF**：官方 BFF 指南把 Next 当前端的后端；`create-next-app --api`。[nextjs.org/docs/app/guides/backend-for-frontend](https://nextjs.org/docs/app/guides/backend-for-frontend)。Server Functions 在服务端跑逻辑。[nextjs.org/docs/app/getting-started/mutating-data](https://nextjs.org/docs/app/getting-started/mutating-data)。
- **一句话：否决（作 CS 控制台宿主）。** 出处：BFF 页否定 WS；Actions 是 POST 往返；默认 RSC/全栈会把内核逻辑吸进 Next。静态 export 能当 SPA，但官方能力被砍掉，与「选 Next 的理由」相反。

---

## 3. Solid.js + Vite

- **官方定位**：细粒度响应式 UI 库。[docs.solidjs.com](https://docs.solidjs.com/)。脚手架 `create-solid` 可选 TypeScript；可选 **SolidStart** 全栈，本候选是库 + Vite。[docs.solidjs.com/quick-start](https://docs.solidjs.com/quick-start)。Vite 模板含 `solid` / `solid-ts`。[vitejs.dev/guide](https://vitejs.dev/guide/)。
- **高频流式文本**：官方：「fine-grained reactive system, updates are only applied to the parts of the page that need to be updated」；Counter 示例「Only `count()` is updated」。[docs.solidjs.com](https://docs.solidjs.com/)、[docs.solidjs.com/concepts/intro-to-reactivity](https://docs.solidjs.com/concepts/intro-to-reactivity)。**无**「LLM token 流 UI」专章。SSR 流是 `renderToStream` / Router streaming（HTML 壳），与客户端 signal 更新是两件事。[docs.solidjs.com/reference/rendering/render-to-stream](https://docs.solidjs.com/reference/rendering/render-to-stream)、[docs.solidjs.com/solid-router/data-fetching/streaming](https://docs.solidjs.com/solid-router/data-fetching/streaming)。
- **WebSocket / SSE**：Solid 库无一等。SolidStart v2 经 Nitro `features.websocket` + `defineWebSocketHandler` 一等（那是 **Start 服务端**，不是 Vite SPA）。[docs.solidjs.com/solid-start/v2/advanced/websocket](https://docs.solidjs.com/solid-start/v2/advanced/websocket)。
- **双向 RPC / 长 socket vs SSR**：纯 CSR 组件可在 effect 里持有 `WebSocket`。SSR/Start 的 WS 落在 Nitro 进程，与「Rust 内核持 socket」叠床。
- **TS**：脚手架一等选项。[docs.solidjs.com/quick-start](https://docs.solidjs.com/quick-start)。
- **代码编辑/diff**：**生态、非官方**。
- **纯静态 SPA**：Vite `dist`。[vitejs.dev/guide/static-deploy](https://vitejs.dev/guide/static-deploy.html)。
- **推向 BFF**：Solid+Vite 不推。SolidStart 提供服务端路由/WS，会把实时层放进 JS 服务器。
- **一句话：采用（Solid + Vite CSR）。** 出处：官方细粒度更新句 + Vite 静态部署。勿把 SolidStart Nitro WS 当成控制台传输层。

---

## 4. Svelte 5 + SvelteKit

- **官方定位**：Svelte = 编译 UI 组件；SvelteKit = 应用框架（类比 Next/Nuxt），可 SSR / CSR / prerender。[svelte.dev/docs/kit/introduction](https://svelte.dev/docs/kit/introduction)。
- **高频流式文本**：`$state` 深代理，「modifying an individual … property will trigger updates to anything in your UI that depends on that specific property」。[svelte.dev/docs/svelte/$state](https://svelte.dev/docs/svelte/$state)。生命周期文档：更新单位不是整个组件，只反应需要反应的部分。[svelte.dev/docs/svelte/lifecycle-hooks](https://svelte.dev/docs/svelte/lifecycle-hooks)。**无** token 流专章。Kit：`load` 里的 Promise 会流到浏览器；`+server.js` 可返回 `ReadableStream` / SSE（部分平台会缓冲）。[svelte.dev/docs/kit/load](https://svelte.dev/docs/kit/load)、[svelte.dev/docs/kit/routing](https://svelte.dev/docs/kit/routing)。
- **WebSocket / SSE**：SSE/Stream 在 `+server.js` 一等；**文档无一等 WebSocket 升级**。客户端自接浏览器 WS。
- **双向 RPC / 长 socket vs SSR**：Remote functions 在客户端变成 `fetch` 包装，命中生成的 HTTP 端点；`query.live` 是服务端 iterable 推流，不是双向 socket。[svelte.dev/docs/kit/remote-functions](https://svelte.dev/docs/kit/remote-functions)。`$effect` 只在浏览器跑。[svelte.dev/docs/svelte/$effect](https://svelte.dev/docs/svelte/$effect)。长 socket 必须放在 CSR。
- **TS**：Svelte 编译 `.svelte.ts` / runes；stores 仍有 `svelte/store`。Svelte 5：跨组件状态优先 runes，stores 场景「greatly diminished」但未弃用。[svelte.dev/docs/svelte/stores](https://svelte.dev/docs/svelte/stores)。
- **代码编辑/diff**：**生态、非官方**。
- **纯静态 SPA**：`@sveltejs/adapter-static` + `fallback`（如 `200.html`）= SPA；可不跑 Node。[svelte.dev/docs/kit/adapter-static](https://svelte.dev/docs/kit/adapter-static)、[svelte.dev/docs/kit/single-page-apps](https://svelte.dev/docs/kit/single-page-apps)。后端用另一语言（含 **Rust**）时，官方建议前端与后端分部署，或由后端托管 SPA。[svelte.dev/docs/kit/project-types](https://svelte.dev/docs/kit/project-types)。
- **推向 BFF**：Form actions / remote functions / `+server.js` 会把逻辑放进 Kit 服务器。走 SPA + 忽略 `server` 文件则可避开。[svelte.dev/docs/kit/project-types](https://svelte.dev/docs/kit/project-types)。
- **一句话：采用（adapter-static SPA，传输仍接 Rust host）。** 出处：project-types 的「Rust 后端 + 分部署/SPA」+ adapter-static。勿用 remote functions 当 agent 内核。

---

## 5. Vue 3 + Vite / Nuxt 3

- **官方定位**：Vue 可 SPA、全栈 SSR、SSG。[vuejs.org/guide/extras/ways-of-using-vue](https://vuejs.org/guide/extras/ways-of-using-vue.html)、[vuejs.org/guide/introduction](https://vuejs.org/guide/introduction.html)。Nuxt = Vue **全栈**，**默认 SSR**。[nuxt.com/docs/getting-started/introduction](https://nuxt.com/docs/getting-started/introduction)。
- **高频流式文本**：Vue 是 **编译器知情的 Virtual DOM**（patch flags、tree flattening）；文本绑定走 patch flag，不是 Solid 式逐节点订阅。[vuejs.org/guide/extras/rendering-mechanism](https://vuejs.org/guide/extras/rendering-mechanism.html)。**无** token 流专章。Nuxt streaming 属于 Nitro/SSR HTML，不是客户端细粒度 token。
- **WebSocket / SSE**：Vue 核心无一等。Nuxt 服务器引擎是 Nitro。[nuxt.com/docs/getting-started/introduction](https://nuxt.com/docs/getting-started/introduction)。Nitro **一等** WS（`features.websocket` + `defineWebSocketHandler`）与 SSE（`createEventStream`）。[nitro.build/docs/websocket](https://nitro.build/docs/websocket)。那是 **Nitro 进程** 上的 socket，不是浏览器连 Rust。
- **双向 RPC / 长 socket vs SSR**：Vue SPA 可在 `onMounted` 持有 WS。Nuxt SSR 默认；`ssr: false` 得静态 SPA 壳。[nuxt.com/docs/getting-started/deployment](https://nuxt.com/docs/getting-started/deployment)。Nitro WS 把双向通道放进 JS 服务器，与 CS（内核在 Rust）叠一层 BFF。
- **TS**：Vue「written in TypeScript」「first-class」；官方包带类型。[vuejs.org/guide/typescript/overview](https://vuejs.org/guide/typescript/overview.html)。Nuxt「Zero-config TypeScript」。[nuxt.com/docs/getting-started/introduction](https://nuxt.com/docs/getting-started/introduction)。
- **代码编辑/diff**：**生态、非官方**。
- **纯静态 SPA**：Vue+Vite → `dist`。[vitejs.dev/guide/static-deploy](https://vitejs.dev/guide/static-deploy.html)。Nuxt：`nuxt generate` 或 `ssr: false` 静态托管；`200.html`/`404.html` fallback。[nuxt.com/docs/getting-started/deployment](https://nuxt.com/docs/getting-started/deployment)。
- **推向 BFF**：Vue 库不推。Nuxt/Nitro 提供 `server/api`、hybrid rendering、可选 WS——官方全栈路径。
- **一句话：Vue 3 + Vite → 采用；Nuxt 3 → 仅远程 SSR 壳（或否决作控制台宿主）。** 出处：Vue SPA 定位 vs Nuxt 默认 SSR + Nitro 一等 WS/API。

---

## 6. TanStack Start / Router

- **官方定位**：Router = React/Solid **类型安全路由器**（可纯 SPA）。[tanstack.com/router/latest/docs/framework/react/overview](https://tanstack.com/router/latest/docs/framework/react/overview)。Start = Router 之上的 **全栈**（SSR、streaming、server functions/routes）；**Release Candidate**。[tanstack.com/start/latest/docs/framework/react/overview](https://tanstack.com/start/latest/docs/framework/react/overview)。官方：若 **确定不需要** SSR/streaming/server functions/routes，应只用 Router。[同上 overview]。
- **高频流式文本**：Router/Start UI 仍是 React（或 Solid）渲染。Start 一等流是 **server function 返回 typed `ReadableStream` / async generator**（「thanks to the rise of AI apps」）。[tanstack.com/start/latest/docs/framework/react/guide/streaming-data-from-server-functions](https://tanstack.com/start/latest/docs/framework/react/guide/streaming-data-from-server-functions)。这是 **Start 服务器 → 客户端** 的 typed RPC 流，不是 host WebSocket 上的 TextDelta。
- **WebSocket / SSE**：Start **server routes = HTTP handlers**（GET/POST… `Response`），文档未列 WebSocket upgrade。[tanstack.com/start/latest/docs/framework/react/guide/server-routes](https://tanstack.com/start/latest/docs/framework/react/guide/server-routes)。流式以 Stream/SSE 形态出现在 server functions，**非一等双向 WS**。客户端 WS 自接。
- **双向 RPC / 长 socket vs SSR**：Server functions「On the client, calls become `fetch` requests to the server」。[tanstack.com/start/latest/docs/framework/react/guide/server-functions](https://tanstack.com/start/latest/docs/framework/react/guide/server-functions)。SPA mode **仍鼓励** 搭配 server functions/routes。[tanstack.com/start/latest/docs/framework/react/guide/spa-mode](https://tanstack.com/start/latest/docs/framework/react/guide/spa-mode)。长连接反向 RPC 不是该 RPC 模型。
- **TS**：Router「100% inferred TypeScript」。[Router overview](https://tanstack.com/router/latest/docs/framework/react/overview)。Start 强调跨栈类型安全。[Start overview](https://tanstack.com/start/latest/docs/framework/react/overview)。
- **代码编辑/diff**：**生态、非官方**。
- **纯静态 SPA**：Router + Vite = 静态 SPA。Start SPA mode 预渲染 `/_shell.html`，CDN 即可；若仍暴露 `/_serverFn/*` 则 **不是** 纯静态。[Start SPA mode](https://tanstack.com/start/latest/docs/framework/react/guide/spa-mode)。
- **推向 BFF**：Start 的核心卖点就是 server functions/routes。Router-only 则不推。
- **一句话：Router + Vite → 采用；Start → 仅远程 SSR 壳。** 出处：Start overview「不需要全栈就用 Router」+ server functions = `fetch` RPC，非长连接 WS。

---

## 对照（相对 CS 约束，仍是上游能力图，不是选型决议）

| 卡点 | 够用（官方能撑客户端壳） | 卡在哪（一手限制） |
|---|---|---|
| 长连接双向 + 反向 RPC | 所有候选的 **CSR** 都能用浏览器 WebSocket 自接 | Next Route Handler **官方否定 WS**；Start/Nuxt/Kit 的一等实时是 **自家 JS 服务器** 上的 Stream/WS/RPC |
| 高频 TextDelta | Solid/Svelte 官方细粒度；Vue 编译器 VDOM；React 整树协调 | 无一家给「token 流 UI」一等 API；React/Next 官方 streaming = HTML/Suspense 或 HTTP 流 |
| 禁止前端 ReAct / 内核在 Rust | Vite SPA、Kit static SPA、Vue SPA、Solid+Vite、Router | Next/Nuxt/Start/Kit remote 把 mutation 放进元框架服务器 |
| 纯静态 self-host | Vite `dist`；Kit adapter-static；Nuxt `ssr:false`/`generate`；Next `output:'export'`（砍动态能力）；Start shell | 全栈默认都要 Node/Nitro/Next runtime |
| Cursor TS SDK（可选、客户端 npm） | 任意 CSR 可 import | RSC/Server 模块不能直接用浏览器 SDK，须 `'use client'` [react.dev use-client](https://react.dev/reference/rsc/use-client) |
| Monaco/CodeMirror | — | 六家官网均未作为一等编辑器；**生态、非官方** |
