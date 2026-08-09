# Tasks：c2010-update-explicit-only-model-auth

## 1. 合约

- [ ] 1.1 `runtime-model-registry`：收紧 m12；新增「显式条目必注册 / 禁 kind-env 回落 / 禁 env 造模」
- [ ] 1.2 `cli-entry`：ce2 与「仅显式配置」对齐（有 YAML 零条目 vs 无模型层）
- [ ] 1.3 更新 `runtime-model-registry.feature` m12 场景（无 YAML+仅 env → 不注册默认模型）

## 2. 实现

- [ ] 2.1 bootstrap：YAML 条目始终 register；`resolve_entry_api_key` 去掉 kind-env 回落
- [ ] 2.2 删除「registry 空时用 OPENAI_/ANTHROPIC_ 注入 default id」分支
- [ ] 2.3 调整 `ModelEntry` / 文档注释；引导文案若仍提「设 OPENAI_API_KEY 即可无」则改为「YAML 显式 + 每模 api_key/secret」
- [ ] 2.4 单测：无 key 仍注册；无 YAML+env 不造模；有 YAML 空 map → ConfigLoadedZeroModels

## 3. 验证

- [ ] 3.1 相关单测 / BDD 绿
- [ ] 3.2 手工：`tui --list-models` 在无 `OPENAI_API_KEY` 时仍列出本地 openai-compat 别名
