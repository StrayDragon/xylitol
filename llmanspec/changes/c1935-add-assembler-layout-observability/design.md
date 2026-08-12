# Design: c1935-add-assembler-layout-observability

> 阶段：Designed / pre-start。本文只描述规划，不执行 `change start`、Specs landing 或代码修改。
>
> 硬依赖：已归档的 `c1890-add-responses-context-policy-assembler`。本 change 不重新设计
> `ContextPolicy`、`WirePolicy` 或 Responses body。

## 1. 目标与事实基线

目标是让一次 Responses 请求可以回答两个相邻但不同的问题：

1. **规则**：这次请求按哪套 `ContextPolicy` / `WirePolicy` / replay / wire 规则组装；
2. **结果**：provider 回报了什么 usage，尤其是 Prompt Cache 读数。

两者都必须落在已有 `fastrace` 过程树的同一个 `llm.request` span 上，并让本地
`provider-trace.jsonl` 能以低频事件窄读；不引入第二套 tracing 或检视台。

已确认的代码事实：

- `ResponsesAssembler` 是 `/v1/responses` body 的唯一业务布局入口；它消费
  `WirePolicy`，而 agent 侧的 `ContextPolicy` 负责提供布局 hooks。
- `XyGenerateOptions.obs_parent` → bridge `AiBridgeGenerateOptions.obs_parent` 是
  `llm.request` 挂到 `agent.iteration` / `agent.compaction` 的既有 seam。
- `ProviderRequestTrace` 已在请求前捕获 body、在完成时挂 usage，并受
  `provider_trace_active` 与 `ObservationIoTier` 控制。
- `FileTraceReporter` 当前只把事件字段投影到 JSONL；span-level 属性可进入 OTLP，
  但不会自动出现在本地文件。因此不能只给 span 加属性后宣称本地
  `provider-trace` 可用。
- `c1925` 已钉死 Responses reasoning 的唯一回放策略为全量回放；不做
  Strip/BestEffort 旋钮。当前 `WirePolicy` 允许为后续 `previous_response_id`
  链式能力预留位，但允许位不等于本次请求实际使用链式。

## 2. 方案总览

```text
ContextPolicy（turn snapshot）
        │ 只传 bridge-neutral 的字符串标签，不传 agent 类型
        ▼
XyGenerateOptions → AiBridgeGenerateOptions
        │
WirePolicy + options + session projection
        ▼
ResponsesAssembler
        │ body + layout decision（不含 body 内容）
        ▼
ProviderRequestTrace: llm.request
        ├─ span properties: xylitol.layout.*
        ├─ one event: kind=layout, event=assembler.layout
        ├─ existing observation I/O (still gated)
        └─ one event: kind=usage (completion only, numeric/provenance only)
             │
             ├─ existing OTLP/Langfuse reporter
             └─ FileTraceReporter → provider-trace.jsonl
```

布局决策应在 Assembler 的输入和最终 wire policy 已知后形成，再交给 trace helper；
禁止从已经序列化的 body 反向猜测策略。实现可以让 Assembler 扩展现有 diagnostics
返回值，或使用等价的 typed result，但必须保持「body 与 decision 同源」。

agent 到 bridge 的新元数据应是小型、bridge-neutral 的 DTO（或等价的 protocol
port DTO），字段使用稳定字符串标签；不得让 bridge 依赖 `src/agent` 的
`ContextPolicy`。它在 turn 边界从 immutable policy snapshot 生成，避免一轮中途
策略变化导致 trace 标签与请求 body 不一致。

## 3. 已钉决策

