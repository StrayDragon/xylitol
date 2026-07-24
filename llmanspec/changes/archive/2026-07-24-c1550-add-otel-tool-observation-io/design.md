# Design: c1550-add-otel-tool-observation-io

## Decisions

| 点 | 选择 | 理由 |
|---|---|---|
| 配置键 | `tool_observation_io`，复用 `OtelObservationIo` 枚举 | 与 generation 档位同形，敏感度可独立开 |
| 闸位置 | bridge `AtomicU8`（平行 `OBSERVATION_IO_TIER`） | agent ↛ infra；与 `observation_io` 同装配模式 |
| 写入时机 | `ToolExecuteSpan::attach_io` 在结果落定后、span drop 前 | 覆盖成功 / deny / error 路径 |
| 截断 | truncated=4096、full=65536 Unicode scalars | 对齐 `PROVIDER_TRACE_TEXT_MAX` / `OBSERVATION_IO_FULL_MAX` |
| input 形状 | `serde_json::to_string` 工具参数（失败则 `"{}"`） | 稳定可搜；不做二次 schema 美化 |

## Non-goals

- 把 tool I/O 合并进 `observation_io`
- 给 Session 列表写 turn 根 I/O（c1555）
