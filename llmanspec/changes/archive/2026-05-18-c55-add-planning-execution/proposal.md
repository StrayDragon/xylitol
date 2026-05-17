---
depends_on: [c10-add-config, c25-add-agent-loop]
---

# c55-add-planning-execution

## Why

规划-执行分离允许用强模型做规划（Architect），快速模型做执行（Editor），节省 token 并提高质量。完全由 YAML 配置驱动（§5）。

## What Changes

1. 在 `src/agent/planner.rs` 实现 Planner/Executor/Validator 三角色
2. 模型路由：规划器和执行器绑定不同模型（从 ModelRegistry 查找）
3. 步骤编排：按计划顺序执行，每步可触发验证
4. fallback：自动切换到配置的备用模型（OpenAI↔Anthropic）
5. 系统提示词模板（architect / editor）

### 执行流程

```
用户任务 → Planner 分解为步骤（JSON 计划）
→ 逐步执行：
    Executor 执行步骤 → Validator 验证（编译/lint/测试）
    → 失败 → 重试 / 请求用户确认
    → 成功 → 下一步
→ 最终结果
```

### YAML 配置

```yaml
planning:
  model: "anthropic/claude-opus-4"
  system_prompt: "architect"
  max_steps: 10
  reasoning_depth: deep
execution:
  model: "openai/gpt-4o-mini"
  system_prompt: "editor"
  max_retries: 2
validator:
  model: null  # 仅静态检查
  commands: ["cargo check", "cargo clippy", "cargo test"]
```

## Capabilities

- `planning-execution`: 规划-执行分离 + 模型路由 + 步骤编排 + fallback（OpenAI↔Anthropic）

## Impact

- `src/agent/planner.rs` 从占位变为实际实现
- feature flag `agent-planning` 启用此模块
- 依赖 c25 agent loop 的事件系统和工具分派
