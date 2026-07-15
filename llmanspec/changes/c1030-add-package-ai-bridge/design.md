# Design — c1030 packages/xylitol-ai-bridge

## 1. 包定位

| 是 | 不是 |
|---|---|
| xylitol → **上游 LLM model provider** 的 client 接线 | 通用「AI SDK」品牌层（避免与 pi-ai 叙事混淆） |
| 方言 HTTP/SSE → 包内流事件 + usage 归一化 | ReAct / session tree / TUI |
| 多源 token accounting（带 provenance） | 单独的 tokenshub 终态包 |
| workspace 库；**零依赖**主 crate `xylitol` | 反向依赖主 crate |

包名 **`xylitol-ai-bridge`**：强调 bridge（接线），不是 agent 本体。

```text
app / agent
    │  只认 XyModel / XyChunk / XyUsage / ContextTokenEstimate
    ▼
infra（薄映射 + 组合根装配）
    │  DTO ↔ domain
    ▼
packages/xylitol-ai-bridge
    provider | usage | accounting | tokenize | registry | fake
```

## 2. 双类型过渡（本 change 选定）

**决策**：包内自有 DTO（如 `BridgeMessage` / `BridgeChunk` / `BridgeUsage`）；主 crate `infra` 负责 ↔ `AgentMessage` / `XyChunk` / `XyUsage`。

| 优点 | 代价 |
|---|---|
| 包可独立编译/测试；不强迫先拆 domain crate | 映射层样板；字段漂移风险 |
| 满足「禁止 providers 包依赖主 xylitol」 | 需单测锁住映射不变量 |

**后续**：独立 `xylitol-llm-types` 见 purpose-draft **c1040**（`depends_on: c1030`）。若映射成本在实施中爆表，按 proposal ethics 升级并优先 promote c1040。

**硬约束**：`agent/` 与 `domain/` MUST NOT import `xylitol_ai_bridge` 的 vendor/HTTP 类型；仅经 `XyModel` / accounting 门面（经 infra 或 runtime_protocol 端口注入）。

## 3. 模块划分

```text
packages/xylitol-ai-bridge/
  src/
    lib.rs
    provider/          # LlmAdapter 等价：OpenAI Responses/Completions、Anthropic Messages、装配
    usage/             # 方言 usage 字段 → BridgeUsage；可选 cost(rates)
    accounting/        # 优先级解析 → ContextTokenEstimate + TokenProvenance
    tokenize/          # Builtin(tiktoken/claude-tokenizer) + HF tokenizer.json 缓存
    registry/          # model_id → TokenizerSource / RemoteCount 能力
    fake/              # 离线 Fake（自测与主仓 BDD 共用策略）
```

主仓：`src/infra/provider` 逐步变为 **map + re-export 装配**；实现体迁入包（tasks 分批，允许中间态双路径但归档前必须单一路径）。

## 4. Token 计量：来源与优先级

### 4.1 TokenProvenance

```text
Api            厂商响应/流末 usage（本回合或适用的历史锚点）
RemoteCount    厂商 count API（如 Anthropic POST /v1/messages/count_tokens）
LocalTokenizer 本地：tiktoken / claude-tokenizer / tokenizer.json
Heuristic      chars/4（及既有 image 常数等）
Unknown        无可用源且策略拒绝展示假数（产品可选；默认链路落到 Heuristic）
```

### 4.2 上下文占用（footer 预留 / compaction）

```text
1. 可信 Api 锚点（适用于当前 leaf/prefix 的最近 assistant usage；
   abort/error、compact 后失效规则见下）
     + 锚点之后消息用下一档估 trailing，合成 total
2. 否则 RemoteCount（registry 声明支持且调用成功）
3. 否则 LocalTokenizer（registry 命中且已加载/可加载）
4. 否则 Heuristic
```

**锚点失效（MUST）**

- `stop_reason` 为 abort/error 的 usage MUST NOT 作锚点
- compaction 摘要插入后，压缩前 usage MUST NOT 描述新 prefix（对齐 pi：timestamp / prefix 边界）
- Fake/无 usage 消息不构成锚点

### 4.3 本回合刚结束的 usage（写入 session）

```text
1. 流末 / 非流 ApiUsage → 记为 Api
2. 否则不得伪造 Api；可选 LocalTokenizer 估 output 并标 LocalTokenizer
3. Heuristic MUST NOT 写入「厂商 usage」语义字段而不改 provenance
```

### 4.4 流式与写放大

