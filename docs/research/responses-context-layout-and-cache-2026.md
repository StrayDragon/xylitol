# OpenAI Responses 上下文布局与缓存权衡（2026-08）

> **范围**：个人 coding agent（xylitol）在 **OpenAI Responses API**（含 llama.cpp 等兼容端）下，如何组织发给 provider 的 body，以及 KV Cache / Prompt Cache、状态栏、MCP/`tool_search`、压缩的工程取舍。
> **一手来源**：本仓 `packages/xylitol-ai-bridge` / `src/agent` 现状；OpenAI Prompt caching / Responses / tool_search 文档；《深入理解 AI Agent》姊妹仓 `ai-agent-book/book/chapter2.md`（及 Ch4/Ch5）（状态栏、KV/Prompt Cache、工具只增不改、Cursor MCP 索引实践）。书语仅经下文 §7 术语表进入工程名，**禁止**写入 live specs。
>
> **Assembler 缝（c1890）**：Responses 请求 body 经 `ResponsesAssembler`（`xylitol-ai-bridge`）唯一构造（底层 assemble 为 crate-private）；agent 侧 `ContextPolicy` 提供 hooks；`set_tools*` 已消费 mid-turn rewrite 闸。状态栏完整行为 **deferred**（`llmanspec/delayed-changes/context/c1895…`）；Todo **deferred**（[`c1955`](../../llmanspec/delayed-changes/context/c1955-add-agent-todo-subsystem/proposal.md)）；本期主线 **`c1900`（MCP 首条门闸 + Full 工具定稿）**；`tool_search` → [`c1960`](../../llmanspec/delayed-changes/tools/c1960-add-tool-search-mcp-discovery/proposal.md)；`c1930`/`c1925` 软配合。**不**引入独立 context epoch 计数（全量重放以当前 SSOT 为准；链式断链见 `c1915` 非 input 快照比对）。Ornith lab：改 tools[] 必 cache miss → 宜首轮定稿后冻死。
> **非目标**：不定实现排期；不改 live specs；不把 hit rate 当唯一 KPI。

## 一句话结论

**主线 = 只把 `openai-responses` 做到可配置、可观测、可测的一类 API；Completions 保留为显式 YAML `api: openai-completions`；Anthropic Messages 留桩不进默认。同协议族用代码内 `compat`（首版 `generic`，`defaults.rs`）区分方言端保守策略，**不**本波扩 YAML/env。缓存是架构约束之一，不是目标函数。MCP 优选 `tool_search`（内部目录 + 搜索注入），避免「异步加载完就热改 `tools` 表」或「阻塞输入直到全加载」。**

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
| `from_responses_usage` 在默认关闸时强制 `cache_read=0`；开闸后可映射 `cached_tokens`（c1880 骨架；三态诚实见 c1885） | Completions 路径可读 `cached_tokens`；Responses 须按 WirePolicy 诚实区分未回报 |
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

> **Codex 对照（2026-08 读码）**：MCP 在 search 可用时为 `ToolExposure::Deferred`（不进顶栏 `tools[]`）；发现结果经轨迹里的 `tool_search_output`（`defer_loading: true`）加载；跨轮顶栏工具名表保持稳定。形似 Responses 的本地端（llama.cpp 等）**不保证**同语义——须 WirePolicy + 真机验证。
>
> **Ornith / llama.cpp lab（2026-08-05）**：网关可达；`type: tool_search` **静默剥离**；`function` 名 `tool_search` 可用；`defer_loading` 忽略；input `tool_search_*` **400**；普通 `function_call_output` 可用。compat 声明须 `hosted_tool_search=false`；方言 search 命中后 **append `tools[]`**（Q8）。forge 走 function 形 + 非 hosted item。
>
> **同名改 description × cache（重启后多臂）**：`input` 前缀 `[[session:UUID]]` 隔离。同 session 重复同 description → cache 升温；**只改 `tool_search` description → `cached_tokens` 掉回 0**（打断前缀缓存）。原始：`/tmp/tool-search-desc-cache-multi.jsonl`。


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

