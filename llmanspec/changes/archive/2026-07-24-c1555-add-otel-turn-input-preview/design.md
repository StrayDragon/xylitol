# Design: c1555-add-otel-turn-input-preview

## Decisions

| 点 | 选择 | 理由 |
|---|---|---|
| 闸 | **复用** `[otel].observation_io` | Session 预览与 generation 同敏；不新开第三档配置 |
| 属性 | 根 span `langfuse.observation.input` | Langfuse 映射为 trace input → Sessions 列表可见 |
| 文本 | 用户 `AgentPart` 文本拼接（image → `[image]`） | 与 UI preview 一致；无 base64 |
| output | 本切片不写 turn 根 output | 避免与多轮 assistant / tool 结果抢预览位；详情看 generation |
| API | `AgentTurnSpan::start(user_preview: Option<&str>)` | 在 push user message 后、建 span 时传入 |

## Non-goals

- 独立 `turn_observation_io` 配置
- tool I/O（c1550）