| 项 | 决定 | 约束 / 理由 |
|---|---|---|
| 观测位置 | `llm.request` span 属性 + 一个 `assembler.layout` 事件 | OTLP 与本地 JSONL 同源；事件只打一条，避免每个 SSE chunk 重复布局字段 |
| 属性命名 | `xylitol.layout.schema` 与 `xylitol.layout.<field>` | 避免与 `api`、`compat` 等通用 OTLP 属性冲突；`schema=v1` 允许未来做加法演进 |
| `api` | adapter 的真实协议名，当前 Responses 为 `openai-responses` | 不从 URL 或 model 名启发式推断；既有 span `api` 属性继续保留 |
| `compat` | `WirePolicy.compat.as_str()` | 记录实际 wire 方言；不得把所有 Responses 兼容端当 OpenAI 第一语言 |
| `tools_mode` | turn snapshot 的稳定标签（如 `full` / `search`） | 记录策略档，不记录 tools schema、名称或内容 |
| `status_bar_mode` | turn snapshot 的稳定标签（`off` / `replace` / `append`） | 即使当前默认是 `off` 也记录；缺少 agent hint 的 bridge 直调用例省略，不伪造默认 |
| `date_placement` | turn snapshot 的稳定标签（如 `omit` / `system_as_today` / `system_pinned_at_session`） | 记录放置策略，不记录实际日期、cwd 或 system prompt |
| thinking 维度 | `xylitol.layout.thinking_level` 原样记录 + `replay_mode=full` | 档名是已有 freeform 配置；`full` 是 c1925 的固定回放策略，不是新配置 |
| `wire_mode` | 当前实际请求为 `full_replay`；只有实际使用链式时才可为 `chained` | `allows_previous_response_id=true` 不能单独触发 `chained`；链式实现留给 c1915 |
| `allow_midturn_tools_rewrite` | 本 change 不单独暴露 | 它是运行时改写闸，不是本次 Assembler layout 标签；若后续需审计另开行为 change |
| 缺失字段 | 省略字段，不写 `unknown` 或推测值 | bridge 直调用、旧 fake 或未来 adapter 可能没有 agent policy hint；诚实优先 |
| cache 对照 | 保留既有 span usage 属性，并增加低频 numeric/provenance `usage` 事件供 JSONL 窄读 | `NotReported` / `NotApplicable` 绝不写 `cache_read=0`；只有 `Tokens(n)` 才写数值 |
| body 指纹 | 本 change 不做 | canonicalization、敏感性、碰撞与跨版本解释尚未钉死；需要时另开 change |
| 闸与失败 | 复用 `provider_trace_active`；观测失败不得阻断请求 | 关闸不分配 layout event 的 JSON/字符串；HTTP error/abort 仍保留已附加的 layout span |
| 配置面 | 不新增 YAML、env 或 OTLP exporter | 沿用 code-first defaults 与既有 `fastrace` + `FileTraceReporter` / OTLP fanout |
| 产品面 | 不做 TUI、Print、Langfuse dataset 或自研 Inspect UI | 只增强已有 provider trace / OTLP 观测面 |

### 3.1 Canonical layout event

观测闸开启时，Responses 的 `llm.request` 在 body 已由 Assembler 组装后追加一条：

```text
kind=layout
event=assembler.layout
xylitol.layout.schema=v1
xylitol.layout.api=openai-responses
xylitol.layout.compat=generic
xylitol.layout.tools_mode=full              # 有 agent hint 时
xylitol.layout.status_bar_mode=off          # 有 agent hint 时
xylitol.layout.date_placement=omit          # 有 agent hint 时
xylitol.layout.thinking_level=medium
xylitol.layout.replay_mode=full
xylitol.layout.wire_mode=full_replay
```

示例只展示字段形状，不承诺 `medium` 等具体用户配置。除 `api` 外，字段均使用
`xylitol.layout.*` 前缀；本地 JSONL 的基础 `api` 仍由现有 reporter 写入。

`usage` 事件只保留 `input`、`output`、`total`、cache provenance，以及在
`Tokens(n)` 时的 `cache_read=n`。不得把 `langfuse.observation.input/output`、
完整 `usage_details`、cache key、`previous_response_id` 或请求 body复制到该事件。

## 4. 分层与调用时序

1. Agent 在 turn binding 处冻结本轮 `ContextPolicy` 的可观测标签，放入现有
   `XyGenerateOptions` seam；不把 `ContextPolicy` 类型下沉到 bridge。
2. infra adapter 做一对一 DTO mapping，保持 `XyModel` / `AiBridgeLlmAdapter`
   的既有调用方向。
3. Responses adapter 继续经 `ResponsesAssembler` 构造 body。Assembler 依据自身
   `WirePolicy` 和 options 生成 layout decision；`compat` 必须来自这里，而非 agent
   重复维护。
