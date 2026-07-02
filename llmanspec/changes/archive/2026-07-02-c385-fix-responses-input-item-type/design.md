# c385 Design — Responses API input item type 修复

## 问题

`convert_messages_to_input_items`（openai_responses.rs）构造的 message item 缺 `type` 字段：
```json
{"role": "user", "content": "hello"}   // 缺 "type": "message"
```

Responses API 要求每个 input item 显式 `type`。第一轮纯 user message 因 OpenAI 兼容解析通过；续轮混入 function_call/function_call_output 后严格校验，缺 type 的 message 被拒（`Cannot determine type of 'item'`）。

## 修复

所有 message 类 item 改为规范格式：
```json
{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "..."}]}
{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "..."}]}
```
- user/bash/compaction/custom → input_text
- assistant → output_text

assistant 纯 tool_call 轮（text 为空）：不发空 message item，只发 function_call item（避免空消息）。

function_call / function_call_output 原本已有 `type`，不变。

## 为何 c375 后才暴露

c375 前 ReAct loop 在 tool call 后 `if done { break }`，续轮从不发生，这段续轮转换从未被驱动。c375 让续轮真正发生后立即暴露此序列化 bug。

## 测试（responses adapter 首个测试模块）

- `all_items_have_type_field`：任意对话转换后每个 item 有 type。
- `tool_round_items_are_well_typed`：user→assistant(tool_call)→tool_result 序列产生 [message, message, function_call, function_call_output]。
- `assistant_tool_only_round_emits_function_call_not_empty_message`：纯 tool_call 无 text 只产 function_call。

## 不在范围

- Chat Completions adapter（openai_completions.rs / openai.rs）不受影响（不同 API 格式，且 OpenAi 默认走 Responses）。
