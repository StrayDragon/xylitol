# OpenAI Responses 上下文布局与缓存权衡（2026-08）

> **范围**：个人 coding agent（xylitol）在 **OpenAI Responses API**（含 llama.cpp 等兼容端）下，如何组织发给 provider 的 body，以及 KV Cache / Prompt Cache、状态栏、MCP/`tool_search`、压缩的工程取舍。
> **一手来源**：本仓 `packages/xylitol-ai-bridge` / `src/agent` 现状；OpenAI Prompt caching / Responses / tool_search 文档；《深入理解 AI Agent》Ch2/Ch4/Ch5（状态栏两实现、工具只增不改、Cursor MCP 索引实践）。
> **非目标**：不定实现排期；不改 live specs；不把 hit rate 当唯一 KPI。

## 一句话结论

**主线 = 只把 Responses 做到可配置、可观测、可测的「一类 API」；Completions 保留为显式 `api` 类型扩展点；Anthropic Messages 留桩不进默认。同协议族用 `flavor` 区分官方 / DeepSeek / llama.cpp 等实现差异并允许覆盖。缓存是架构约束之一，不是目标函数——注意力干净与策略可切换优先于盲目抬命中率。MCP 优选 `tool_search`（内部目录 + 搜索注入），避免「异步加载完就热改 `tools` 表」或「阻塞输入直到全加载」两条更重的路。**

---

## 1. 两层缓存（勿混）

| 层 | 作用域 | 失效条件（直觉） |
|---|---|---|
| **KV Cache** | 单次推理内部 | 前缀任意字节改动 → 从改点起重算 |
| **Prompt Cache** | 跨请求（服务商） | 前缀不匹配 / TTL；OpenAI 多为**自动前缀**；Anthropic 常需显式 `cache_control`（举例，落地时按文档复核） |

编码 agent 场景：用户多在**同一项目目录**会话，`cwd` 写进 system **往往可接受**（目录很少中途变）。**日历日级 `date` 则有已知坑**：单日内稳定，但**隔天 resume 同一 session** 时若仍用「开会话那天」的 date → 模型时间感过时；若每日改写 system 内 date → 整段稳定前缀字节变，Prompt Cache / 前缀匹配从该点失效。秒级时间戳更糟，禁止进 system。

可选方向（**留给 `c1905` / `c1895` 深挖，此处不定案**）：date 留 system 但定义「日界刷新」策略；或 date 走状态栏（replace/append）；或仅在跨日首轮追加一条 meta、system 不再含 date。与「不盲目追命中」一致：选哪条看 resume 频率与注意力，不看 hit rate。

真正伤前缀的常见项还包括：工具表中途变长、工具重排、滑动窗口砍头部消息。

**权衡**：堆叠重复状态（例如每轮 append 一条大状态栏）可能抬命中，但稀释注意力。策略必须**可配置、可消融、可对照 usage**，而不是默认「能 cache 就 cache」。

---

## 2. xylitol 现状（代码事实）

| 事实 | 含义 |
|---|---|
| Responses body 硬编码 `"store": false` | 未用 `previous_response_id` 链式续跑；靠全量 `input` + 自动前缀缓存 |
| `from_responses_usage` 的 `cache_read` 恒 0 | Completions 路径可读 `cached_tokens`；Responses **看不见**命中 |
| `AssistantMessage.response_id` 字段有，ReAct 落盘常为 `None` | 链式续跑无锚点 |
| `build_system_prompt` 末尾含 date + cwd | 单项目场景通常稳定；秒级时间会伤缓存 |
| MCP settle / reload 热合并进 `ToolSet` → 每轮 `tools` 全表 | **改请求顶栏工具定义** = 动稳定前缀；与「会话早期已缓存前缀」冲突 |
| 应用层 `AiBridgeMessage` + 方言 adapter | 统一视图合理；风险在用同一套断点语义硬压平 OpenAI / Anthropic |

---

## 3. 状态栏（Agent Status Bar）

