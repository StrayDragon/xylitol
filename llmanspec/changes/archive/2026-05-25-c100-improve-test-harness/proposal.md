---
depends_on: []
---

# c100-improve-test-harness

## Why

来自 c95-fix-test-stability 归档中的 deferred 项。当前测试缺乏统一的 timeout wrapper，async 测试可能无限等待。MockToolContext 缺少 `workspace_root` 字段，限制了工具测试的完整性。

## What Changes

1. **创建 `with_test_timeout` helper**：统一的测试超时 wrapper
2. **为 async 测试包裹 timeout**：防止 CI 中无限挂起
3. **MockToolContext 添加 `workspace_root`**：使工具测试可以验证路径相关逻辑

## Impact

- 仅影响测试代码，不影响生产行为
- 提高 CI 稳定性
