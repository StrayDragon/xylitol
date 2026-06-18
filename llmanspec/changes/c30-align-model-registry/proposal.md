---
depends_on: []
---

# c30-align-model-registry: 对齐 pi ModelRegistry/ModelResolver

## Why
当前 xylitol 的 ModelRegistry 仅为玩具级别实现（按 index 选取模型），完全无法支持 pi 的核心模型管理能力。

## What Changes
- **完全重写** `src/agent/registry.rs` 为 pi-aligned `ModelRegistry`
- **删除** 旧的 index-based 选取逻辑
- **新增** `src/infra/config/config_value.rs`：三级配置值解析
- **新增** `src/agent/auth_storage.rs`：OAuth 凭据持久化
- **新增** `src/agent/resolver.rs`：对齐 pi model-resolver.ts
- 更新 `src/agent/session.rs` 直接使用新 `ModelRegistry`

## Capabilities
- model-registry

## Impact
- 旧代码全部删除，不做兼容
- `ModelConfig` 类型完全替换为新定义
- `ModelMeta` 字段扩展到 pi 等价结构
