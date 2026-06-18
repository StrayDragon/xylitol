---
depends_on:
  - c30-align-model-registry
  - c35-align-settings-manager
  - c40-align-event-bus
  - c45-align-skills-system
  - c55-align-trust-manager
---

# c60-align-resource-loader: 对齐 pi ResourceLoader

## Why
当前 xylitol 没有统一的资源加载器。pi 的 ResourceLoader 是中央资源发现和缓存层。

## What Changes
- **新增** `src/infra/resource/loader.rs`：`DefaultResourceLoader` 对齐 pi
- **新增** `src/agent/prompt/system_prompt.rs`：`build_system_prompt(opts)` 对齐 pi
- **重写** `src/infra/resource.rs` 为新的 resource 模块结构

## Capabilities
- skill-extension

## Impact
- 破坏性变更：旧的 `infra/resource.rs` 被重写为模块化结构
