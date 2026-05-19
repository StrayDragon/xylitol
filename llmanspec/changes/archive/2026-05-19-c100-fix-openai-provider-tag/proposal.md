---
depends_on:
  - c10-add-config
---

# c100-fix-openai-provider-tag

## Why

当前 `ProviderKind::OpenAI` 在 `serde(rename_all = "snake_case")` 下会被序列化为 `open_a_i`。
这会导致配置文件里直觉上应写的 `openai` 无法通过 schema/反序列化校验，影响可用性。

## What changes

- 将配置字段 `model.models.*.provider` 的 OpenAI provider 标识统一为 `openai`（而不是 `open_a_i`）。
- 更新 JSON Schema 与相关测试用例。

## Capabilities

- `runtime-config`

## Impact

- 配置里若使用了旧值 `open_a_i`，需要升级为 `openai`。
