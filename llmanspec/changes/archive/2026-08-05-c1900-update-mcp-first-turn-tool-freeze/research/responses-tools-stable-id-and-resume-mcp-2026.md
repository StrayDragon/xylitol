# Responses `tools` stable ID 与 resume MCP（2026-08-05）

> **问题**：新会话首轮后冻结 `tools[]` 有利于 llama.cpp 前缀缓存；resume 后 MCP
> 服务器、schema 或可执行器可能已经变化。是否有 OpenAI Responses / Codex 机制能把历史
> `mcp:…` 调用稳定映射到新工具集，同时不重写稳定前缀？
>
> **证据范围**：仅 OpenAI 官方文档 / Responses API reference，及本机
> `../codex/codex-rs/` 源码与测试；未把兼容端的 Responses 形状当成 OpenAI 语义。

## 执行摘要

- **稳定 definition ID + 占位/重映射：否。** Responses 的 `function` 定义有
  `name`，没有可由客户端指定的 `id`；`id` / `call_id` 是一次 *调用* 的 response
  item 标识，不能作为工具 definition 的稳定别名。官方 schema 也没有
  `placeholder`、`slot`、`alias`、`replace` 或 `remap` 字段。
- **保持稳定前缀的真实机制：部分有。** `tool_search` + `defer_loading` 将发现到的
  工具作为轨迹末尾的 `tool_search_output` 载入；官方明确说这会保留 cache。客户端执行
  的 tool search 还能返回首个 `tools[]` 中未声明的受信 definition。但这是**追加的
  历史事件**，不是把旧 definition 映射成新 definition，也不能无代价撤销或改写已载入
  的工具。
- **Codex 不靠 stable ID。** 它在 resume 时重放 rollout history，同时用当前
  `Config` 重建 MCP runtime；每个 model sampling request 捕获当时的 MCP binding。
  其支持的路径是 client `tool_search_output` 历史，不是把旧 `mcp__…` 名重映射到新
  server/tool。所读 resume 路径中没有这种 reconciliation。
- **结论给 xylitol**：`c1900` 的“首轮定稿后冻表”仍是新会话、完整全量表的正确策略；
  resume 必须是一个显式兼容性决策。静态表字节/工具语义均相同才继续同一 epoch；否则
  开新的 tools/context epoch，并接受 cache bust。不要把“按 name 重绑”称为安全恢复。
  原生 OpenAI 上 `c1960` 的 client `tool_search` 是后来追加能力的较好路线；它不能
  消除删除、重命名或语义变更的 resume 决策。

| 问题 | 结论 |
|---|---|
| 给 `function` 定义稳定 `id`？ | **否（文档化 schema 中无此字段）** |
| 预留空 tool 并以后 remap？ | **否（无 placeholder/slot/remap 协议）** |
| 静态 `tools[]` 而动态发现？ | **部分可行**：`tool_search`、namespace、MCP `defer_loading`、`additional_tools` |
| 已载入工具可静默删除/替换且保 cache？ | **否**；官方明确说修改 loaded set 会从该点破坏 cache |
| `previous_response_id` 解决 tool identity？ | **否**；它是 response-state chain，不是 tool-definition 映射 |
| xylitol：仅删 MCP 后 `input` 前缀？ | **可不变**；`tools[]` 仍变 → cache 仍 bust（见下文实证） |
| xylitol：删内建后 `input` 前缀？ | **必变**（Available-tools 进 system）；与 `tools[]` 双 bust |

## 官方 Responses wire：相关对象的精确字段