书 Ch2：状态栏 = 末尾 meta（常借用 `user` role）注入的**代码维护**读数；非银弹。

| 更新模式 | 缓存 | 注意力 / token | 适用 |
|---|---|---|---|
| **replace**（每轮删旧插新） | 旧条之后 cache 失效（范围通常小于改 system） | 上下文干净、无歧义 | 短轨迹 / 单条状态很大 |
| **append**（只追加不删） | 前缀友好 | 陈旧条堆积，要求模型认「最新一条」 | 长轨迹、更新频繁 |
| **off** | — | — | 默认候选 |

硬约束（书 + 本仓产品定调）：

- 用**代码**维护读数，禁止 LLM 批量扫历史写栏
- 读数宜键值对；复杂节奏决策需要「读数 + 短操作策略」成对
- 模型几乎无条件信任栏内容 → 防投毒；可关

coding agent 默认：环境类（cwd）可留 system；高变读数（工具计数、TODO、git dirty）才进栏，且**默认可 off**。

---

## 4. 工具 / MCP 披露

### 4.1 问题

全量 MCP schema 进 `tools`：贵、伤选择、中途武装改前缀。
「等全部 MCP ready 再允许输入」：首条前缀稳，但 TUI 要锁输入；**resume** 时历史里可能已有中途加载的工具痕迹，需遍历差分再对齐——复杂。

### 4.2 推荐：`tool_search` 元工具（主路径）

```text
异步 MCP 连接照旧（不挡 TTI）
        │
        ▼
内部 ToolRegistry（核心 + 已发现 MCP）← 热合并只发生在这里
        │
        ▼
发给 Responses 的 tools 表 = 稳定子集
  （核心工具 + tool_search [+ 可选 namespace/defer_loading]）
        │
        ▼
模型调用 tool_search → 旁路/同 model 短请求或 client 检索
        │
        ▼
结果以「只增不改」进入主轨迹（固定首次位置），不每轮搬到末尾
```

- **启用 tool_search 时**：不把异步加载结果自动追加进 provider `tools` 数组。
- **发现请求**：默认当前用户 model **另开请求**；可配置 sidecar model，避免脏主轨迹。
- **兼容端无 hosted tool_search**：client-executed 元工具 + 约定形状的 output 项（capability 配置声明，首版不自动发现）。
- **不启用 tool_search**：不把「阻塞至全加载」当主方案；可降级为「仅核心工具」或显式全量档（接受前缀重算），文档写清。

书/业界对照：Cursor 索引式 MCP（约 −47% 相关 token）；Codex 默认 `tool_search`；OpenAI `defer_loading` + `tool_search` / client `tool_search_output`。

---

## 5. Provider 边界建议

| API | 角色 |
|---|---|
| **OpenAI Responses** | 默认主交付；布局 / cache / tool_search 优化只在此极致做 |
| **OpenAI Completions** | **保留**为显式 `api: openai-completions`（或等价），避免系统设计与 Responses 形状绑死；不为其扭曲 Responses Assembler |
| **Anthropic Messages** | 桩 + 注释；需要时再开；**禁止**为 Anthropic 断点语义反向设计主 Assembler |

### 5.1 易忘坑：同「Responses」面，不同 provider flavor

声明实现 OpenAI Responses 的端点（官方 OpenAI、DeepSeek、llama.cpp、各类网关）**协议形似 ≠ 语义等价**：cache 字段是否回报、`tool_search`/`defer_loading` 是否真支持、`previous_response_id`/`store`、reasoning/include、缺字段 SSE 等均可迥异。

工程要求（落地见 `c1880` / `c1890`）：

- 配置分层：`api`（协议族，如 `openai-responses`）× **`flavor`**（实现口味，如 `openai-official` / `deepseek` / `llamacpp` / `generic`，名称以实现为准）
- 用户可 **flavor 覆盖**默认策略（及 capabilities），不靠自动探测
- Assembler / adapter **保留适配层**：同一 `AiBridgeMessage` 投影，按 flavor 选字段子集、usage 映射、降级路径
- **禁止**假设「凡 Responses 端点行为同 OpenAI 官方」