### 5.1 易忘坑：同「Responses」面，方言 ≠ 第一语言

**第一语言** = 厂商原生 API（OpenAI / Anthropic / Kimi 官方等）。**方言** = 他方实现该协议形状（DeepSeek / llama.cpp / 网关实现 `openai-responses`）。形似 ≠ 语义等价：cache 字段、`tool_search`/`defer_loading`、`previous_response_id`/`store`、reasoning/include、缺字段 SSE 等均可迥异。

工程要求（落地见 `c1880` / `c1890`；**以 c1880 定稿为准**）：

- 分层：YAML **`api`**（协议族全称，如 `openai-responses`）× 代码内 **`compat`**（兼容策略档；首版常量 `generic`）× 代码内 **`extra_policy`**（仅 API req/resp 布尔；非 agent 能力）
- **本波 code-first**：`compat` / `extra_policy` 在 `xylitol-ai-bridge` 的 **`defaults.rs` 纯常量**；**不**新增 YAML 旋钮、**不**用 env 当配置面；调试改 defaults 文件
- Assembler / adapter **保留适配层**：同一 `AiBridgeMessage` 投影，按 `api`×WirePolicy 选字段子集、usage 映射、降级
- **禁止**假设「凡 `openai-responses` 端点 ≡ OpenAI 第一语言语义」

旧稿用语：`flavor` → **`compat`**；配置面 `capabilities`（易混）→ wire 侧 **`extra_policy`**（`tool_search` 等 agent 策略不进此块，见 `c1900`）。

`previous_response_id`：公共能力之后的可选优化；默认全量重放 `input`；断链回退全量；仅当 `extra_policy.previous_response_id`（代码默认板）允许时才开链式（→ `c1915`）。

### 5.2 Lab：resume × reasoning 回放 × prompt cache（c1925 · 2026-08-06）

> 维护脚本：`cargo run -p xylitol-ai-bridge --example lab_resume_prompt_cache`（**不进 qa**）。配置：`<global-dir>/dev/live-provider.yaml`（Ornith / llama.cpp）。试验命名统一 `lab_`（见根 `AGENTS.md`「试验 / 打网命名」）；qa 串行闸二进制为 `lab_responses_prompt_cache`。

**产品策略（钉死）**：回放 **只有默认全量**——有合法 `thinkingSignature` 则原样进 `input`；**不**做 Strip/BestEffort 旋钮（改前缀易破 cache）。

流程：热身多轮（普通对话 + 只读 tool / skill 提示）→ 序列化历史 → **新 adapter 实例**（模拟进程退出）→ Preserve 续跑；记 `usage.cached_tokens`。

| 条件 | warm1 | warm3 | resume Preserve |
|---|---|---|---|
| `thinking=medium` | 0 | 727 | **761** |
| `thinking=off`（`XYLITOL_LAB_THINKING=off`） | 0 | 710 | **747** |

早期对照臂曾测「Strip 历史 reasoning」resume → cache_read **404**（相对 Preserve 761 腰斩）——仅作否决三态的证据，**不**产品化。原始目录：`/tmp/xylitol-lab-resume-cache-*`。

**敏感**：thinking 开时 `include: reasoning.encrypted_content`，`thinkingSignature` 可为 **含 `encrypted_content` 的整包 reasoning JSON** 并写入 session JSONL。本地磁盘视为敏感材料（勿贴公共 issue / 日志）；本波不为改 `store:true` 而剥落盘。

隔天 resume 另有 system **date** 日界前缀漂移风险（→ `c1905`）；与 reasoning 回放正交。

**验证闸（c1925 apply）**：同 lab 重跑后，**主闸** `resume_full ≥ warm3`（同次 run）；§5.2 表上绝对数（medium ≥761 / off ≥747）为历史地板，网关抖动时先对照同次 warm3，勿以 Strip 臂当对照。

