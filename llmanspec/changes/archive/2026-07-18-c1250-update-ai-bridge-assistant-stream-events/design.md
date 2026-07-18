# Design: c1250 ai-bridge assistant stream events

## 目标形状（对齐 pi，不抄 TS）

pi SSOT：`AssistantMessageEvent` =

- `start` / `done` / `error`
- `text_start|delta|end`
- `thinking_start|delta|end`
- `toolcall_start|delta|end`（delta 携带 args JSON 片段；partial 上 `arguments` 为 `parseStreamingJson` 结果）

xylitol 落地原则：

1. **语义同构**，Rust 命名可 `Xy`/`AiBridge` 前缀。
2. **开闭**：新兼容端只写 adapter，不改 ReAct/TUI。
3. **一次改完**：不保留「整包 FunctionCall + 新事件」双路径长期 shim（若迁移需极短过渡，tasks 内限期删除）。

## 推荐类型草图

```text
AiBridgeAssistantEvent
  Start { partial }
  TextStart { content_index, partial }
  TextDelta { content_index, delta, partial }
  TextEnd { content_index, content, partial }
  ThinkingStart / ThinkingDelta / ThinkingEnd
  ToolCallStart { content_index, partial }   // name/id 可能仍空，随后 delta 补齐
  ToolCallDelta { content_index, delta, partial }  // delta = args JSON 片段
  ToolCallEnd { content_index, tool_call, partial }
  Done { reason, message }
  Error { reason, error }
```

`partial`：累积中的助手消息（`content: [Text | Thinking | ToolCall]`），与落盘 `AgentPart` 可投影。

兼容策略（实施时二选一，prefer A）：

- **A**：`stream()` 改为产出 `AiBridgeAssistantEvent`；旧 `AiBridgeChunk` 仅测试/假模型用或删除。
- **B**：内部事件流，对外仍提供 `Stream<AiBridgeChunk>` 由 `ToolCallEnd` 折叠——**拒绝**：会再次丢掉流式意图。

主仓 `map.rs`：事件 → domain 等价类型（扩展 `XyChunk` 或新 `XyAssistantEvent`）。

## Adapter 映射表

| Provider | start | delta | end |
|---|---|---|---|
| Completions | 首个 `delta.tool_calls[i]`（按 index/id） | `function.arguments` 片段 | `finish_reason` 或流结束 finalize |
| Responses | `output_item.added` type=function_call | `function_call_arguments.delta` | `output_item.done` |
| Anthropic | `content_block_start` tool_use | `input_json_delta` | `content_block_stop` |

`parse_streaming_json`：可依赖成熟 partial-json 类库，或移植 pi `repairJson`+partial-parse 逻辑；MUST 对非法中间态返回尽最大努力 object，不得 panic。

## 与 t0718 的对照验收

对 Responses 假流重放「53× args delta → 3× item.done」：

- 第 1 个 args delta 之后 MUST 已存在 `ToolCallStart`（或 name 已知的 Start）+ 至少一次 `ToolCallDelta`；
- MUST NOT 仅在三个 `item.done` 才首次出现工具相关 mapped 事件。

## 非目标

- 文本通道伪工具抽取
- agent/TUI 行为（仅保证下游能订阅增量事件）
