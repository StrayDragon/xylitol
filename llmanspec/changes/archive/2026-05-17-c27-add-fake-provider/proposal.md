---
depends_on: [c25-add-agent-loop]
---

# c27-add-fake-provider

## Why

开发阶段无法依赖真实 LLM API 进行测试——需要 API key、网络连接、产生费用、且响应慢不稳定。
需要一个 fake LLM provider 作为离线替代，能在不联网的情况下模拟模型行为（文本回复 + 工具调用），
用于快速验证 agent loop、工具系统、各模式流程的正确性。

这是项目本地快速验证的核心基础设施，后续所有需要 LLM 交互的功能都应能用它做离线测试。

## What Changes

1. 基于 c25 的 `Provider` trait（`src/agent/provider/`）实现 `FakeProvider`
2. 实现场景编排：文本回复、工具调用、多轮对话
3. 支持文本回复、工具调用、多轮对话的场景编排
4. 延迟模拟与错误注入
5. 随着后续功能扩展（tools、agent loop、print mode）同步完善 FakeProvider 的场景覆盖能力

### FakeProvider 能力

| 能力 | 说明 |
|------|------|
| 文本响应 | 预设一段或多段文本，按顺序逐段返回 |
| 工具调用 | 模拟模型发起工具调用，可指定工具名、参数、并预设执行结果 |
| 多轮对话 | 编排完整的多步 scenario（模型说→工具调用→工具结果→模型继续说...） |
| 延迟模拟 | 可配置每条 chunk 的模拟延迟，测试流式效果 |
| 错误注入 | 模拟 API 错误、超时、非法响应等异常场景 |
| Scenario DSL | 通过 builder 模式或 YAML/JSON 配置场景 |

### 使用方式（示例）

```rust
let fake = FakeProvider::new(vec![
    ScenarioStep::text("Hello! How can I help?"),
    ScenarioStep::tool_call("read", json!({"path": "src/main.rs"})),
    ScenarioStep::tool_result("read", json!({"content": "fn main() {}"})),
    ScenarioStep::text("Done! I read the file."),
]);
let response = fake.generate(request).await?;
```

### Feature 门控

- 新增特性 `dev-fake-provider`，不在默认特性中
- 仅用于测试和 dev 场景，不进入生产 release

## Capabilities

- `fake-provider`: Provider trait + FakeProvider 实现 + 场景编排

## Impact

- 新增 `src/agent/provider/` 模块
- 新增 Cargo feature `dev-fake-provider`
- 依赖于 c25 提供的 Provider trait/类型定义，FakeProvider 基于此实现
- c30 print mode 可使用 FakeProvider 编写模式测试