### 5.3 Lab：session 前缀幂等 × resume/import（c1930 · 2026-08-06）

> 维护脚本：`cargo run -p xylitol-ai-bridge --example lab_session_prefix_idempotency`（**不进 qa**）。规划/证据：[`landing.tmp.md`](../../llmanspec/changes/c1930-update-session-provider-view-contract/landing.tmp.md) §5。

**主钉**：resume/import 后 Responses `input`（+tools）前缀与同进程续跑在固定旋钮下规范化相等；**本波不做**状态栏。

| 闸 | 结果（Ornith 复跑） |
|---|---|
| offline serde/JSONL 哈希 | 相等 |
| arm A 内存续跑 vs arm B 新 adapter+JSONL | `input[0..-1]` 相等；B `cache_read` ≥ A |
| Langfuse | example 不 OTEL 导出；dump ≡ `observation.input` 同形；全量 xylitol 可对照 |

固定旋钮：`Current date: 2026-08-06`、固定 cwd/tools/`thinking=medium`。date 日界产品化 → `c1905`。

离线单测：`llm_project::resume_import_shaped_jsonl_matches_memory_assemble_prefix`；`ResponsesAssembler::assemble_prefix_idempotent_and_serde_roundtrip`。

复验（2026-08-06 apply，Ornith 同网关）：medium warm3=694 resume=728（Δ+34，对齐基线 Δ）；off warm3=762 resume=792（Δ+30，绝对高于历史 747）。

---

## 6. 压缩

- 只压轨迹中段；稳定前缀不动
- 大工具结果替换串**首次冻结**（避免重启字节漂移）
- 禁止滑动窗口砍头部
- 策略档挂可观测 + 日后 Eval；省 token 不自动等于更好

---

## 7. 与后续 draft change 的映射

> 开发顺序以各提案 `depends_on` 为准，用 `llman sdd graph` 查看；**不**在本文维护波次表。
> 下表用**工程提案标题/方向**对齐 draft（非把 live specs 翻译成书语）；书中概念对照见下表后术语表。

| 方向（工程） | change（草案） | `depends_on` 摘要 |
|---|---|---|
| Responses 默认 + Completions 显式 `api` + Anthropic 桩 + code-first WirePolicy | `c1880`（已归档） | `[]` |
| Responses cache usage 诚实透出 | `c1885`（已归档） | `[]` |
| ContextPolicy + ResponsesAssembler | `c1890`（已归档） | `c1880` |
| Thinking/reasoning 回放保真（JSONL→input） | [`c1925`](../../llmanspec/changes/c1925-update-responses-thinking-replay-flavor/proposal.md) | `c1880`+`c1890` |
| Session SSOT ↔ Provider view · **本波** | [`c1930`](../../llmanspec/changes/c1930-update-session-provider-view-contract/proposal.md)（§5.3 lab） | `c1890` |
| Assembler 布局决策可观测（**deferred**） | [`delayed c1935`](../../llmanspec/delayed-changes/context/c1935-add-assembler-layout-observability/proposal.md) | `c1890` |
| Agent Todo（**deferred**；扩展后置） | [`delayed c1955`](../../llmanspec/delayed-changes/context/c1955-add-agent-todo-subsystem/proposal.md) | — |
| Agent 状态栏族（**deferred**） | [`delayed c1895`](../../llmanspec/delayed-changes/context/c1895-add-agent-status-bar-subsystem/proposal.md)（+ c1896/97/98） | 升格待 Todo/事件 |
| MCP 首条门闸 + 工具定稿 · **本期主线** | [`c1900`](../../llmanspec/changes/c1900-update-mcp-first-turn-tool-freeze/proposal.md) | `c1880`+`c1890` |
| tool_search + Deferred · **双轨 B（活跃草案，后实现）** | [`c1960`](../../llmanspec/delayed-changes/tools/c1960-add-tool-search-mcp-discovery/proposal.md) | `c1900` |
| tools 稳定 id / resume MCP（**调研**） | `research`（含 2026-08-10：MCP-only vs 删内建的 `input`/`tools[]` bust 面） | — |
| system 稳定/可变切分（**deferred**） | [`delayed c1905`](../../llmanspec/delayed-changes/context/c1905-update-system-prompt-stable-volatile-split/proposal.md) | `c1890` |
| 压缩冻结替换串（**deferred**） | [`delayed c1910`](../../llmanspec/delayed-changes/context/c1910-update-compaction-freeze-tool-replacements/proposal.md) | `c1890`+`c1930` |
| previous_response_id 可选链 | [`c1915`](../../llmanspec/changes/c1915-add-previous-response-id-optional-chain/proposal.md)（active；断链=非 input 快照比对，无 epoch） | `c1880`+`c1890` |

