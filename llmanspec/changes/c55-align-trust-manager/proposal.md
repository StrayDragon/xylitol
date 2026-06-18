---
depends_on: []
---

# c55-align-trust-manager: 对齐 pi TrustManager

## Why
当前 xylitol 的 trust 系统仅做基本的项目信任检查，缺少 pi 的完整信任管理：信任选项（Trust/Don't Trust/Trust parent/Session only）、信任存储持久化、信任继承路径检测、信任资源检测（.pi config 资源如 settings.json, extensions, skills, SYSTEM.md）。扩展系统和 ResourceLoader 依赖 TrustManager 进行项目资源门控。

## What Changes
- **新增** `src/infra/trust/mod.rs`：`TrustManager` 完全对齐 pi trust-manager.ts
  - `is_project_trusted(cwd)` 从信任存储查找
  - `get_trust_options(cwd, include_session_only)` 返回信任决策选项
  - `set_trust(path, decision)` 持久化信任决策
  - `has_trust_requiring_resources(cwd)` 检测需要信任的项目配置
- **新增** `src/infra/trust/types.rs`：`TrustStore` 类型定义
- **删除** 旧的 `src/agent/trust.rs` 和 `src/agent/project_trust.rs`
- BDD 测试新增信任场景

## Capabilities
- session-persistence

## Impact
- 破坏性变更：旧的 `ProjectTrustStore` 替换为 `TrustManager`
- 信任决策结果现在支持 session-only 模式
- 信任调用点从 agent/ 层迁移到 infra/ 层
