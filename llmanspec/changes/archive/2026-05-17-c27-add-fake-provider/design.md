# Fake Provider 设计

## Provider Trait 定位

### 问题
- c25 将引入 `adk-model`，其中包含 provider 抽象，或在其 adapter 中定义自有 `Provider` trait
- c27 的 FakeProvider 需要实现 c25 定义的 trait

### 决策：实现 c25 定义的 Provider trait

FakeProvider 直接实现 c25 提供的 `Provider` trait，不自行定义新接口：
1. c25 定义 trait + request/response 类型
2. c27 实现 `FakeProvider: Provider`，用于离线测试
3. 保持 provider 接口的唯一来源（Single Source of Truth）

```
c25:  Provider trait + GenerateRequest/Response + ProviderError
      ↳ adk-model 的 thin wrapper（适配层）
c27:  FakeProvider: Provider
      ↳ 基于 ScenarioStep 的场景编排
```

## FakeProvider 状态模型

```
Scenario: [Step1, Step2, ..., StepN]
  ↑ cursor (原子递增)
  ↓
每次 generate() 消耗一步，返回对应的 Response

Step 类型（内部定义，非 trait 的一部分）：
- Text(String)          → 返回文本 delta
- ToolCall(name, args)  → 返回工具调用请求
- ToolResult(name, result) → 返回工具执行结果
- Error(error)          → 返回错误
- Delay(Duration)       → 睡眠指定时间后继续下一步
```

### 多轮 vs 单次调用

FakeProvider 支持两种模式：
1. **场景模式**（默认）：每次 `generate()` 消耗一步，游标逐步推进。适合编排完整的多轮交互。
2. **循环模式**：配置 N 条文本响应，每次调用按顺序返回，超出则从第一条重新开始。适合简单的压力测试。

### 错误注入

通过 `ScenarioStep::Error(ProviderError)` 模拟：
- `ProviderError::Api(message)` — API 错误
- `ProviderError::RateLimited(retry_after)` — 限流
- `ProviderError::InvalidResponse(message)` — 异常响应

## 测试策略

1. **单元测试**: 直接测试 FakeProvider 各场景路径
2. **集成测试**: 将 FakeProvider 接入 agent loop，验证完整流程
3. **二进制验证**: `cargo run --features dev-fake-provider -- --mode print`（c30 后）

## Open Questions

- Q: 是否需要 YAML/JSON 序列化的 scenario 配置，以支持不重新编译的测试？
  - A: 先做 builder API，后续若需要再扩展 YAML 加载。
