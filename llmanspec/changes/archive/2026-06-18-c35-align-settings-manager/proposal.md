---
depends_on:
  - c30-align-model-registry
---

# c35-align-settings-manager: 对齐 pi SettingsManager

## Why
当前 xylitol 的 config 模块缺少 pi 的 Settings struct 完整字段、三层深度合并、文件锁、热重载。

## What Changes
- **新增** `src/infra/settings/` 模块（manager.rs, storage.rs, types.rs）
- **删除** `src/infra/config/types.rs` 中 settings 相关旧字段定义
- **删除** 旧的 YAML config 中 settings 解析路径，全部迁移至新模块

## Capabilities
- runtime-config

## Impact
- 旧 config types 中的 settings 部分直接删除，不保留
- `AppConfig.settings` 废弃并移除
- `ConfigPaths` 中 settings 相关路径迁移