| 允许 | 禁止 |
|---|---|
| 在厂商事件点更新 usage（Anthropic `message_start` / `message_delta`；OpenAI `include_usage` 末包；Responses `response.usage`） | 每个 `TextDelta` 对全文 `tokenizer.encode` |
| 活 UI：缓冲文本 + **节流** Heuristic（可选） | 假设 BPE 可按字符可靠增量合并 |
| `Done` 时一次性 Local 估（仅当 Api 缺失） | 默认静默联网下载 10MB+ tokenizer.json |

性能预期（Local，非合约硬指标，作实现指引）：单条消息 encode 通常 ≪ 1ms；加载 tokenizer.json 一次性百 ms 级，须缓存。

### 4.5 RemoteCount

- Anthropic：`/v1/messages/count_tokens`（含 tools/system 时优于裸本地 tokenizer）
- OpenAI：Responses input token count（若端点可用）作可选
- MUST 可配置关闭；失败降级下一档；MUST NOT 阻塞首 token 流（异步/显式调用路径）

### 4.6 LocalTokenizer 注册表

```text
Builtin { OpenAiO200k | OpenAiCl100k | AnthropicClaude | ... }
HuggingFace { repo, file, mirrors[] }   # 缓存 ~/.xylitol/tokenizers/；下载 opt-in
```

用户覆盖：`registry.user.toml` 优先级高于内置。未命中 → Heuristic + warn 日志。

## 5. 映射到 xylitol 内部语义

```text
BridgeChunk::Delta/Done(BridgeUsage?)
        │ infra map
        ▼
XyChunk::TextDelta|ThinkingDelta|FunctionCall|Done { usage: Option<XyUsage> }

accounting::estimate_context(...)
        │ infra 或 agent 端口
        ▼
ContextTokenEstimate {
  tokens, provenance: TokenProvenance,
  usage_tokens, trailing_tokens, ...
}
```

`XyUsage` 继续为 canonical 用量结构（domain-compaction c15）；cost 字段由 `usage` 模块按 model rates 可选填充。

**Driver 预留（本 change 合约，非 footer UI）**

- 只读 API 形状：返回 `ContextTokenEstimate`（或等价 seam DTO）
- 绑定当前 leaf/path 消息集
- 产品 footer 文案 / harness → purpose-draft **c1035**

## 6. 与现有合约关系

| Spec | 关系 |
|---|---|
| `infra-provider` | 实现迁入 bridge；主树保留 LlmAdapter/XyModel 装配语义与 pa7 vendor 隔离 |
| `domain-compaction` | `estimate_context_tokens` 等改走 accounting 优先级；仍用 `XyUsage` |
| `runtime-model-registry` | 不合并；tokenizer 映射是 bridge `registry`，可后续交叉引用 model id |
| c1035（draft） | 本 change 提供可信 Estimate 源；footer 产品面由 c1035 承接（原 c655） |

## 7. 实施分期（与 tasks 对齐）

1. 立包 + 空模块 + workspace 成员 + 边界测试（零依赖主 crate）
2. DTO + usage 归一化 + 映射层骨架
3. 迁 Fake + 一个 adapter，打通 map → XyModel
4. 迁齐 OpenAI/Anthropic 路径；删主树重复实现
5. accounting 优先级 + builtin tokenize
6. HF tokenizer.json 缓存（opt-in）+ RemoteCount 接线（至少 Anthropic 或可测 stub）
7. compaction / stats 切换；Driver Estimate 预留 seam
8. 文档 / arch_guard / validate

## 8. 风险

| 风险 | 缓解 |
|---|---|
| 双类型漂移 | 映射单测 + 禁止 agent 直接用 Bridge*；恶化则 promote **c1040** |
| 大迁回归 | tasks 分批；Fake/BDD 先绿再迁下一 adapter |
| HF 下载 | opt-in；镜像可配；失败降级；显式 CLI 见 **c1050** |
| 「Exact」误解 | 对外用 Provenance，文档写明 Local ≠ 账单保证 |

## 9. 下游 draft 依赖（无 future.md）

后续能力不进本 change 的 `future.md`，一律为独立 purpose-draft，并用 `depends_on` 表达：

| Change | 依赖 |
|---|---|
| c1035 footer token | c1030 |
| c1055 footer cost | c1035 |
| c1040 llm-types | c1030 |
| c1045 GGUF tokenize | c1030 |
| c1050 CLI tokenize | c1030 |
| c1060 OpenAI RemoteCount | c1030 |

实施顺序受 DAG 约束：未归档依赖不可 apply。
