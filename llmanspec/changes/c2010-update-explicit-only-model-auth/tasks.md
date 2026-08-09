# Tasks：c2010-update-explicit-only-model-auth

## 1. 合约

- [x] 1.1 `runtime-model-registry`：收紧 m12；新增 m17
- [x] 1.2 `cli-entry`：ce2 与「仅显式配置」对齐
- [x] 1.3 更新 `runtime-model-registry.feature` m12 场景

## 2. 实现

- [x] 2.1 bootstrap：YAML 条目始终 register；`resolve_entry_api_key` 去掉 kind-env 回落
- [x] 2.2 删除「registry 空时用 OPENAI_/ANTHROPIC_ 注入 default id」分支
- [x] 2.3 调整 `ModelEntry` / 引导文案 / `AppConfig::resolve_model`
- [x] 2.4 单测：无 key 仍注册；无 YAML+env 不造模；有 YAML 空 map → ConfigLoadedZeroModels

## 3. 验证

- [x] 3.1 相关单测绿（bootstrap / provider_guidance / thinking_levels）
- [x] 3.2 手工：`tui --list-models` 在无 `OPENAI_API_KEY` 时仍列出本地 openai-compat 别名