以下是与本题有关的 `tools[]` 变体；不是对 web/code/image 等所有 built-in tool 的穷举。
字段来自 [Responses `create` API reference](https://developers.openai.com/api/reference/resources/responses/methods/create)；
function 与 namespace 的可读示例见 [Function calling](https://developers.openai.com/api/docs/guides/function-calling)。

| 变体 | 文档化字段 | 对稳定性意味着什么 |
|---|---|---|
| `function` | `type`, `name`, `parameters`, `strict`, `allowed_callers`, `defer_loading`, `description`, `output_schema` | 没有 definition `id`；`name` 才是可调用名称。`defer_loading` 是发现控制，不是引用/映射 ID。 |
| `namespace` | `type`, `name`, `description`, `tools` | `tools` 内仍是命名的 `function` / `custom`。namespace 是分组，不是可重填的 slot。 |
| `tool_search` | `type`, `description`, `execution`, `parameters` | `execution: "server"` 为 hosted；`"client"` 由应用检索并回传 output。没有工具表 revision 或 remap 字段。 |
| `mcp` | `type`, `server_label`, `allowed_callers`, `allowed_tools`, `authorization`, `connector_id`, `defer_loading`, `headers`, `require_approval`, `server_description`, `server_url`, `tunnel_id` | `server_label` 识别 server；不是 versioned tool-definition ID。`server_url` / `connector_id` / `tunnel_id` 三者之一必需。 |

### 不要混淆 definition identity 与 call identity

- `function_call` output item 带 API 生成的 `id` 和 `call_id`；应用以该次调用的
  `call_id` 回传 `function_call_output`。这只关联一次 invocation，不能标识一个可复用
  definition。[Function-calling 调用/回传流程](https://developers.openai.com/api/docs/guides/function-calling)
- `tool_search_call.call_id` 也只把 client 的 `tool_search_output` 关联回那次搜索。
  Hosted mode 的 `call_id` 为 `null`。[Tool search](https://developers.openai.com/api/docs/guides/tools-tool-search)
- `additional_tools` item 自身可有 item `id`，但其 role 为 `developer`，且它把实际
  `tools` 放入某个历史位置；不是“在既有 slot 中换实现”的协议。

因此，不能把 `call_id`、`function_call.id`、`server_label` 或 namespace name 假定为
可安全跨 MCP schema/行为变更的 stable definition ID。

### `tool_choice.allowed_tools`：有用但不是映射

[Function calling 的 allowed-tools 文档](https://developers.openai.com/api/docs/guides/function-calling)
给出：

```json
{
  "type": "allowed_tools",
  "mode": "auto",
  "tools": [
    { "type": "function", "name": "get_weather" }
  ]
}
```

它位于 request 的 `tool_choice`，目标正是“不修改传入的完整 tools list”以获得 prompt
cache 收益。它可在**固定、已声明的超集**中暂时禁用某些 function，但：

1. 只按 `type` + `name` 选择，没有 stable ID / version / remap；
2. 不能引入原超集不存在的新 MCP tool；
3. 不能让已删除的 server 真正恢复执行能力，也不解释历史工具调用的语义。

## `tool_search`、namespace 与尾部注入

### Hosted tool search

OpenAI 的 hosted 路径要求在初始 `tools[]` 里声明 deferred functions、namespaces 或 MCP
server，并加入 `{ "type": "tool_search" }`。对于 namespace/MCP，模型起初只看见
高层 name/description；选中后服务端输出 `tool_search_call` 和
`tool_search_output`，再发出 function call。

- 对 function：`defer_loading: true` 主要延后参数 schema；
- 对 namespace：`defer_loading` 放在内部 function，不放 namespace 本身；
- 对 MCP：`defer_loading: true` 放在 MCP tool definition，模型仍可看见
  `server_label` / `server_description`。

来源：[Tool search](https://developers.openai.com/api/docs/guides/tools-tool-search)、
[MCP/Connectors：defer loading](https://developers.openai.com/api/docs/guides/tools-connectors-mcp)。

这会稳定**已知**的顶层 declaration，但不能令“resume 后新增/删除 MCP server”不改顶层
`tools[]`：hosted search 仍需声明可搜索的 server/namespace。

### Client-executed tool search：本题最接近的能力

`tools[]` 可只含稳定的 `tool_search` meta-tool：

```json
{
  "type": "tool_search",
  "execution": "client",
  "description": "Find project-specific trusted tools.",
  "parameters": { "type": "object" }
}
```

模型先输出 `tool_search_call`；应用从自己的 MCP registry 检索，然后以
`tool_search_output { type, execution, call_id, status, tools }` 回传 selected definitions。
官方明确允许这个 advanced workflow 返回“最初 request 未出现”的受信 tools。

其关键 cache 性质是：**所有发现到的 tools 都在 context window 末尾载入**，官方说这
可保留前缀 cache。新工具能作为新尾部 item 出现，不必重写首轮 `tools[]`。但 official
guide 也明确规定：

- `tool_search_output.tools` 中的 definitions 在未来 turns 可调用；
- 未出现的 tool 不可调用；
- 若想 disable 已 loaded tool，修改该 `tool_search_output` 的 loaded set，**会从该点
  break cache**；
- 手工 round-trip 时 `additional_tools` 必须保留原始位置，否则模型看见的顺序不同。

所以它是 append-only discovery，而非可撤销的 placeholder table。

## Codex：resume、rollout 与 MCP 的实证读码

### Resume 分成“历史重建”和“当前 runtime”两条线

1. [`thread_manager.rs`](../../../codex/codex-rs/core/src/thread_manager.rs) 的
   `resume_thread_from_rollout` / `resume_thread_with_history`（约 871–924）先读 rollout，
   用 `InitialHistory::Resumed` 保存已持久化的 `history.items`（约 1830–1844），再用**传入
   的当前 `Config`** spawn 新 thread。
2. [`session/mod.rs`](../../../codex/codex-rs/core/src/session/mod.rs) 的
   `record_initial_history`（约 1290–1455）对 `Resumed` history 做
   `apply_rollout_reconstruction`，把重建出的 `ResponseItem` history 放回 context manager。
   这保留历史中的 `tool_search_output` / function call output，而不是重新生成它们。
3. 同一新 session 的 [`session/mcp_runtime.rs`](../../../codex/codex-rs/core/src/session/mcp_runtime.rs)
   `install_initial_mcp_runtime`（约 82–114）从当前 per-turn config 建 `McpRuntimeInput` 并
   `replace` runtime。[`codex-mcp/src/runtime.rs`](../../../codex/codex-rs/codex-mcp/src/runtime.rs)
   说明它拥有“one Codex thread”的 mutable MCP state，publication 原子替换当前 snapshot。
4. 每个 sampling step 通过
   [`session/mcp.rs`](../../../codex/codex-rs/core/src/session/mcp.rs) `mcp_runtime_for_step`
   （约 278–306）获取 binding。[`codex-mcp/src/binding.rs`](../../../codex/codex-rs/codex-mcp/src/binding.rs)
   明确该 binding 是“one model sampling request”的 exact tool catalog/execution handles。

这意味着：**历史与当前 MCP catalog 不是同一个持久化工具表。** 在上述 resume path 未见
“历史 `mcp__server.tool` → 新 server/tool definition”的 stable-id or name-remap 层。
当前 catalog 由新 runtime 构建；历史 response items 由 rollout 重放。

### Codex 的 deferred 路径

- [`mcp_tool_exposure.rs`](../../../codex/codex-rs/core/src/mcp_tool_exposure.rs:22-87) 在 search
  可用时将普通 MCP tools 注册为 `ToolExposure::Deferred`，否则为 `Direct`。
- [`tools/spec_plan.rs`](../../../codex/codex-rs/core/src/tools/spec_plan.rs:315-362,1129-1150)
  收集 deferred search metadata，注册 `tool_search` executor；model-visible `tools[]` 只含
  direct specs / meta-tool。
- [`tools/handlers/tool_search.rs`](../../../codex/codex-rs/core/src/tools/handlers/tool_search.rs)
  用 BM25 搜索 `ToolSearchInfo`，返回 namespace/function definitions，并把
  `defer_loading: true` 写回 loaded definition。
- 集成测试
  [`core/tests/suite/search_tool.rs`](../../../codex/codex-rs/core/tests/suite/search_tool.rs:549-834)
  断言：首请求有 `tool_search`、没有待发现 MCP tool/namespace；第二、三请求也**不**
  重新把该 tool 注入 `tools[]`，而是依赖历史中的 client `tool_search_output`。

这正是“稳定 meta-tool 前缀 + 轨迹尾部加载”的实现，而不是 placeholder 重映射。

### Codex 对 resume × MCP 的现有保护边界

- [`core/tests/suite/mcp_tool_exposure.rs`](../../../codex/codex-rs/core/tests/suite/mcp_tool_exposure.rs:645-714)
  验证**未变化**的 deferred-tool world state resume 后不重复注入。
- 同文件约 552–604 的 live refresh 测试在关闭 Calendar 后产生“removed deferred tool
  namespaces”状态更新：这是把变化告知模型，不是维持旧 definition 的别名。
- [`app-server/tests/suite/v2/thread_resume.rs`](../../../codex/codex-rs/app-server/tests/suite/v2/thread_resume.rs:4266-4309)
  还验证 required MCP server 在 resume 时无法初始化则 resume 失败，进一步表明 resume
  依赖当前 runtime 可用性。

所读测试没有覆盖“resume 后 server/tool rename 或 schema change，历史调用自动映射”；
不应从 Codex 的 tool search 机制推导出这种保证。

## 对 xylitol 的含义

### `c1900`：首轮冻结仍成立，但只对兼容 resume 成立

1. 在新 session，继续首条前等待 MCP settle、一次性定稿、之后不改 provider-visible
   `tools[]`；这与 `c1900` 的实验结论一致。
2. 持久化一个 **static tool-root fingerprint**：canonical serialized `tools[]`、model-visible
   namespace/server descriptions、排序、以及每个 definition 的
   `(namespace, name, schema hash, executor/semantic version)`。不要只存 name。
3. resume 时先比较 fingerprint：
   - **完全兼容**：复用记录的 frozen root，并按原位置重放历史 outputs；不用“重新定稿”。
   - **不兼容**（MCP add/remove/rename/schema/implementation 改变）：显式开新的
     tools/context epoch，重建或从新边界开始发请求，并承认 prefix cache 已断。
4. 历史里已加载但当前不可执行的工具，不可静默按同名绑定到不同语义。选择其一：
   兼容 executor/tombstone，或在真正调用时回传确定性的“tool unavailable after resume”
   `function_call_output`；再让模型搜索当前替代工具。

也就是说，`/reload` 的“重定稿”应是可见的 epoch transition，不是声称仍在同一 frozen
session。

### `c1960`：把原生 tool search 定为另一条 wire policy

- **OpenAI native 优选**：client-executed `tool_search` + xylitol 的受信 MCP registry。
  顶层保持核心工具与稳定 search meta-tool；新发现 definition 作为尾部
  `tool_search_output` 追加。它最符合“动态增加不重写前缀”。
- **Hosted search 限制**：它适合“本请求创建时已知的完整 inventory”；MCP server
  declaration 本身变了仍会改变 initial `tools[]`。
- **namespace 的作用**：为稳定、高层、可搜索分类服务；不能当空位或 ID namespace。
- **`allowed_tools` 可选补充**：若 session 起始已知一个固定超集，可限制本 turn 可调 subset
  而不改完整 list；不要用它伪装新增或替换 MCP tool。
- **兼容端隔离**：现有 Ornith/llama.cpp lab 已记录 hosted `tool_search` 被静默剥离、
  `tool_search_*` input item 400、`defer_loading` 被忽略。它必须继续走明确
  `WirePolicy` / function fallback，不能因 endpoint 名为 Responses 而启用该路径。

## `previous_response_id` / stored response 不会改变工具身份结论

[Conversation state](https://developers.openai.com/api/docs/guides/conversation-state) 规定
`previous_response_id` 用于把 response 链接成 threaded conversation；response 默认保存
30 天，`store: false` 可关闭保存，且链中先前 input tokens 仍计费。API reference 还说明：
新 request 使用 `previous_response_id` 时，前一 response 的 `instructions` 不会自动继承。

这能减少客户端重复传输/管理 history，并可能让 OpenAI server 继续已有 state；但文档没有
把它定义为：

- tool definition 的 stable ID；
- 旧 MCP call 到新 MCP catalog 的 mapping；
- 在 tools/schema/server 改变时仍可保 cache 的兼容性协议。

因此应把它当 OpenAI-only 的可选 state transport，而非 `c1900` 的 resume 正确性基础。
本题已知 llama.cpp 拒绝 `previous_response_id`，故 xylitol 必须保留全量重放与明确 epoch
fallback；不要为这条优化移除该路径。

## xylitol 实证：删工具后 Responses 前缀 / cache（2026-08-10）

> 闸内单测（`agent::llm_project`）：`responses_assemble_after_mcp_remove_keeps_input_busts_tools`、
> `responses_assemble_after_builtin_remove_rewrites_input_and_tools`。
> Assembler 把 system 预置进 `input[0]`（thinking on → `developer`）；`tools[]` 为顶栏独立字段。
> Available-tools 文本经 `default_prompt_base` **滤掉** `is_mcp_tool_name` 名。

| 变更 | System Available-tools | Responses `input`（含 system 项） | 顶栏 `tools[]` | Prompt-cache 含义 |
|---|---|---|---|---|
| **仅删 MCP** | **不变**（MCP 本不进 Available-tools） | **可 byte-stable**（历史 `function_call` 仍在） | **变**（定义消失） | **`tools[]` 必 bust**；历史前缀可不 bust |
| **删内建**（如 `bash`） | **改写**（列表少一行） | **变**（system/developer 项变） | **变** | **前缀 + tools 双 bust** |
| 历史里已发生的 ToolCall | — | **保留**（`project_for_llm` 不抹） | 与当前 catalog 无关 | call identity ≠ definition identity |

产品含义（相对上文「冻表 / epoch」）：

- `/reload` 只动 MCP 表：下一轮请求仍会因 `tools[]` 变化断 cache；但 **system 前缀与历史
  `input` 可保持稳定**——不是「无代价」，而是 bust 面比删内建更窄。
- 若未来允许会话中途减内建工具（或改 Available-tools 文案），那是**显式前缀 epoch 断点**，
  与 MCP-only 不可混谈「只 bust tools」。
- 与官方一致：改 loaded / 声明的 tools set 从该点破坏 cache；xylitol 没有 stable definition
  remap 可绕过。

## 待验证问题与下一轮实验

1. **OpenAI client search + resume matrix**：保存完整 `tool_search_call/output` 历史后，
   分别新增、删除、rename、仅 schema 变更一个 MCP tool；验证历史 loaded tool 是否仍
   callable、如何返回不可用错误、以及新 tool 是否只以尾部 output 出现。
2. **`previous_response_id` 语义**：在相同与不同 `tools[]`、省略/传入 `tools`、以及已有
   `tool_search_output` 的组合中抓 request/response。官方文档未说明这些组合是否继承
   tool availability，不能猜测。
3. **cache 可观测性**：对 OpenAI 原生 / Ornith 记录 `cached_tokens`（或等价）；对照
   MCP-only `tools[]` 变更 vs 内建 Available-tools 改写，验证上表「窄 bust / 双 bust」
   在 live 网关上是否可观测（维护 lab：`lab_resume_prompt_cache`，不进 qa）。
4. **Ornith/llama.cpp compatibility probe**：逐一测 `namespace`、`tool_search`
   client mode、`tool_search_output`、`additional_tools`、`allowed_tools` 与
   `previous_response_id`；把接受/忽略/400 固化为 WirePolicy 测试矩阵，禁止自动猜测。
5. **xylitol resume BDD**：覆盖 fingerprint equal、MCP removed、same name/new schema、
   provider reload、历史 tool output replay；断言既不静默重绑，也不在声称 cache-safe 的
   epoch 中重写 frozen root。装配层「MCP vs 内建删工具」前缀差异已由上节单测锁住。

## 一手来源

### OpenAI

- [Responses API — create reference](https://developers.openai.com/api/reference/resources/responses/methods/create)
- [Function calling](https://developers.openai.com/api/docs/guides/function-calling)
- [Tool search](https://developers.openai.com/api/docs/guides/tools-tool-search)
- [MCP and connectors](https://developers.openai.com/api/docs/guides/tools-connectors-mcp)
- [Conversation state](https://developers.openai.com/api/docs/guides/conversation-state)

### Codex source

- `../../../codex/codex-rs/core/src/thread_manager.rs`
- `../../../codex/codex-rs/core/src/session/mod.rs`
- `../../../codex/codex-rs/core/src/session/mcp.rs`
- `../../../codex/codex-rs/core/src/session/mcp_runtime.rs`
- `../../../codex/codex-rs/codex-mcp/src/runtime.rs`
- `../../../codex/codex-rs/codex-mcp/src/binding.rs`
- `../../../codex/codex-rs/core/src/mcp_tool_exposure.rs`
- `../../../codex/codex-rs/core/src/tools/spec_plan.rs`
- `../../../codex/codex-rs/core/src/tools/handlers/tool_search.rs`
- `../../../codex/codex-rs/core/tests/suite/search_tool.rs`
- `../../../codex/codex-rs/core/tests/suite/mcp_tool_exposure.rs`
- `../../../codex/codex-rs/app-server/tests/suite/v2/thread_resume.rs`
