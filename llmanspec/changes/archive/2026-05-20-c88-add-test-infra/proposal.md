---
depends_on: [c05-init-skeleton, c25-add-agent-loop]
---

# c88-add-test-infra

## Why

PRD §13 用 ~400 行详述测试基础设施：FauxProvider（mock LLM）、TestHarness（全连线测试）、VT100Backend（TUI 渲染验证）、wiremock SSE（agent loop 集成测试）、PTY E2E、CI 管道。这些是项目质量保障的骨架，缺少这部分意味着测试策略规格缺失。

## What Changes

1. 在 `tests/support/` 建立共享测试工具模块
2. FauxProvider — 无网络、确定性的 mock LLM provider
3. TestHarness — Builder 模式全连线测试 session
4. VT100Backend — VT100 终端模拟 backend（feature-gated）
5. SSE mock 构建器 — wiremock HTTP mock for LLM API
6. 内存管理器模式 — SessionManager/SettingsManager/AuthStorage 的 in_memory() 构造
7. CI 配置 — nextest 配置、CI 分层、快照审批流程
8. 回归测试工作流模板

### 测试金字塔

```
E2E (PTY 进程级) → 跨模块 (TestHarness 全连线) → 集成 (mock 依赖) → 单元 (纯函数)
```

### 测试技术栈

| Crate | 用途 |
|-------|------|
| `insta` | 快照测试（TUI 渲染、API 响应） |
| `cargo-nextest` | 测试运行器 |
| `wiremock` | HTTP mock（LLM API SSE） |
| `assert_cmd` | CLI 进程断言 |
| `vt100` | 终端模拟器（feature-gated） |
| `tempfile` | 临时目录隔离 |

### Feature Gate

```toml
[features]
dev-vt100 = ["vt100"]  # VT100Backend 完整渲染管线测试
dev-e2e = []            # PTY E2E 测试
```

## Capabilities

- `test-infra`: FauxProvider + TestHarness + VT100Backend + SSE mocks + CI 配置

## Impact

- 新增 `insta`, `wiremock`, `assert_cmd`, `vt100`, `tempfile`, `ctor`, `assert_matches`, `serial_test`, `test-case` 开发依赖
- `.config/nextest.toml` CI 配置
- 被所有包含测试的 change 引用为 dev-dependency
