# c385-fix-responses-input-item-type — Tasks

> 修复 Responses API 续轮 input item 缺 type 字段。c375 让续轮真正发生后暴露：message item 缺 `type: "message"`，混入 function_call item 后被 OpenAI 拒绝（`Cannot determine type of 'item'`）。

## 0. 规划工件

- [x] proposal.md + design.md + specs/provider-adapter/spec.toon（add pa5）
- [x] `llman sdd validate c385 --strict` 通过

## 1. 修复（openai_responses.rs）

- [x] message item 加 `type: "message"`，content 改数组格式（input_text/output_text）
- [x] assistant 纯 tool_call 轮（无 text）不发空 message item

## 2. 测试（responses adapter 首个测试模块）

- [x] `all_items_have_type_field`
- [x] `tool_round_items_are_well_typed`（user→assistant tool_call→tool_result）
- [x] `assistant_tool_only_round_emits_function_call_not_empty_message`

## 3. 校验

- [x] `cargo test --lib openai_responses` 3 全过
- [x] `just test` 567 全过
- [x] fmt + clippy 干净
- [x] 同步 tasks + 归档
