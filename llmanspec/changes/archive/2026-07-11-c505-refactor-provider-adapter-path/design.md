# design — c505 Provider 单路径（简图）

## 装配图（落地后）

```text
XyModelConfig
    → factory::build_provider
    → adapter::factory::build_adapter  (by AdapterKind / api string)
         ├─ openai-responses  → OpenAiResponsesAdapter : LlmAdapter
         ├─ openai-completions → OpenAiCompletionsAdapter : LlmAdapter
         │                         └─ private OpenAIProvider (HTTP only, no XyModel)
         └─ anthropic-messages → AnthropicMessagesAdapter : LlmAdapter
    → AdapterXyModel(adapter) : XyModel
    → Arc<dyn XyModel> 注入 agent

Fake / Mock：直接 impl XyModel（测试双，不经 adapter）
```

禁止：

```text
OpenAIProvider : XyModel
    → 再被 OpenAiCompletionsAdapter 包一层
    → 再被 AdapterXyModel 包一层
```

Completions 的 HTTP 客户端类型可以保留为 adapter 的私有实现细节，但 MUST NOT 再对外 `impl XyModel`。
