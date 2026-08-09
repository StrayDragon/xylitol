---
depends_on: []
---

# 仅认用户显式模型配置（禁止隐式 env 回落与 env 造模）

## Why

当前 bootstrap 对 YAML `models.*.api_key` **省略**时回落 `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`；缺 key 则**跳过注册**。结果：本地 llama.cpp 等 openai-compat 条目因未设全局 `OPENAI_API_KEY` 而不出现在 `--list-models`，被迫用占位 env 才能列出——与「用户按条目自注 secret（Zen / DeepSeek / 本地 server 各不相同）」冲突，且隐式过强。

另：无 YAML 模型层时，仍可用 env 注入默认 `gpt-4o` / Anthropic 默认 id 进 registry（虽 m12 禁止自动**选中**）。产品要 **仅认用户显式配置**：有条目才进表；无条目则空表硬失败，不靠 env 造模。

## What Changes

- YAML 中声明的每个 model 别名 MUST 注册进 ModelRegistry（`--list-models` / 可选中），**不因**缺 API key 而跳过。
- `api_key` 省略或空 → 运行时 key 为空；**MUST NOT** 回落 kind 级 `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`（及 `OPENAI_KEY` / `ANTHROPIC_KEY`）。
- 无 YAML 模型条目（无配置层或 `models.models` 空）时 MUST NOT 用 env 注册默认模型；经 bootstrap 的表面 MUST 硬失败（对齐「仅显式」）。
- 真发请求时缺 key → 既有鉴权引导（m7/ux4）；list/选模不要求 key 已解析。
- 更新相关单测 / BDD（m12 场景收紧）。

## Capabilities

- `runtime-model-registry` — 显式注册 / 禁 env 造模
- `cli-entry` — bootstrap 零显式模型硬失败语义（ce2 对齐）

## Impact

- 本地 openai-compat 无全局 `OPENAI_API_KEY` 也可列出与切换。
- 依赖「只设 OPENAI_API_KEY、不写 YAML」开箱的旧路径失效（预 1.0 可接受；文档提示写 YAML + 每模 `api_key`/`{{ secret.* }}`）。

## Ethics

- risk_level: low
- prohibited_actions: 恢复隐式 kind-env 回落；无 YAML 时静默用 env 注入可调用默认模型
- required_evidence: bootstrap 单测（有条目无 key 仍注册；无条目+env 不造模）；`--list-models` 或等价可观察
- refusal_contract: 不承诺无 key 时请求成功
- escalation_policy: 若需保留「纯 env 发现」调试开关须单独确认
