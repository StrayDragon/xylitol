---
depends_on: [c03-update-cli-mode-dispatch]
---

# c04-add-agent-profile-foundation

## Why

当前系统在启动时选择一个 model，整个 session 都用同一个。无法按功能/角色分配不同 model（如规划用强模型、执行用快模型）。同时 `src/agent/provider/mod.rs` 有 620 行，需拆分为独立文件。

## What Changes

1. **拆分 provider 模块**: `provider/mod.rs` → `mod.rs`(re-export) + `fake.rs`(全部逻辑)
2. **新增 `AgentProfile` / `AgentsConfig` 配置类型**: flat struct 支持 YAML anchor/alias
3. **新增 `ResolvedProfile` 运行时类型**: 完整描述一个 agent 的 model/prompt/tools/iterations
4. **配置解析方法**: `AppConfig::resolve_profile()` / `resolve_model()` / `resolve_default_profile()`
5. **重构 `AgentLoop::new()`**: 接受 `ResolvedProfile` 替代 `ModelConfig`，支持 system_prompt、max_iterations、tool scoping
6. **更新调用链**: CLI → print → AgentLoop 全部使用 profile

## Capabilities

- agent-profile: 多 agent 配置（不同 model/prompt/tools/iterations）的基础运行时支持
- provider-split: FakeProvider 拆分为独立源文件

## Impact

- 新文件: `src/agent/provider/fake.rs`, `src/agent/profile.rs`
- 改动: `src/agent/loop.rs`, `src/interface/cli/mod.rs`, `src/interface/print.rs`, `src/agent/tools/mod.rs`, `src/infra/config/types.rs`
- 向后兼容: 无 `agents` section 时从 `model.default_model` + `execution.*` 合成
- 无新依赖
