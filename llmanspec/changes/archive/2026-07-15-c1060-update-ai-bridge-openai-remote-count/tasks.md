# Tasks — c1060-update-ai-bridge-openai-remote-count

- [x] 1. Delta + design + tasks；`llman sdd validate c1060-update-ai-bridge-openai-remote-count --no-interactive`
- [x] 2. 导出 Responses `messages → input` 转换；实现 `OpenAiResponsesRemoteCounter`
- [x] 3. registry：Responses / gpt* 标明可 RemoteCount；Completions-only 不强制
- [x] 4. wiremock：`/v1/responses/input_tokens` 成功返回 input_tokens；失败降级单测
- [x] 5. 文档（AGENTS / design）；`cargo test -p xylitol-ai-bridge` + 相关 lint
