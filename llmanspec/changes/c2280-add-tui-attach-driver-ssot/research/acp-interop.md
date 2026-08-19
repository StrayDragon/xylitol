# ACP 外部侧表面调研（一手来源）

> 本页回答：**xylitol 是否该在 host core 上额外暴露一个 ACP "侧表面"**，让 Goose / GitHub Copilot 等其它 agent 客户端 attach 到 xylitol host core。只引官方一手来源：`agentclientprotocol.com` 文档（各页以 `.md` 后缀可取原文）、`github.com/agentclientprotocol/agent-client-protocol`、各 SDK 仓库。无 blog 转述。
> 本文是 c2280 的中立事实页；"做/不做/何时做"由 `proposal.md` 裁决。c2301 已定：**AcP 若支持，以"外层适配器"消费 native 契约，不进核心协议闭集**（见 `llmanspec/changes/c2301-update-stable-wire-protocol/proposal.md` 第 43 行）。

## 1. 规范所在与状态

- **规范主站**：`agentclientprotocol.com`；**源码仓库**：`github.com/agentclientprotocol/agent-client-protocol`（[Overview 文档索引 `llms.txt`](https://agentclientprotocol.com/llms.txt)）。协议本身是**官方文档 + JSON Schema + SDK**，类型在 schema 定义（[v1 Schema](https://agentclientprotocol.com/protocol/v1/schema)、[v2 Schema](https://agentclientprotocol.com/protocol/v2/schema)）。
- **v1 为稳定版**（`protocolVersion: 1`，见 [session-setup](https://agentclientprotocol.com/protocol/v1/session-setup) 的 `initialize` 示例）。
- **v2 于 2026-07-20 发布 Draft**（[acp-v2-draft 公告](https://agentclientprotocol.com/announcements/acp-v2-draft)）；"v2 is a Draft … various pieces can, and will, change before stabilization"，且**明确要求 v1 与 v2 并存**，guide "gate your implementation behind version negotiation AND feature flags"。自 v1 起已用 RFD 流程落地 15+ 项特性（同页）。
- **治理**：**Zed 与 JetBrains 联合治理**，双 lead maintainer = Ben Brandt（Zed）、Sergey Ignatov（JetBrains），BDFL；下辖 core maintainers / maintainers / contributors，并设有 Working/Interest Groups（[governance](https://agentclientprotocol.com/community/governance)、[sergey-ignatov-lead-maintainer](https://agentclientprotocol.com/announcements/sergey-ignatov-lead-maintainer)）。所有 RFD 经公开流程变更（[RFD 流程](https://agentclientprotocol.com/rfds/about)）。
- **SDK**：Rust / TypeScript 已到 **1.0.0**（[sdk-1-0-releases](https://agentclientprotocol.com/announcements/sdk-1-0-releases)；[rust-sdk](https://github.com/agentclientprotocol/rust-sdk)、[ts-sdk](https://github.com/agentclientprotocol/typescript-sdk)）。

## 2. 传输层（Streamable HTTP & WebSocket RFD）

- 现状主传输是 **stdio**（客户端把 agent 当子进程拉起，stdin/stdout 传换行分隔 JSON-RPC）（[v1 Transports](https://agentclientprotocol.com/protocol/v1/transports)）。
- **新增远程传输 RFD**：[Streamable HTTP & WebSocket Transport](https://agentclientprotocol.com/rfds/streamable-http-websocket-transport) 当前 **Active**（修订史 2026-07-02 "Moved to Active"，Transports Working Group 主导）。提出**单一 `/acp` 端点、两种连通档案**：
  - **Streamable HTTP（POST/GET/DELETE）**：`initialize` 返回 200 + JSON（含 `Acp-Connection-Id`）；其余 POST 返回 202，响应走 **长连接 GET SSE 流**（一条 connection-scoped + 每条 session 一条 session-scoped），用 JSON-RPC `id` 关联。要求 **HTTP/2**。
  - **WebSocket（GET + `Upgrade: websocket`）**：同一端点升级为全双工，文本帧承载 JSON-RPC；**服务器可只支持 WebSocket**，而客户端 MUST 两者都支持。
  - 身份用双头 `Acp-Connection-Id` + `Acp-Session-Id`；session 归属经 JSON-RPC 体里的 `sessionId`。
- 语义层始终 **JSON-RPC 2.0**（Method=request-response，Notification=单向；[v1 Overview](https://agentclientprotocol.com/protocol/v1/overview)）；与 stdio/local 用**同一套 JSON-RPC 消息与生命周期**（传输 RFD 第 21 行）。参考实现进行中在 **Goose**（`block/goose`，传输 RFD Phase 2）。
- **可靠性分代**（传输 RFD "Durability and reliability expectations"）：
  - **v1**：会话跨断线存活（可 `session/load` 重连）；**不重放断线期间消息、无消息序号、无流续传**——reconnect/retry/liveness 全是实现方责任。
  - **v2**：流式消息带 ID（"last replay ID"）、SSE `Last-Event-ID` 式**流可续传**、定义重连语义、标准化 keepalive。

## 3. 词汇与角色（provider vs client）

- **角色**：**Agent**=agent runtime（"typically run as subprocesses of the Client"，即 server 侧/提供方）；**Client**=IDE/UI（"manage the environment, handle user interactions, and control access to resources"）（[v1 Overview](https://agentclientprotocol.com/protocol/v1/overview)）。**xylitol host core 对应 Agent（provider）角色**。
- **会话（session 组）**：[v1 Overview](https://agentclientprotocol.com/protocol/v1/overview) + [session-setup](https://agentclientprotocol.com/protocol/v1/session-setup)：
  - `session/new`（建会话：`cwd` + `mcpServers`）→ 返回 `sessionId`；`session/load`（**整段重放**历史再应答，需 `loadSession` 能力）；`session/resume`（**不重放**，恢复上下文即应答，需 `sessionCapabilities.resume`）；`session/close`（取消在途 + 释放资源）；`session/list`（[已稳定](https://agentclientprotocol.com/announcements/session-list-stabilized)）；`session/delete`（[已稳定](https://agentclientprotocol.com/announcements/session-delete-stabilized)）。
  - 一个连接可挂多条 session；多 client 可 load/resume 同一 session（无"单写者"概念，见 §5）。
- **消息（message 组）**：`session/prompt`（发用户消息：`ContentBlock[]`）；Agent 经 **`session/update` notification** 流式回报：`agent_message_chunk` / `agent_thought_chunk` / `tool_call` / `tool_call_update` / `plan` / `usage_update`；流式 chunk 同 `messageId` 归一条消息（[prompt-turn](https://agentclientprotocol.com/protocol/v1/prompt-turn)、[tool-calls](https://agentclientprotocol.com/protocol/v1/tool-calls)）。`session/prompt` 响应带 `stopReason` 结束回合。
- **权限（permission 组）**：`session/request_permission`（server→client 双向请求）带 `options`（`allow_once/allow_reject_once/…`），Client 回 `outcome`（`selected`/`cancelled`）（[tool-calls](https://agentclientprotocol.com/protocol/v1/tool-calls)）；v2 改为通用 `title`/`description` + `subject` tagged union（tool_call / command，见 [v2-permission-requests](https://agentclientprotocol.com/rfds/v2/permission-requests)）。
- **文件系统与终端（fs/terminal 组，client 侧能力）**：`fs/read_text_file` / `fs/write_text_file` 读写**客户端**环境（含未保存编辑态）（[file-system](https://agentclientprotocol.com/protocol/v1/file-system)）；`terminal/*`（create/output/release/wait_for_exit/kill）执行命令（[Overview 方法表](https://agentclientprotocol.com/protocol/v1/overview)）。
- **结构化提问**：`elicitation/create`（form/URL 两模式，[已稳定](https://agentclientprotocol.com/announcements/elicitation-stabilized)，[elicitation](https://agentclientprotocol.com/protocol/v1/elicitation)）。
- **取消**：`session/cancel`（取消整回合）+ 通用 `$/cancel_request`（按 JSON-RPC id 取消单个请求，[已稳定](https://agentclientprotocol.com/announcements/request-cancellation-stabilized)；[request-cancellation](https://agentclientprotocol.com/rfds/request-cancellation)）。
- 扩展机制：`_meta` 字段、`_` 前缀自定义方法、初始化时声明自定义 capabilities（[v1 Overview](https://agentclientprotocol.com/protocol/v1/overview)；v2 向前兼容性更好，[acp-v2-draft](https://agentclientprotocol.com/announcements/acp-v2-draft)）。

## 4. 采用与生态（谁以什么角色讲 ACP）

- **GitHub Copilot**：以 **Agent（provider）** 角色接入——"ACP support in Copilot CLI is now in public preview"（[github.blog changelog 2026-01-28](https://github.blog/changelog/2026-01-28-acp-support-in-copilot-cli-is-now-in-public-preview/)，via [agents 列表](https://agentclientprotocol.com/get-started/agents)）；并有官方演讲 **"Agent Client Protocol in GitHub Copilot"**（Copilot Dev Days Shanghai, 2026-04-11，[publications](https://agentclientprotocol.com/publications)）与 [bilibili 视频](https://www.bilibili.com/video/BV1bDQ4B9Eri/)。
- **Zed**：**Client（IDE）**，内置 ACP client（[zed docs](https://zed.dev/docs/ai/external-agents)；[clients 列表](https://agentclientprotocol.com/get-started/clients)）。
- **Goose（Block）**：既是 **Agent/provider**（[goose docs](https://block.github.io/goose/docs/guides/acp-clients)）也是**参考 transport 实现宿主**（传输 RFD Phase 2）。
- **其它 provider**：Claude Agent、Codex CLI、Gemini CLI、Cursor CLI、Qwen Code、OpenCode、Pi 等（[agents 列表](https://agentclientprotocol.com/get-started/agents)）；**其它 client**：JetBrains AI Assistant、neovim（CodeCompanion/avante 等插件）、VS Code 扩展、Qt Creator、Obsidian 插件、一众桌面/CLI（[clients 列表](https://agentclientprotocol.com/get-started/clients)）。生态已相当大。

## 5. 与 xylitol native 契约的映射可行性

对照 native 契约（`llmanspec/specs/protocol-app/spec.toon`、`server-core` spec、`c2300`/`c2301` 提案）：

| xylitol native | ACP 对应 | 映射 |
|---|---|---|
| `session/new` / 会话生命周期 | `session/new` / `session/list` / `session/close` | **近 1:1** |
| journal 重连按 `last_seq` 回放 + `ResyncRequired` | v1 `session/load`（整段重放）/ `session/resume`（不重放）；v2 `session/resume`+`replayFrom` 游标（[v2-session-resume-replay](https://agentclientprotocol.com/rfds/v2/session-resume-replay)） | **概念对齐**：ACP 的 resume(no-replay)≈订阅续传、load(replay)≈自序重放。但 **ACP v1 流没有序号/断线重放**（§2），`ResyncRequired` 的"容量满→强制全量重同步"是 xylitol 独有语义，**外层适配器需自实现** |
| 反向 RPC（`approve`/`question`，first-wins） | `session/request_permission`（server→client）+ `elicitation/create` | **近 1:1**；ACP 无"首个应答生效"细节，但语义同为 client 回决定，适配层可保证 first-wins |
| `Command::Abort` | `session/cancel` + `/cancel_request` | **近 1:1** |
| **steer / follow-up / 队列**（`Steer`/`FollowUp`/`ClearQueue`） | v1 面向回合，**无队列原语**；v2 "Moving beyond the turn" 引入后台 `session/update` + idle `state_update`（[acp-v2-draft](https://agentclientprotocol.com/announcements/acp-v2-draft)、[v2 overview](https://agentclientprotocol.com/protocol/v2/overview)） | **v1 缺，v2 才对齐**；需 v2 或自定义扩展 |
| **一 session 一写者 + 只读静态第二 attach**（c2300/c2301） | **ACP 无此规则**；多 client 可同时 load/resume 同一 session，v2 明确支持"multiple clients observing the same session"（[acp-v2-draft](https://agentclientprotocol.com/announcements/acp-v2-draft)） | **必须靠外层适配器桥接**：适配层自己实现写者锁，把"只读第二 attach"体现为"可 resume 但 prompt 返回只读错误" |

**结论**：会话/消息/权限/取消有**天然近 1:1** 映射（都是"事件流 + 双向请求"形态）。**缺口集中在三处，都需外层适配器桥**：(a) xylitol 的 `last_seq` 精确续传/`ResyncRequired` 全量重同步——ACP v1 不重放，v2 才提供；接入 v1 的通用 ACP 客户端会失去 xylitol 的强续传，除非适配层自实现；(b) steer/follow-up/队列——需 v2 或自定义；(c)**单写者+只读 attach 是 xylitol 独家产品语义，ACP 没有，必须由适配层强制执行**。

## 6. 作为 ACP provider 的成本/范围门

- xylitol host 已在 serve 内拥有：**session registry、tagged JSON Command/Event + dispatch、journal（`last_seq`/resync）、反向 RPC 网关、WebSocket**（`server-core`：`server-runtime`/`server-ws`/`server-reverse-rpc` spec；`protocol-app`）。因此 provider 侧地基已具备。
- **补的是一层"信封+方法表"适配**（非第二套核心语义）：`/acp` 端点（HTTP POST/GET/DELETE + WS upgrade）、JSON-RPC 序列化、`initialize`/`session/*`/`request_permission`/（可选 `fs/*`、`terminal/*`、`elicitation/*`）方法表、`Acp-Connection-Id`/`Acp-Session-Id` 身份映射、以及 §5 的续传/队列/写者锁三处桥接。Streamable HTTP 需 **HTTP/2**；**只做 WebSocket 最简单**（服务器可仅支持 WS，传输 RFD §2）。
- **可以结构化为"消费 native 契约的外层适配器"**：适配器把 ACP JSON-RPC method 翻译成 native `Command`，把 native `Event` 翻译成 `session/update` notification 推送；不复制 ReAct/会话/工具语义。这与 c2301 第 43 行一致，也避免"第二协议作为核心语义"。Rust SDK（`agentclientprotocol/rust-sdk`）1.0 可复用做客户端互操作测试。

## 决策相关要点

- **角色**：xylitol host 是 **ACP Agent/provider**；外部 Goos/桌面/GitHub Copilot 类是 **client**。先决定是否值得为它们开这一侧表面。
- **范围**：v1 方法闭集（session/message/permission/fs/terminal/elicitation）规模可控；**需要 v2 才有队列/steer 与流续传**——若目标是"完整承载 xylitol 的 steer+强续传"，倾向 v2（Draft 中）需评估其未稳定风险。
- **时序**：现在核心 wire/SD（c2301 及后续）尚未定型；ACP 侧建议排在 native 协议稳定 + serve 多会话之后，作独立 change（`depends_on` 于核心 wire）。
- **适配器 vs 核心**：**明确适配器**——消费 native Command/Event；不复制语义。三处必桥：`last_seq`精确续传 vs ACP v1 不重放；steer/队列（v2 才有）；**单写者+只读第二 attach 需适配层强制执行（ACP 无此概念）**。
- **传输选型**：只做 WebSocket（免 HTTP/2 负担）即可满足多数桌面 client；Streamable HTTP 面向负载均衡/serverless 场景，非必选。
- **对象**：Rust SDK 1.0 可作互操作锚；参考实现（Goose）可作对照测试。
