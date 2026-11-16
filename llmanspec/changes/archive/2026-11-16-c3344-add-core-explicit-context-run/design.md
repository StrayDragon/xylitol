# Design — c3344 核心显式上下文运行

## 目标

- 让调用方以第一类请求对象传入本轮 user parts、前序上下文与 system 覆盖。
- 请求上下文作为本轮种子历史；session 只记录真实 turn。
- 不破坏现有 `run` / `run_with_context` / `submit_parts` 与所有 `XyDriver` 实现。

## 非目标

- 不实现 surface 的 HTTP/SSE 端点；由 thoughtdag `add-explicit-context-run` 承担。
- 不在本 change 引入 model/effort/request_id；留给后续 change 扩展 `TurnRequest`。
- 不把请求上下文持久化为 session 条目；重放语义另行设计。

## 核心设计

### TurnRequest

`TurnRequest` 是 `XyDriver` 应用协议的完整请求对象：

```rust
#[non_exhaustive]
pub struct TurnRequest {
    pub parts: Vec<AgentPart>,
    pub context: Vec<XyLlmMessage>,
    pub system_prompt: Option<String>,
}
```

- `parts` 是本轮 user 内容，含图片。
- `context` 是前序 user/assistant/toolResult；为空时从 session 加载历史。
- `system_prompt` 是本轮覆盖；`None` 用 agent 配置。
- `#[non_exhaustive]` + 构造器允许后续加字段而不破坏外部调用方。

### Driver 默认行为

`XyDriver::run_turn` 默认只对纯 prompt 回退 `run(prompt_text)`；带 `context` 或 `system_prompt` 时返回 `XyDriverError::unsupported`。这样现有 `ScriptedDriver`、`OccupancyDriver`、`StubDriver` 等实现无需改动，但不会静默丢弃请求上下文。

调用方必须处理 `Result`；surface 在不支持时返回 501，而不是假装上下文生效。

### AgentRuntime 种子历史

- `submit_turn` 与 `submit_parts` 共用准入、turn 创建、`freeze_root_config`。
- `start_root_stream` 接受 `Option<Vec<AgentMessage>>`；system 覆盖经 `freeze_root_config` 参数传入，不改 `FrozenRootConfig` 字段。
- 有 seed 时跳过 `load_conversation_history`；无 seed 保持现状。
- 本轮 user/assistant 仍 `persist_agent_message`；seed 不写。
- `freeze_root_config` 用覆盖值替代 `inner.system_prompt()`，最终经 `XyGenerateOptions.system_prompt` 传给模型，不写成 user。

### InProcessDriver 转换

- `context: Vec<XyLlmMessage>` 直接包成 `AgentMessage::Llm`，不新增投影类型。
- 保留 capability overlay、tool table 冻结、session 绑定。
- 空 context 时行为与 `run` 等价。

## 备选方案

| 方案 | 问题 | 结论 |
|---|---|---|
| `run_with_messages` | 方法增殖；system/parts 无处放；后续 model/effort 还要加方法 | 拒绝 |
| `run_turn` 必填、无默认 | 编译期强制支持；但破坏外部 driver 与测试 stub | 拒绝 |
| 写 session 条目 / 临时 session | 污染 session；fork/replay 复杂；违背一画布不强制一 session | 拒绝 |
| 复用 `McpResourceContext` | MCP 资源语义，不是对话消息 | 拒绝 |
| context-inject loop 中间件 | 按请求动态注册；线程/生命周期复杂 | 拒绝 |
| `TurnRequest` 第一类对象 | 新增公共类型，但一次到位、可扩展 | 采用 |

## 兼容与影响面

- `XyDriver` 新增默认方法：现有实现不受影响。
- `run` / `run_with_context` / `submit_parts` 不变。
- 不涉及线协议、TUI、provider 方言。
- 公共类型从 `xylitol::embed` 再导出，thoughtdag 可直接构造。

## 错误与边界

- 空 messages / 无 user 由 surface 校验；核心不静默回退。
- seed 含未知 role 由 surface 拒绝；核心不做额外校验。
- seed 不写 session；session 重放看不到完整上下文，需文档标注。
- system 覆盖是替换而非追加；追加可作为后续可选参数。

## 测试策略

- 纯类型单测：`TurnRequest::text` / `prompt_text` / 构造器。
- Driver 默认回退单测：自定义 stub driver 验证 `run_turn` 调 `run`。
- AgentRuntime 单测：seed 不落盘、本轮 user/assistant 落盘、system 不写 user。
- InProcess 单测：空 context 等价 `run`；非空 context 传给 `submit_turn`。
- BDD：`agent-run-context` 四个场景绑定。
- 回归：`run`、`run_with_context`、`submit_parts` 既有测试。

## 迁移与发布

- 无破坏性合约变更；新增 API 为可选。
- thoughtdag 在核心归档后升级依赖并接 `run_turn`。
- 不需要迁移脚本。

## 未决问题

- `TurnRequest` 未来字段命名（model/effort/request_id）在后续 change 定。
- 是否提供 system 追加而非覆盖，后续按需求加。