模型档案 **capabilities**（首版手写配置，不做自动探测）：如 `prompt_cache_usage`、`tool_search`、`defer_loading`、`previous_response_id`、`prompt_cache_key`；可由 flavor 预设，再被用户覆盖。

`previous_response_id`：公共能力之后的可选优化；默认全量重放 `input`；断链（compact / 换模 / fork / 工具世代变更）回退全量；仅当 flavor/capabilities 声明支持时才允许配置开启。

---

## 6. 压缩

- 只压轨迹中段；稳定前缀不动
- 大工具结果替换串**首次冻结**（避免重启字节漂移）
- 禁止滑动窗口砍头部
- 策略档挂可观测 + 日后 Eval；省 token 不自动等于更好

---

## 7. 与后续 draft change 的映射

| 方向 | change（草案） | 备注 |
|---|---|---|
| Responses 默认 + Completions 显式类型 + Anthropic 桩 + capabilities 配置 | [`c1880-update-responses-first-api-boundary`](../../llmanspec/changes/c1880-update-responses-first-api-boundary/proposal.md) | Wave A |
| Responses cache usage 诚实透出 | [`c1885-add-responses-cache-usage-honesty`](../../llmanspec/changes/c1885-add-responses-cache-usage-honesty/proposal.md) | Wave A 可并行 |
| ContextPolicy + ResponsesAssembler | [`c1890-add-responses-context-policy-assembler`](../../llmanspec/changes/c1890-add-responses-context-policy-assembler/proposal.md) | Wave B，依赖 c1880 |
| Status bar 子系统 | [`c1895-add-agent-status-bar-subsystem`](../../llmanspec/changes/c1895-add-agent-status-bar-subsystem/proposal.md) | Wave C，依赖 c1890 |
| tool_search + MCP 内部目录 | [`c1900-add-tool-search-mcp-discovery`](../../llmanspec/changes/c1900-add-tool-search-mcp-discovery/proposal.md) | Wave C，依赖 c1880+c1890 |
| system 稳定/可变切分 | [`c1905-update-system-prompt-stable-volatile-split`](../../llmanspec/changes/c1905-update-system-prompt-stable-volatile-split/proposal.md) | Wave C，依赖 c1890 |
| 压缩冻结替换串 | [`c1910-update-compaction-freeze-tool-replacements`](../../llmanspec/changes/c1910-update-compaction-freeze-tool-replacements/proposal.md) | Wave C，依赖 c1890 |
| previous_response_id 可选链 | [`c1915-add-previous-response-id-optional-chain`](../../llmanspec/changes/c1915-add-previous-response-id-optional-chain/proposal.md) | Wave D，依赖 c1880+c1890 |

依赖以各 `proposal.md` frontmatter `depends_on` 为准；本文不钉实现细节。

---

## 8. 明确不做（本调研结论）

- 命中率最大化作为产品目标
- 无配置的隐式每轮状态栏 append
- 用同一套断点 API 假装 OpenAI ≡ Anthropic 缓存
- **假设凡声明 Responses 兼容的端点 ≡ OpenAI 官方语义**（须 `api` × `flavor` + 可覆盖 capabilities）
- 首版自动探测网关能力
- 以「阻塞用户至 MCP 全加载」为主路径（尤其 resume）
- LLM 维护状态栏统计

## 相关

- 产品候补：[../roadmaps/上下文缓存与极致压缩.md](../roadmaps/上下文缓存与极致压缩.md)
- 已落地压缩：[../architecture/压缩与上下文.md](../architecture/压缩与上下文.md)
- 多厂商：[../architecture/多厂商模型.md](../architecture/多厂商模型.md)
- MCP：[../architecture/扩展能力-MCP.md](../architecture/扩展能力-MCP.md)
- OpenAI：[Prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching) · [tool search](https://developers.openai.com/api/docs/guides/tools-tool-search)
