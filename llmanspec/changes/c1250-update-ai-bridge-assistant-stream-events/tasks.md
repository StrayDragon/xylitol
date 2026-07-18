# Tasks: c1250-update-ai-bridge-assistant-stream-events

status: purpose-draft — promote（live specs + attach）后再 apply。

## Promote 前

- [ ] 在 feature 分支编辑 live `package-ai-bridge` / `infra-provider` 约束与必要 `.feature` 或包内单测契约
- [ ] `llman sdd change attach c1250-update-ai-bridge-assistant-stream-events`
- [ ] `status: full`；本文件勾选改为实施清单

## 实施

- [ ] 定义 `AiBridgeAssistantEvent`（或最终命名）与 partial 消息结构；删除/迁移整包-only `FunctionCall` 热路径
- [ ] 实现 `parse_streaming_json` + 单测（残缺 JSON、转义、空对象）
- [ ] Responses adapter：added/delta/done → toolcall_*；保留 thinking/text 映射
- [ ] Completions adapter：增量 `tool_calls` → toolcall_*（对齐 pi openai-completions）
- [ ] Anthropic adapter：tool_use + input_json_delta 对齐
- [ ] `map.rs` + domain 类型；更新 fake model / 测试夹具
- [ ] 录制或手写「t0718 时序」级 fixture：断言首 delta 即有意图事件
- [ ] `cargo test -p xylitol-ai-bridge`；相关主仓 map 测试
- [ ] `llman sdd validate c1250-update-ai-bridge-assistant-stream-events --strict --no-interactive`