4. `ProviderRequestTrace::start_with_parent` 仍使用 `obs_parent`。body 完成后，
   在 HTTP 调用前调用等价的 `emit_layout` / `attach_layout` helper，再按既有
   `ObservationIoTier` 处理 body。
5. `Done` 时继续走既有 usage 映射；同时写低频 `usage` event。流式和非流式
   Responses 路径必须使用同一 helper，不能一条路径漏打。
6. `FileTraceReporter` 只从 layout/usage event 的 allowlist 投影字段；普通
   `raw` / `mapped` chunk 不重复携带布局属性。新增字段属于 v1 的向后兼容加法，
   不改变既有行的必需字段。

## 5. 测试 seam 与证据

以下 seam 在 branch binding 后落 live specs / feature（若需可执行 GWT），当前
仅作为 pre-start 规划：

| seam | 必须证明 |
|---|---|
| `ResponsesAssembler` 的公共组装结果/diagnostics seam | 同一 body 与 layout decision 同源；Generic/Deepseek、不同 context hint、thinking level、full replay 标签稳定 |
| `ProviderRequestTrace` + `ObsGateScope` / `SpanCollectScope` | 闸开有 `llm.request` layout 属性与单条 layout event；闸关无 span/无布局分配；`obs_parent` 仍正确 |
| `ProviderRequestTrace` usage helper | Tokens(n) 写 cache 数值，NotReported/NotApplicable 不伪造 0；abort 不伪造 usage |
| `FileTraceReporter` 的 JSONL 投影 | layout/usage 字段可读且不重复到每个 chunk；schema 仍为 v1；reporter 写失败不传播到请求 |
| Responses adapter offline/mock HTTP seam | stream 与 non-stream 都在发请求前附加同一 layout；HTTP error 仍可关联 request/layout；不新增 body I/O |

BDD-on 时，产品约束进入 `infra-observability`、`infra-otel` 与
`package-ai-bridge` 的 live `spec.toon`；可执行 GWT 只进入对应 `.feature` 并通过
`@req:` 关联。无需为内部字段形状重复编写第二份 feature。

## 6. 不做与升级边界

- 不改 Responses body 的布局语义，不实现 `tool_search`、状态栏、日界、压缩、
  `previous_response_id` 链式或 thinking 本身。
- 不把 body 全文、system prompt、cwd、日期值、工具 schema、reasoning signature
  或 cache key 写入新增观测事件。
- 不把 `xylitol.layout.*` 做成用户可配置的观测开关；仅现有 trace/OTLP 闸控制
  是否记录。
- 若未来需要 body hash，必须另行决定 canonical body、是否加盐/去敏、hash 版本及
  与 `observation_io` 的关系，不能在本 change 以 debug 便利隐式加入。

## 7. Start readiness（pre-start）

| 检查项 | 状态 | 结论 |
|---|---|---|
| Change identity | 已确认 | `c1935-add-assembler-layout-observability` |
| 硬依赖 | 已确认 | `c1890-add-responses-context-policy-assembler` 已归档 |
| 研究与代码事实 | 已确认 | 已阅读 Responses layout research、Assembler、ContextPolicy、WirePolicy、provider trace / FileTraceReporter / obs seam |
| Open Questions | 已钉 | body hash 后置；属性 namespace、双通道落点、回放/链式语义已在本文固定 |
| 测试 seam | 已列出 | 组装同源、span 闸、usage 诚实、JSONL 投影、stream/non-stream |
| scope fence | 已确认 | 只改本 change 规划壳；不改 live specs、代码、产品文档或 exporter |
| Branch binding | **未执行** | 有意保持 pre-start；不得在本阶段执行 `start` / `attach` |
| Specs landing | **未执行** | 必须在未来绑定的 change 分支完成；不得现在写 live specs |
| Apply readiness | **未满足** | 完成 branch binding + specs landing + `readyToImplement=true` 后才能 apply |

因此，本 change 已具备从默认分支进入 Branch binding 的设计前置条件，但仍保持
“未分支、未落 specs、不可实现”的门状态。
