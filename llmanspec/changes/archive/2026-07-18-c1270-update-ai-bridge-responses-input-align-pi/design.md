# Design: c1270 Responses input align pi

## Decision

Align xylitol Responses (and system channel for Completions/Anthropic) with pi's `convertMessages` contract:

| Concern | Before | After (pi-like) |
|---|---|---|
| System | First empty-session `AgentMessage::user(system)` | `options.system_prompt` every request; adapter role inject |
| Responses role | n/a | `developer` if `thinking_level != off`, else `system` |
| Thinking replay | Merged into `output_text` via `as_text()` | Text-only for message; signature → reasoning item; else omit |

## Shape notes

- System/developer item: pi uses bare `{role, content: string}`. Xylitol keeps typed `message` items for user/assistant/tool mix (c375). Prefer pi bare shape for system/developer first; if a server rejects, fall back to typed message with `input_text` (document in test comment).
- `thinkingSignature`: opaque JSON string of a Responses `reasoning` item (pi). Parse fail → omit that block (do not panic, do not merge into text).

## Non-goals

- Capturing encrypted_content on stream into signature (follow-up if needed).
- Changing UI chrome.
