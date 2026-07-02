---
change_id: c385-fix-responses-input-item-type
title: 修复 Responses API 续轮请求 input item 缺 type 字段被 OpenAI 拒绝
status: proposed
priority: 385
depends_on:
  - c375-fix-react-loop-tool-continuation
author: agent
---

# c385-fix-responses-input-item-type

## Why

c375 修复了 ReAct loop 不续轮的 bug。续轮一旦真正发生（user → assistant tool_call → tool result → 续轮调模型），OpenAI Responses API 返回 `Cannot determine type of 'item'` 拒绝续轮请求。

### 根因（代码事实）

OpenAI 默认走 Responses API（`/v1/responses`，adapter/mod.rs:43 `OpenAi => OpenAiResponses`）。其 `convert_messages_to_input_items`（openai_responses.rs）构造的 **message 类 item 缺 `type` 字段**：

```json
// 旧（错）：
{"role": "user", "content": "hello"}
{"role": "assistant", "content": "let me check"}
```

Responses API 要求**每个 input item 显式声明 `type`**：
- 消息：`{"type": "message", "role": "...", "content": [{"type": "input_text"|"output_text", "text": "..."}]}`
- 函数调用：`{"type": "function_call", ...}`
- 函数输出：`{"type": "function_call_output", ...}`

第一轮（纯 user message）能通过，是因为 OpenAI 对顶层 input 做了兼容解析。但续轮时 items 数组混入 `function_call` / `function_call_output`，严格校验启用，缺 `type` 的 message item 即被拒。

### 为何长期未发现

- c375 之前 ReAct loop 在 tool call 后提前 break，**续轮从不发生**，这段转换代码从没被 tool-round 序列真正驱动过。
- `convert_messages_to_input_items` **无任何测试覆盖**（responses adapter 整个文件没有测试模块）。

## What Changes

1. **修复 `convert_messages_to_input_items`**：所有 message 类 item（user/assistant/bash/compaction/custom）加 `type: "message"`，content 改为规范数组格式（`[{"type": "input_text"|"output_text", "text": ...}]`）。
2. **assistant 纯 tool_call 轮**：text 为空时不发空 message item（只发 function_call item），避免空消息混淆 API。
3. **新增测试模块**（responses adapter 首个）：覆盖 all_items_have_type_field / tool_round_items_are_well_typed / assistant 纯 tool_call 轮。

## Capabilities

- `provider-adapter`（修改）：Responses adapter 的 input item 序列化 MUST 符合 OpenAI Responses API 规范（每个 item 带 `type`）。

## Impact

- **受影响代码**：`src/infra/provider/adapter/openai_responses.rs`（convert 函数 + 新增测试）。
- **风险**：低。改的是序列化格式，向规范靠拢；既有单轮对话行为不变（OpenAI 对规范格式同样接受）。

## 反降级护栏

- [ ] 每个 input item MUST 带 `type`（message / function_call / function_call_output）。
- [ ] message item 的 content MUST 是数组格式（input_text / output_text），MUST NOT 是裸字符串。
- [ ] assistant 纯 tool_call 轮（无 text）MUST NOT 发空 message item。
- [ ] tool round（user→assistant tool_call→tool result）续轮请求 MUST 不被 OpenAI 拒绝。
