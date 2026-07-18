# Design: c1290 Responses reasoning replay + body fields + guidelines

## Decision

Align with pi `packages/ai/src/api/openai-responses.ts` + `openai-responses-shared.ts` and coding-agent tool `promptGuidelines` assembly. **No** XML salvage; ReAct stop-without-native-tools unchanged.

| Concern | Before (post-c1270) | After (pi-like) |
|---|---|---|
| Body `store` | omitted | `false` |
| Tools `strict` | omitted | `false` per tool |
| `reasoning` | `{effort}` only | + `summary: "auto"` (or options override) when thinking ≠ off |
| `include` | omitted | `["reasoning.encrypted_content"]` when thinking ≠ off |
| Stream reasoning | `ThinkingDelta` text only | On reasoning `output_item.done`（或非流式 output item）：整 item JSON → signature |
| History Thinking | signature field unused in practice | ReAct persists signature on `AgentPart::Thinking` |
| Replay | c1270：有 signature 则推 reasoning | unchanged contract; now fed by capture |
| Guidelines | `SystemPromptOpts.prompt_guidelines` 空；tools 默认 `[]` | `set_tools` 收集；内置工具补短句 |

## Capture → persist shape

pi sets `block.thinkingSignature = JSON.stringify(item)` on `response.output_item.done` when `item.type === "reasoning"`, then emits `thinking_end`.

Xylitol today only has `ThinkingDelta` / accumulates a `String` in ReAct with `AgentPart::thinking(...)`（signature=None）.

**Preferred：**

1. Add `AiBridgeChunk::ThinkingEnd { thinking: String, thinking_signature: Option<String> }`（及 `XyChunk` 镜像）。
2. Responses adapter：reasoning `output_item.done` → 更新 thinking 文本（summary∥content）+ `ThinkingEnd`（signature = item 的 JSON 串）；delta 仍走 `ThinkingDelta`。
3. 非流式 `parse_responses_output`：对每个 reasoning item 发出等价 End（或直接带 signature 的终态）。
4. ReAct：收到 `ThinkingEnd` 时写入 `thinking_acc` + `thinking_signature_acc`；最终 `AgentPart::Thinking { thinking, thinking_signature, .. }`。
5. Mid-stream `MessageUpdate` 可不带 signature（仅终态落盘必须有）。

若发现可在不扩 chunk 的前提下把 signature 挂在现有路径上且不破坏流式 UX，允许等价设计，但 MUST 仍保证落盘 Thinking 带 signature。

## Compat / degradation

- Ornith/tufa：证据显示接受 store/strict/summary/include，且 SSE 可回 `encrypted_content`。
- 若某兼容端 4xx：经既有 provider/compat 开关省略 `include` 和/或 `store`；记文档；默认路径仍对齐 pi。
- Parse 失败的 signature：省略该 reasoning item（c1270），不 panic、不并入 `output_text`。

## Guidelines

- `session::set_tools`（及 bootstrap 等同路径）对每个 tool 调用 `prompt_guidelines()`，扁平并入 `prompt_opts.prompt_guidelines`。
- `build_system_prompt` 已有 Guidelines 段；custom/SYSTEM.md 替换默认正文时 **不**强制再塞 Available tools（跟 pi customPrompt）。
- 文案对齐 pi 工具短句（bash/read/edit/write/…），不是「禁止 XML」防呆。

## Non-goals

- UI chrome（c1280）；fastrace（c1265）；把 encrypted blob 渲染到 TUI。
