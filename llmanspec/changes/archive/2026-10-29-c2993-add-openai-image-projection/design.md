# Design

## Context

`AgentPart::Image(ImageContent { url, data, media_type })` 是会话真源里的图片分片。Read 工具把它放进 tool result parts；Anthropic 方言已映射为 `{"type":"image","source":{"type":"base64",...}}`。OpenAI 两族只取文本，图片静默消失。

## Decision

### Responses

- user message：content 数组逐 part 映射 `input_text` / `input_image`。
- tool result：`function_call_output.output` 在无图时保持字符串（兼容既有行为）；有图时改为 `[input_text, input_image...]`。无文本时用 `[image]` 占位。
- 依据：Responses API 的 `function_call_output.output` 明确支持 `string | Array<ResponseInputText | ResponseInputImage | ResponseInputFile>`。

### Completions

- user message：有图时改用 content 数组（`text` + `image_url`），无图时保持字符串，避免无谓形状变化。
- tool message：Chat Completions 的 `role=tool` content 只接受文本分片，因此图片不能塞进 tool message。
- tool result 图片：按工具结果顺序收集，在同一轮工具结果之后批量补发**一条** user message，文本标签带来源 call id（`[tool result images: call-1, call-2]`），再跟 `image_url` 分片。遇到下一条 user / assistant 消息前 flush；循环结束再 flush 一次。
- 依据：OpenAI Chat Completions 的 tool content 仅支持 text；业界通行做法是把工具图片搬到紧邻的 user message，避免 interleaved user 消息破坏工具轮。

### 共用

- `image_data_url` 放 `provider/native/mod.rs`，两族共用，避免 data URL 规则漂移。
- `url` 存在时优先用 `url`；否则用 `data:<media_type>;base64,<data>`。
- 仅图片无文本：Completions tool message 写 `[image]`；Responses output 写 `[image]`。

## Alternatives

- **A1. 把 base64 塞进 tool 文本**：模型不会当图片读，且浪费 token；拒绝。
- **A2. 每个 tool result 后立刻补一条 user 消息**：连续工具结果会产生多条 user 消息，部分兼容端拒绝 interleaved user；改为批一条。
- **A3. 只在 Responses 支持**：Completions 仍静默丢图，不解决缺口。

## Risks

- Completions 的 trailing user 消息会让消息条数增加；只在有工具图片时发生。
- 部分 OpenAI 兼容网关可能不支持 `image_url`；这是上游能力差异，不在本 change 伪造降级。
- data URL 体积受 `infra-image` 既有 4.5MB 上限约束，不新增限制。
