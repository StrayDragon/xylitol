# Verify：c2010-update-explicit-only-model-auth

**日期：** 2026-08-09
**HEAD：** `a3d4b8cb`（+ 本报告后可选 polish）
**Validate：** `--strict --no-check` → valid

## 合约轴

| Req | 结果 | 证据 |
|---|---|---|
| m12 | **PASS** | 已删 env→default_model 注入；`env_only_does_not_invent_models` |
| m17 | **PASS** | YAML 别名始终 register；`resolve_entry_api_key` 无 kind-env；`yaml_entry_registers_without_kind_env_key`；手工 `--list-models` 无 `OPENAI_API_KEY` 仍列本地别名 |
| ce2 | **PASS** | 零显式别名 → `ConfigLoadedZeroModels` / `NoModelsAvailable` |

**CRITICAL：** 无

## 标准轴

| 级 | 项 |
|---|---|
| WARNING | `AppConfig::resolve_model` 对 `api_key: ""` 曾与 bootstrap「空=省略」略不一致（建议统一为空串） |
| SUGGESTION | m12 `.feature` 无 BDD step 绑定（单测兜底即可）；`NoModelsAvailable` 文档字符串可再收紧 |

## 手工

- `--list-models` + 用户 config（anchor 版）→ **23** 模型、**0** 缺 key 警告（本地用 `*llamacpp_api_key`）

## Verdict

**READY_TO_ARCHIVE**
