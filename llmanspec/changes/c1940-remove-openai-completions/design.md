# Design: c1940-remove-openai-completions

## 状态策略（官方对照）

| 模式 | 官方 | 本产品 |
|---|---|---|
| ① `store:true` + `previous_response_id` | 服务端串链；依赖可存储账号 | **不做**（跨厂商不可用；与本地 SSOT / resume 冲突） |
| ② `store:false` + 完整 Item 回放 | ZDR / 客户端持态 | **唯一 OpenAI 兼容路径**（已有 Assembler + `thinkingSignature`） |
| ③ Conversations API | 持久 conversation 对象 | 不做 |

默认 WirePolicy 保持：`previous_response_id=false`、`prompt_cache_key=false`、`prompt_cache_usage=true`。

## 装配选择（删除后）

```text
resolve(kind, api):
  anthropic-messages → AnthropicMessages
  openai-responses   → OpenAiResponses
  openai-completions →（不再是合法选型）→ 与未知 api 相同
  省略 / 未知（OpenAI kind）→ OpenAiResponses
  省略（Anthropic kind）→ AnthropicMessages
```

`config.api` 字符串 MAY 原样保留用户写入值（观测诚实）；**AdapterKind 选型**不得再产生 Completions 实例。

## 删除边界

| 删 | 留 |
|---|---|
| `openai.rs` / `openai_completions.rs` 及主仓 Completions 外壳 | `openai_responses.rs` / `assembler` / `openai_client`（Responses 共用 Client） |
| `AdapterKind::OpenAiCompletions` | `OpenAiResponses` + `AnthropicMessages` |
| `apply_thinking_openai_completions` | `apply_thinking_openai_responses` + Anthropic |
| Cargo feature `chat-completion` | `responses` + `byot` + `middleware` + `rustls` |
| 文档「Completions 遗留逃生」 | 多厂商 = Responses 方言 + Anthropic |

共享 `openai_client` 在删 Completions 后仍服务 Responses；确认无 Completions-only 类型泄漏。

## 测试缝（harness）

| 缝 | 覆盖 |
|---|---|
| `AdapterKind::from_config_str` / `resolve_adapter_kind` | `openai-completions` → 等价省略 → Responses；省略默认 Responses |
| bridge `build_adapter*` | 无 Completions 分支；factory 单测改写 |
| `package-ai-bridge` feature / toon | 去掉 Completions effort 场景；pab* 措辞去 Completions |
| `infra-provider` / `runtime-model-registry` / `agent-hooks` | 删 Completions 场景与 req 措辞 |
| `package-ai-bridge-accounting` | Completions-only 远程计数场景改为「非 Responses 路径」或删 |
| 文档 / example.yaml | 去掉 Completions 注释档 |

不新扩 BDD step；以改写/删除既有 `@req` 场景 + 单测为主。

## 风险

- 方言端若**仅**实现 Chat Completions：本波后不再有产品路径——接受；用户改用支持 Responses 的端点或 Anthropic。
- `meta.api` 仍显示 `openai-completions` 时可能误导排障——可接受（开发阶段）；后续可选规范化（out of scope）。
