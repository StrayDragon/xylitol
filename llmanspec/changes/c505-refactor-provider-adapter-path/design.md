# design — c505 Provider 单路径（简图）

```text
XyModelConfig
    → factory::build_provider
    → 选择 LlmAdapter（Completions | Responses | Anthropic | …）
    → AdapterXyModel(adapter) : XyModel
    → Arc<dyn XyModel> 注入 agent
```

禁止：

```text
OpenAIProvider : XyModel
    → 再被 OpenAiCompletionsAdapter 包一层
    → 再被 AdapterXyModel 包一层
```

Completions 的 HTTP 客户端类型可以保留为 adapter 的私有实现细节，但 MUST NOT 再对外 `impl XyModel`。