依赖以各 `proposal.md` frontmatter `depends_on` 为准；本文不钉实现细节。**草稿不改 live specs**；propose 前才 specs landing。

### 术语对照（书中 / 白话 ↔ 工程）

| 书中 / 白话 | 工程（draft / 代码意向） |
|---|---|
| 静态前缀 / 轨迹 | ContextPolicy 切点；Assembler 输入布局 |
| 系统提示词 / 工具定义 | system·`instructions` / Responses `tools` |
| 第一语言 / 方言 | 厂商原生 API vs 他方兼容实现（`c1880`） |
| 实现口味 (flavor)（旧稿） | → 代码内 **`compat`**（`defaults.rs`；首版 `generic`） |
| 能力声明 / capabilities（旧稿，易混） | wire → **`extra_policy`**（仅 req/resp）；agent 能力另案 |
| Prompt Cache / KV Cache | usage `cached_tokens`；本地推理侧另论 |
| 静态前缀可比性 / 断链 | 非 input 请求快照比对（`c1915`；对照 Codex）；全量重放无独立 epoch |
| 会话真源 / 发给模型的投影 | Session SSOT ↔ provider view（`c1930`） |
| 框架元信息 | harness meta |
| Agent 状态栏 · replace / append | StatusBar 模式（**deferred** `c1895`） |
| 主动工具发现 / 只增不改 | tool_search + append-only（`c1900`） |
| 思考回放（全量） | reasoning full replay（`c1925`） |
| 增量续跑 | `previous_response_id`（`c1915`） |
| 本轮组装决策可观测 | layout observability（`c1935`） |

---

## 8. 明确不做（本调研结论）

- 命中率最大化作为产品目标
- 无配置的隐式每轮状态栏 append
- 用同一套断点 API 假装 OpenAI ≡ Anthropic 缓存
- **假设凡声明 Responses 兼容的端点 ≡ OpenAI 第一语言语义**（须 `api` × 代码 `compat`/`extra_policy`；见 `c1880`；本波不扩 YAML/env）
- 本波把未暴露策略做成 YAML 或散落 env 影子配置（默认板 = `defaults.rs`）
- 首版自动探测网关能力- 以「阻塞用户至 MCP 全加载」为主路径（尤其 resume）
- LLM 维护状态栏统计
- 把 live specs「翻译」成书中话术来代替工程草案（书语只在术语对照 / research 叙事层）

## 相关

- 产品候补：[../roadmaps/上下文缓存与极致压缩.md](../roadmaps/上下文缓存与极致压缩.md)
- 已落地压缩：[../architecture/压缩与上下文.md](../architecture/压缩与上下文.md)
- 多厂商：[../architecture/多厂商模型.md](../architecture/多厂商模型.md)
- MCP：[../architecture/扩展能力-MCP.md](../architecture/扩展能力-MCP.md)
- OpenAI：[Prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching) · [tool search](https://developers.openai.com/api/docs/guides/tools-tool-search)
