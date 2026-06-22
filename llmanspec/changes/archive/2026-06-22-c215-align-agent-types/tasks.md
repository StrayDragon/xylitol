# c215-align-agent-types — Tasks

## Phase 1 — AgentMessage 字段补齐

- [x] **T1.1** — 给 `AssistantMessage` 添加 `api`, `provider`, `model`, `response_id`, `error_message`, `timestamp`, `diagnostics` 字段
- [x] **T1.2** — 给 `ToolResultMessage` 添加 `tool_name`, `details`, `timestamp` 字段
- [x] **T1.3** — 给 `UserMessage` 添加 `timestamp` 字段
- [x] **T1.4** — 给 `Usage` 添加 `cache_write_1h` 和 `cost: UsageCost` 字段
- [x] **T1.5** — 定义 `UsageCost` 结构体（`input`, `output`, `cache_read`, `cache_write`, `total: f64`）
- [x] **T1.6** — 定义 `Diagnostic` 结构体
- [x] **T1.7** — 给 `AgentPart::Thinking` 添加 `redacted: bool`, `signature: Option<String>` 字段
- [x] **T1.8** — 定义 `AgentState` 结构体
- [x] **T1.9** — 定义 `AgentContext` 结构体
- [x] **T1.10** — 更新 `AgentMessage` 的便利构造函数和辅助方法
- [x] **T1.11** — 更新 serde 序列化/反序列化 `#[serde(...)]` 属性
- [x] **验证**: `cargo build`

## Phase 2 — 内部历史迁移 Vec<XyContent> → Vec<AgentMessage>

- [x] **T2.1** — 迁移 `loop.rs` 的 `history: Vec<XyContent>` → `Vec<AgentMessage>`
  - 更新 `run_dual_loop` 的内部状态管理
  - 更新 `execute_tools_parallel` / `execute_tools_sequential` 的工具结果构建
  - 更新 `AgentEvent::AgentEnd { messages }` 的消息类型
- [x] **T2.2** — 迁移 `session.rs` 的 `drain_queued_messages` 消息类型
- [x] **T2.3** — 迁移 `compaction.rs` 的对话序列化（`serialize_conversation` 等）
- [x] **T2.4** — 更新 provider 转换函数（已接收 `AgentMessage`，无需改变，但更新内部分配）
- [x] **验证**: `cargo build`

## Phase 3 — 事件系统补齐

- [x] **T3.1** — 在 `lifecycle.rs` 中添加 `MessageStart` 和 `MessageUpdate` 变体
- [x] **T3.2** — 在 `loop.rs` 的内层循环中发射 `MessageStart` 和 `MessageUpdate`
- [x] **T3.3** — 更新 `AgentEvent::MessageEnd` 携带完整 `AgentMessage`
- [x] **验证**: `cargo build`

## Phase 4 — ModelMeta 补齐

- [x] **T4.1** — 给 `ModelMeta` 添加 `api`, `provider`, `cost_input`, `cost_output`, `cost_cache_read`, `cost_cache_write`, `max_tokens`, `thinking_levels` 字段
- [x] **T4.2** — 更新 `model_manifest.rs` 的 JSON 加载器解析新字段
- [x] **T4.3** — 更新测试（`test_load_hunyuan`, `test_load_claude` 等）
- [x] **验证**: `cargo build`

## Phase 5 — 删除旧类型

- [x] **T5.1** — 删除 `types.rs` 中 `XyContent`/`XyPart`/`XyRole` 定义
- [x] **T5.2** — 删除 `from_xy_content`/`into_xy_content`/`from_xy_part`/`into_xy_part` 转换函数
- [x] **T5.3** — 删除 `XyFinishReason` 类型，全局替换为 `StopReason`
- [x] **T5.4** — 删除 `XyChunk` 中已废弃的旧引用
- [x] **T5.5** — 清理所有 import 和 re-export
- [x] **验证**: `cargo build && cargo test --lib`

## Phase 6 — 最终验证

- [x] **T6.1** — `cargo build`（零错误）
- [x] **T6.2** — `cargo test --lib`（所有测试通过）
- [x] **T6.3** — `cargo clippy --all-targets -- -D warnings`（零警告）
- [x] **T6.4** — 验证 `XyContent`/`XyPart`/`XyRole` 在 `src/` 中零引用
