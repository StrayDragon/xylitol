# Tasks: c1250-update-ai-bridge-assistant-stream-events

## Promote

- [x] live `package-ai-bridge` 约束 pab13/pab14 + scenarios（feature:true；BDD step 在 tests/bdd.rs）
- [x] `llman sdd change attach c1250-update-ai-bridge-assistant-stream-events`
- [x] proposal `status: full`

## 实施

- [x] 定义 `AiBridgeChunk` ToolCallStart/Delta/End；移除整包-only `FunctionCall`
- [x] 实现 `parse_streaming_json` + 单测
- [x] Responses adapter：added/delta/done → toolcall_*；fixture `function_call_args_stream_before_done`
- [x] Completions adapter：增量 `tool_calls` → toolcall_*
- [x] Anthropic adapter：tool_use + input_json_delta 对齐
- [x] `map.rs` + domain `XyChunk`；react 将 End 视作原 FunctionCall，Start/Delta 暂忽略（c1255）
- [x] fake / 测试夹具更新
- [x] `cargo test -p xylitol-ai-bridge`；相关主仓 react/xy_chunk 测试
- [x] `llman sdd validate c1250-update-ai-bridge-assistant-stream-events --strict --no-interactive`
